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
             comments(first:100){nodes{databaseId body createdAt author{login avatarUrl}}}}}}"
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
        let comments = d["comments"]["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| Comment {
                id: c["databaseId"].as_u64().unwrap_or(0),
                author: author_of(&c["author"]),
                body: c["body"].as_str().unwrap_or_default().into(),
                created_at: c["createdAt"].as_str().and_then(|t| t.parse().ok()),
            })
            .collect();
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
        let query = "query($o:String!,$n:String!){repository(owner:$o,name:$n){projectsV2(first:5){nodes{title \
             field(name:\"Status\"){... on ProjectV2SingleSelectField{options{name}}} \
             items(first:100){nodes{fieldValueByName(name:\"Status\"){... on ProjectV2ItemFieldSingleSelectValue{name}} \
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
                .filter_map(|o| o["name"].as_str())
                .map(|name| BoardColumn {
                    name: name.into(),
                    cards: Vec::new(),
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
                };
                if card.title.is_empty() {
                    continue; // a card whose content is not readable with this token
                }
                match columns.iter_mut().find(|c| c.name == status) {
                    Some(col) => col.cards.push(card),
                    None => columns.push(BoardColumn {
                        name: status.into(),
                        cards: vec![card],
                    }),
                }
            }
            boards.push(Board {
                name: p["title"].as_str().unwrap_or_default().into(),
                columns,
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
                }];
                columns.extend(names.iter().map(|n| {
                    BoardColumn {
                        name: n.clone(),
                        cards: issues
                            .iter()
                            .filter(|i| i.labels.contains(n))
                            .map(card)
                            .collect(),
                    }
                }));
                Board {
                    name: b.name,
                    columns,
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
