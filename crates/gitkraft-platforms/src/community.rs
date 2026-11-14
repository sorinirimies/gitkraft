//! Wikis and discussions: the parts of a forge that are neither code, PRs nor CI.
//!
//! GitHub has discussions (GraphQL only) but no wiki API: its wiki is a git
//! repository, `<repo>.wiki.git`. GitLab and Gitea/Forgejo/Codeberg have wiki
//! REST APIs but no discussions.

use base64::Engine;
use serde::Deserialize;
use serde_json::json;

use crate::error::{PlatformError, Result};
use crate::github_like::GitHubLike;
use crate::gitlab::{GitLab, project};
use crate::http::HttpClient;
use crate::model::{
    Board, BoardCard, BoardColumn, Comment, Discussion, DiscussionDetail, User, WikiContent,
    WikiPage,
};
use crate::platform::{PlatformKind, RepoId, url_encode};

const DISCUSSION_FIELDS: &str = "id number title url body updatedAt isAnswered \
     category{name} author{login avatarUrl} commentCount:comments{totalCount}";

fn discussion_of(n: &serde_json::Value) -> Discussion {
    Discussion {
        number: n["number"].as_u64().unwrap_or(0),
        title: n["title"].as_str().unwrap_or_default().into(),
        author: author_of(&n["author"]),
        category: n["category"]["name"].as_str().unwrap_or_default().into(),
        comments: n["commentCount"]["totalCount"].as_u64().unwrap_or(0) as u32,
        answered: n["isAnswered"].as_bool().unwrap_or(false),
        body: n["body"].as_str().unwrap_or_default().into(),
        url: n["url"].as_str().unwrap_or_default().into(),
        updated_at: n["updatedAt"].as_str().and_then(|t| t.parse().ok()),
    }
}

/// Deleted accounts come back as `null`; they read as `ghost`, like on the web.
fn author_of(a: &serde_json::Value) -> User {
    User {
        login: a["login"].as_str().unwrap_or("ghost").into(),
        avatar_url: a["avatarUrl"].as_str().map(str::to_string),
    }
}

impl<C: HttpClient> GitHubLike<C> {
    fn require_github(&self, what: &'static str) -> Result<()> {
        if self.api.kind == PlatformKind::GitHub {
            Ok(())
        } else {
            Err(PlatformError::Unsupported(what))
        }
    }

    /// Latest discussions, most recently updated first. GitHub only.
    pub(crate) async fn discussions(&self, repo: &RepoId, first: u32) -> Result<Vec<Discussion>> {
        self.require_github("discussions exist on GitHub only")?;
        let query = "query($o:String!,$n:String!,$first:Int!){repository(owner:$o,name:$n){\
             hasDiscussionsEnabled \
             discussions(first:$first,orderBy:{field:UPDATED_AT,direction:DESC}){nodes{FIELDS}}}}"
            .replace("FIELDS", DISCUSSION_FIELDS);
        let v = self
            .graphql(
                &query,
                &json!({ "o": repo.owner, "n": repo.name, "first": first.clamp(1, 100) }),
            )
            .await?;
        let r = &v["data"]["repository"];
        if r.is_null() {
            return Err(PlatformError::NotFound(repo.full_name()));
        }
        if r["hasDiscussionsEnabled"] == json!(false) {
            return Err(PlatformError::NotFound(format!(
                "discussions are switched off for {}",
                repo.full_name()
            )));
        }
        Ok(r["discussions"]["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .map(discussion_of)
            .collect())
    }

    /// One discussion with its comments. GitHub only.
    pub(crate) async fn discussion(&self, repo: &RepoId, number: u64) -> Result<DiscussionDetail> {
        self.require_github("discussions exist on GitHub only")?;
        let query = "query($o:String!,$n:String!,$num:Int!){repository(owner:$o,name:$n){\
             discussion(number:$num){FIELDS \
             comments(first:100){nodes{databaseId id body createdAt author{login avatarUrl} \
             replies(first:50){nodes{databaseId body createdAt author{login avatarUrl}}}}}}}}"
            .replace("FIELDS", DISCUSSION_FIELDS);
        let v = self
            .graphql(
                &query,
                &json!({ "o": repo.owner, "n": repo.name, "num": number }),
            )
            .await?;
        let d = &v["data"]["repository"]["discussion"];
        if d.is_null() {
            return Err(PlatformError::NotFound(format!(
                "discussion #{number} in {}",
                repo.full_name()
            )));
        }
        let comment = |c: &serde_json::Value, parent: Option<u64>| Comment {
            id: c["databaseId"].as_u64().unwrap_or(0),
            author: author_of(&c["author"]),
            body: c["body"].as_str().unwrap_or_default().into(),
            created_at: c["createdAt"].as_str().and_then(|t| t.parse().ok()),
            parent,
        };
        // Each top-level comment is followed by its replies, so a flat list reads as the thread.
        let mut comments = Vec::new();
        for c in d["comments"]["nodes"].as_array().into_iter().flatten() {
            let top = comment(c, None);
            let id = top.id;
            comments.push(top);
            comments.extend(
                c["replies"]["nodes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|r| comment(r, Some(id))),
            );
        }
        Ok(DiscussionDetail {
            discussion: discussion_of(d),
            comments,
        })
    }

    /// Adds a top-level comment to a discussion. GitHub only.
    pub(crate) async fn comment_on_discussion(
        &self,
        repo: &RepoId,
        number: u64,
        body: &str,
    ) -> Result<()> {
        self.require_github("discussions exist on GitHub only")?;
        let id = self
            .graphql(
                "query($o:String!,$n:String!,$num:Int!){repository(owner:$o,name:$n){discussion(number:$num){id}}}",
                &json!({ "o": repo.owner, "n": repo.name, "num": number }),
            )
            .await?["data"]["repository"]["discussion"]["id"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| PlatformError::NotFound(format!("discussion #{number}")))?;
        self.graphql(
            "mutation($id:ID!,$body:String!){addDiscussionComment(input:{discussionId:$id,body:$body}){comment{id}}}",
            &json!({ "id": id, "body": body }),
        )
        .await
        .map(|_| ())
    }

    /// Answers a comment of a discussion (`comment_id` is the number [`Comment::id`] carries; a
    /// reply to a reply goes to the same thread). GitHub only.
    pub(crate) async fn reply_to_discussion(
        &self,
        repo: &RepoId,
        number: u64,
        comment_id: u64,
        body: &str,
    ) -> Result<()> {
        self.require_github("discussions exist on GitHub only")?;
        let v = self
            .graphql(
                "query($o:String!,$n:String!,$num:Int!){repository(owner:$o,name:$n){discussion(number:$num){id \
                 comments(first:100){nodes{databaseId id replies(first:50){nodes{databaseId}}}}}}}",
                &json!({ "o": repo.owner, "n": repo.name, "num": number }),
            )
            .await?;
        let d = &v["data"]["repository"]["discussion"];
        let discussion = d["id"]
            .as_str()
            .ok_or_else(|| PlatformError::NotFound(format!("discussion #{number}")))?;
        // Replies are one level deep on GitHub: answering a reply answers its thread.
        let thread = d["comments"]["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| {
                c["databaseId"].as_u64() == Some(comment_id)
                    || c["replies"]["nodes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|r| r["databaseId"].as_u64() == Some(comment_id))
            })
            .and_then(|c| c["id"].as_str())
            .ok_or_else(|| {
                PlatformError::NotFound(format!("comment {comment_id} in discussion #{number}"))
            })?;
        self.graphql(
            "mutation($id:ID!,$to:ID!,$body:String!){addDiscussionComment(input:{discussionId:$id,replyToId:$to,body:$body}){comment{id}}}",
            &json!({ "id": discussion, "to": thread, "body": body }),
        )
        .await
        .map(|_| ())
    }

    /// Writes a wiki page: `slug` set edits that page, absent creates one called `title`.
    /// Gitea family only; GitHub's wiki is a git repository.
    pub(crate) async fn save_wiki_page(
        &self,
        repo: &RepoId,
        slug: Option<&str>,
        title: &str,
        content: &str,
        message: &str,
    ) -> Result<()> {
        self.require_gitea("GitHub wikis are git repositories: clone <repo>.wiki.git")?;
        let body = json!({
            "title": title,
            "content_base64": base64::engine::general_purpose::STANDARD.encode(content),
            "message": message,
        });
        let (method, path) = match slug {
            Some(s) => (
                crate::http::Method::Patch,
                format!("{}/wiki/page/{}", Self::repo_path(repo), url_encode(s)),
            ),
            None => (
                crate::http::Method::Post,
                format!("{}/wiki/new", Self::repo_path(repo)),
            ),
        };
        self.api.send(method, &path, Some(&body)).await.map(|_| ())
    }

    /// Moves a card to another column of its board. GitHub Projects (v2) only.
    pub(crate) async fn move_card(
        &self,
        board: &Board,
        card: &BoardCard,
        to: &BoardColumn,
    ) -> Result<()> {
        let (Some(project), Some(field), Some(item), Some(option)) = (
            board.project_id.as_deref(),
            board.field_id.as_deref(),
            card.item_id.as_deref(),
            to.option_id.as_deref(),
        ) else {
            return Err(PlatformError::Unsupported(
                "this card or column cannot be moved: load the board again",
            ));
        };
        self.graphql(
            "mutation($p:ID!,$i:ID!,$f:ID!,$o:String!){updateProjectV2ItemFieldValue(input:{projectId:$p,itemId:$i,fieldId:$f,value:{singleSelectOptionId:$o}}){projectV2Item{id}}}",
            &json!({ "p": project, "i": item, "f": field, "o": option }),
        )
        .await
        .map(|_| ())
    }

    /// The wiki's pages. Gitea family only; GitHub's wiki is a git repository.
    pub(crate) async fn wiki_pages(&self, repo: &RepoId) -> Result<Vec<WikiPage>> {
        self.require_gitea("GitHub wikis are git repositories: clone <repo>.wiki.git")?;
        let pages: Vec<GiteaPage> = self
            .api
            .get(&format!("{}/wiki/pages", Self::repo_path(repo)))
            .await?;
        Ok(pages
            .into_iter()
            .map(|p| WikiPage {
                title: p.title,
                slug: p.sub_url,
            })
            .collect())
    }

    /// One wiki page with its content.
    pub(crate) async fn wiki_page(&self, repo: &RepoId, slug: &str) -> Result<WikiContent> {
        self.require_gitea("GitHub wikis are git repositories: clone <repo>.wiki.git")?;
        let p: GiteaPage = self
            .api
            .get(&format!(
                "{}/wiki/page/{}",
                Self::repo_path(repo),
                url_encode(slug)
            ))
            .await?;
        let raw: String = p
            .content_base64
            .unwrap_or_default()
            .split_whitespace()
            .collect();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw)
            .map_err(|e| PlatformError::Parse(format!("wiki page is not base64: {e}")))?;
        Ok(WikiContent {
            page: WikiPage {
                title: p.title,
                slug: p.sub_url,
            },
            content: String::from_utf8_lossy(&bytes).into_owned(),
        })
    }

    /// The boards of a repository. GitHub: its Projects (v2), columns from the Status field.
    /// Gitea has projects in the web UI but no API for them.
    pub(crate) async fn boards(&self, repo: &RepoId) -> Result<Vec<Board>> {
        if self.api.kind != PlatformKind::GitHub {
            return Err(PlatformError::Unsupported(
                "Gitea and Forgejo have no project board API",
            ));
        }
        let card = "number title url assignees(first:5){nodes{login}}";
        let query = "query($o:String!,$n:String!){repository(owner:$o,name:$n){projectsV2(first:5){nodes{id title \
             field(name:\"Status\"){... on ProjectV2SingleSelectField{id options{id name}}} \
             items(first:100){nodes{id fieldValueByName(name:\"Status\"){... on ProjectV2ItemFieldSingleSelectValue{name}} \
             content{... on Issue{CARD} ... on PullRequest{CARD} ... on DraftIssue{title}}}}}}}}"
            .replace("CARD", card);
        let v = self
            .graphql(&query, &json!({ "o": repo.owner, "n": repo.name }))
            .await?;
        let r = &v["data"]["repository"];
        if r.is_null() {
            return Err(PlatformError::NotFound(repo.full_name()));
        }
        let mut boards = Vec::new();
        for p in r["projectsV2"]["nodes"].as_array().into_iter().flatten() {
            // the Status options give the column order, so empty columns show too
            let mut columns: Vec<BoardColumn> = p["field"]["options"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|o| Some((o["name"].as_str()?, o["id"].as_str())))
                .map(|(name, id)| BoardColumn {
                    name: name.into(),
                    cards: Vec::new(),
                    option_id: id.map(str::to_string),
                })
                .collect();
            for item in p["items"]["nodes"].as_array().into_iter().flatten() {
                let status = item["fieldValueByName"]["name"]
                    .as_str()
                    .unwrap_or("No status");
                let c = &item["content"];
                let card = BoardCard {
                    number: c["number"].as_u64(),
                    title: c["title"].as_str().unwrap_or_default().into(),
                    url: c["url"].as_str().unwrap_or_default().into(),
                    assignees: c["assignees"]["nodes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|a| a["login"].as_str().map(str::to_string))
                        .collect(),
                    item_id: item["id"].as_str().map(str::to_string),
                };
                if card.title.is_empty() {
                    continue; // a card whose content is not readable with this token
                }
                match columns.iter_mut().find(|c| c.name == status) {
                    Some(col) => col.cards.push(card),
                    None => columns.push(BoardColumn {
                        name: status.into(),
                        cards: vec![card],
                        option_id: None,
                    }),
                }
            }
            boards.push(Board {
                name: p["title"].as_str().unwrap_or_default().into(),
                columns,
                project_id: p["id"].as_str().map(str::to_string),
                field_id: p["field"]["id"].as_str().map(str::to_string),
            });
        }
        Ok(boards)
    }

    fn require_gitea(&self, what: &'static str) -> Result<()> {
        if self.api.kind == PlatformKind::GitHub {
            Err(PlatformError::Unsupported(what))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct GiteaPage {
    #[serde(default)]
    title: String,
    #[serde(default)]
    sub_url: String,
    #[serde(default)]
    content_base64: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct GitLabPage {
    #[serde(default)]
    title: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize, Default)]
struct GitLabBoard {
    #[serde(default)]
    name: String,
    #[serde(default)]
    lists: Vec<GitLabList>,
}

#[derive(Debug, Deserialize, Default)]
struct GitLabList {
    #[serde(default)]
    label: Option<GitLabListLabel>,
}

#[derive(Debug, Deserialize, Default)]
struct GitLabListLabel {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize, Default)]
struct GitLabCard {
    #[serde(default)]
    iid: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    web_url: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    assignees: Vec<GitLabAssignee>,
}

#[derive(Debug, Deserialize, Default)]
struct GitLabAssignee {
    #[serde(default)]
    username: String,
}

impl<C: HttpClient> GitLab<C> {
    /// The project's issue boards: one column per list label, plus `Open` for issues on no list.
    pub(crate) async fn boards(&self, repo: &RepoId) -> Result<Vec<Board>> {
        let boards: Vec<GitLabBoard> = self.api.get(&format!("{}/boards", project(repo))).await?;
        if boards.is_empty() {
            return Ok(Vec::new());
        }
        let issues: Vec<GitLabCard> = self
            .api
            .get(&format!(
                "{}/issues?state=opened&per_page=100&order_by=updated_at",
                project(repo)
            ))
            .await?;
        let card = |i: &GitLabCard| BoardCard {
            number: Some(i.iid),
            title: i.title.clone(),
            url: i.web_url.clone(),
            assignees: i.assignees.iter().map(|a| a.username.clone()).collect(),
            item_id: None,
        };
        Ok(boards
            .into_iter()
            .map(|b| {
                let names: Vec<String> = b
                    .lists
                    .iter()
                    .filter_map(|l| l.label.as_ref().map(|l| l.name.clone()))
                    .collect();
                let mut columns = vec![BoardColumn {
                    name: "Open".into(),
                    cards: issues
                        .iter()
                        .filter(|i| !i.labels.iter().any(|l| names.contains(l)))
                        .map(card)
                        .collect(),
                    option_id: None,
                }];
                columns.extend(names.iter().map(|n| {
                    BoardColumn {
                        name: n.clone(),
                        cards: issues
                            .iter()
                            .filter(|i| i.labels.contains(n))
                            .map(card)
                            .collect(),
                        option_id: None,
                    }
                }));
                Board {
                    name: b.name,
                    columns,
                    project_id: None,
                    field_id: None,
                }
            })
            .collect())
    }

    /// The project wiki's pages.
    pub(crate) async fn wiki_pages(&self, repo: &RepoId) -> Result<Vec<WikiPage>> {
        let pages: Vec<GitLabPage> = self.api.get(&format!("{}/wikis", project(repo))).await?;
        Ok(pages
            .into_iter()
            .map(|p| WikiPage {
                title: p.title,
                slug: p.slug,
            })
            .collect())
    }

    /// Writes a wiki page: `slug` set edits that page, absent creates one called `title`.
    pub(crate) async fn save_wiki_page(
        &self,
        repo: &RepoId,
        slug: Option<&str>,
        title: &str,
        content: &str,
    ) -> Result<()> {
        use crate::http::Method;
        let (method, path, body) = match slug {
            Some(s) => (
                Method::Put,
                format!("{}/wikis/{}", project(repo), url_encode(s)),
                json!({ "title": title, "content": content }),
            ),
            None => (
                Method::Post,
                format!("{}/wikis", project(repo)),
                json!({ "title": title, "content": content, "format": "markdown" }),
            ),
        };
        self.api.send(method, &path, Some(&body)).await.map(|_| ())
    }

    /// Moves an issue between the columns of a board: a column is a list label, `Open` is none.
    pub(crate) async fn move_card(
        &self,
        repo: &RepoId,
        board: &Board,
        card: &BoardCard,
        to: &BoardColumn,
    ) -> Result<()> {
        let number = card
            .number
            .ok_or(PlatformError::Unsupported("a draft card cannot be moved"))?;
        // Every list label of the board but the target comes off: the issue is in one column.
        let remove: Vec<&str> = board
            .columns
            .iter()
            .filter(|c| c.name != "Open" && c.name != to.name)
            .map(|c| c.name.as_str())
            .collect();
        let add = if to.name == "Open" {
            ""
        } else {
            to.name.as_str()
        };
        self.api
            .send(
                crate::http::Method::Put,
                &format!("{}/issues/{number}", project(repo)),
                Some(&json!({ "add_labels": add, "remove_labels": remove.join(",") })),
            )
            .await
            .map(|_| ())
    }

    /// One wiki page with its content.
    pub(crate) async fn wiki_page(&self, repo: &RepoId, slug: &str) -> Result<WikiContent> {
        let p: GitLabPage = self
            .api
            .get(&format!("{}/wikis/{}", project(repo), url_encode(slug)))
            .await?;
        Ok(WikiContent {
            page: WikiPage {
                title: p.title,
                slug: p.slug,
            },
            content: p.content,
        })
    }
}
