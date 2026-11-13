//! Discussions and wikis as the lists and pages every front-end shows.

use gitkraft_platforms::{HttpClient, RepoId};

use crate::Hub;

/// Which half of the view is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// GitHub discussions.
    #[default]
    Discussions,
    /// The repository wiki (GitLab, Gitea family).
    Wiki,
    /// Kanban boards: GitHub Projects, GitLab issue boards.
    Board,
}

impl Tab {
    /// `discussions` or `wiki`.
    pub fn label(self) -> &'static str {
        match self {
            Tab::Discussions => "discussions",
            Tab::Wiki => "wiki",
            Tab::Board => "boards",
        }
    }
}

/// One line of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Discussion number or wiki slug: what [`page`] takes.
    pub key: String,
    /// Title.
    pub title: String,
    /// Category, author, comment count; empty for wiki pages.
    pub meta: String,
}

/// An open discussion or wiki page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Page {
    /// Same as [`Item::key`].
    pub key: String,
    /// Title.
    pub title: String,
    /// Markdown.
    pub text: String,
    /// `(author, body)` of each comment; wiki pages have none.
    pub comments: Vec<(String, String)>,
}

/// The list for `repo`. The error says why a platform has no such thing.
pub async fn list<C: HttpClient + Clone>(
    hub: &Hub<C>,
    repo: &RepoId,
    tab: Tab,
) -> Result<Vec<Item>, String> {
    let b = hub
        .backend(&repo.host)
        .ok_or_else(|| format!("no account for {}", repo.host))?;
    match tab {
        Tab::Discussions => b
            .discussions(repo, 50)
            .await
            .map(|ds| {
                ds.into_iter()
                    .map(|d| Item {
                        key: d.number.to_string(),
                        title: format!("#{} {}", d.number, d.title),
                        meta: format!(
                            "{} · {} · {} comments{}",
                            d.category,
                            d.author.login,
                            d.comments,
                            if d.answered { " · answered" } else { "" }
                        ),
                    })
                    .collect()
            })
            .map_err(|e| e.to_string()),
        Tab::Board => b
            .boards(repo)
            .await
            .map(|boards| {
                let mut items = Vec::new();
                for board in boards {
                    for col in board.columns {
                        for card in col.cards {
                            let who = if card.assignees.is_empty() {
                                String::new()
                            } else {
                                format!(" · {}", card.assignees.join(", "))
                            };
                            items.push(Item {
                                key: card.number.map(|n| n.to_string()).unwrap_or_default(),
                                title: match card.number {
                                    Some(n) => format!("#{n} {}", card.title),
                                    None => card.title,
                                },
                                meta: format!("{} · {}{who}", board.name, col.name),
                            });
                        }
                    }
                }
                items
            })
            .map_err(|e| e.to_string()),
        Tab::Wiki => b
            .wiki_pages(repo)
            .await
            .map(|ps| {
                ps.into_iter()
                    .map(|p| Item {
                        key: p.slug,
                        title: p.title,
                        meta: String::new(),
                    })
                    .collect()
            })
            .map_err(|e| e.to_string()),
    }
}

/// The page behind an [`Item`].
pub async fn page<C: HttpClient + Clone>(
    hub: &Hub<C>,
    repo: &RepoId,
    tab: Tab,
    item: &Item,
) -> Result<Page, String> {
    let b = hub
        .backend(&repo.host)
        .ok_or_else(|| format!("no account for {}", repo.host))?;
    match tab {
        Tab::Discussions => {
            let n: u64 = item
                .key
                .parse()
                .map_err(|_| "not a discussion number".to_string())?;
            b.discussion(repo, n)
                .await
                .map(|d| Page {
                    key: item.key.clone(),
                    title: format!("#{} {}", d.discussion.number, d.discussion.title),
                    text: d.discussion.body,
                    comments: d
                        .comments
                        .into_iter()
                        .map(|c| (c.author.login, c.body))
                        .collect(),
                })
                .map_err(|e| e.to_string())
        }
        Tab::Board => match item.key.parse::<u64>() {
            // a draft card is only a title
            Err(_) => Ok(Page {
                key: String::new(),
                title: item.title.clone(),
                text: String::new(),
                comments: Vec::new(),
            }),
            Ok(n) => {
                let issue = b.issue(repo, n).await.map_err(|e| e.to_string())?;
                let comments = b
                    .comments(repo, gitkraft_platforms::ItemKind::Issue, n)
                    .await
                    .unwrap_or_default();
                Ok(Page {
                    // not a discussion: comments go through the item, not this view
                    key: String::new(),
                    title: item.title.clone(),
                    text: issue.body,
                    comments: comments
                        .into_iter()
                        .map(|c| (c.author.login, c.body))
                        .collect(),
                })
            }
        },
        Tab::Wiki => b
            .wiki_page(repo, &item.key)
            .await
            .map(|p| Page {
                key: item.key.clone(),
                title: p.page.title,
                text: p.content,
                comments: Vec::new(),
            })
            .map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::testing::FakeClient;
    use gitkraft_platforms::{Account, DynClient, PlatformKind, Secret};

    fn hub(kind: PlatformKind, host: &str) -> (Hub<DynClient>, FakeClient) {
        let fake = FakeClient::new();
        let mut a = Account::new(kind, host).with_token(Secret::new("t"));
        a.me = Some("me".into());
        (Hub::new(vec![a], &DynClient::new(fake.clone())), fake)
    }

    #[tokio::test]
    async fn discussions_become_titled_items_and_pages_with_comments() {
        let (hub, fake) = hub(PlatformKind::GitHub, "github.com");
        let repo = RepoId::new("github.com", "o", "r");
        fake.push_json(
            "https://api.github.com/graphql",
            r#"{"data":{"repository":{"hasDiscussionsEnabled":true,"discussions":{"nodes":[{"id":"D","number":5,"title":"Q","url":"u","body":"b","updatedAt":"2026-05-01T10:00:00Z","isAnswered":true,"category":{"name":"Q&A"},"author":{"login":"alice"},"commentCount":{"totalCount":2}}]}}}}"#,
        );
        fake.push_json(
            "https://api.github.com/graphql",
            r#"{"data":{"repository":{"discussion":{"id":"D","number":5,"title":"Q","url":"u","body":"the question","updatedAt":"2026-05-01T10:00:00Z","isAnswered":false,"category":{"name":"Q&A"},"author":{"login":"alice"},"commentCount":{"totalCount":1},"comments":{"nodes":[{"databaseId":9,"body":"answer","createdAt":"2026-05-01T11:00:00Z","author":{"login":"bob"}}]}}}}}"#,
        );
        let items = list(&hub, &repo, Tab::Discussions)
            .await
            .unwrap_or_default();
        assert_eq!(items[0].key, "5");
        assert_eq!(items[0].title, "#5 Q");
        assert_eq!(items[0].meta, "Q&A · alice · 2 comments · answered");

        let p = page(&hub, &repo, Tab::Discussions, &items[0])
            .await
            .unwrap_or_default();
        assert_eq!(p.text, "the question");
        assert_eq!(p.comments, vec![("bob".to_string(), "answer".to_string())]);
    }

    #[tokio::test]
    async fn a_wiki_page_reads_and_an_unsupported_tab_says_why() {
        let (hub, fake) = hub(PlatformKind::GitLab, "gitlab.com");
        let repo = RepoId::new("gitlab.com", "o", "r");
        fake.push_json(
            "https://gitlab.com/api/v4/projects/o%2Fr/wikis",
            r#"[{"title":"Home","slug":"home"}]"#,
        );
        let items = list(&hub, &repo, Tab::Wiki).await.unwrap_or_default();
        assert_eq!(items[0].key, "home");
        fake.push_json(
            "https://gitlab.com/api/v4/projects/o%2Fr/wikis/home",
            r##"{"title":"Home","slug":"home","content":"# Hi"}"##,
        );
        let p = page(&hub, &repo, Tab::Wiki, &items[0])
            .await
            .unwrap_or_default();
        assert_eq!((p.title.as_str(), p.text.as_str()), ("Home", "# Hi"));
        let err = list(&hub, &repo, Tab::Discussions)
            .await
            .err()
            .unwrap_or_default();
        assert!(err.contains("GitHub only"), "{err}");
        let err = list(&hub, &RepoId::new("nowhere.org", "o", "r"), Tab::Wiki)
            .await
            .err()
            .unwrap_or_default();
        assert!(err.contains("no account"), "{err}");
    }

    #[tokio::test]
    async fn board_cards_list_with_their_column_and_open_as_the_issue() {
        let (hub, fake) = hub(PlatformKind::GitLab, "gitlab.com");
        let repo = RepoId::new("gitlab.com", "o", "r");
        fake.push_json(
            "https://gitlab.com/api/v4/projects/o%2Fr/boards",
            r#"[{"id":1,"name":"Dev","lists":[{"label":{"name":"doing"}}]}]"#,
        );
        fake.push_json(
            "https://gitlab.com/api/v4/projects/o%2Fr/issues",
            r#"[{"iid":1,"title":"a","web_url":"u","labels":["doing"],"assignees":[{"username":"bob"}]}]"#,
        );
        let items = list(&hub, &repo, Tab::Board).await.unwrap_or_default();
        assert_eq!(items[0].title, "#1 a");
        assert_eq!(items[0].meta, "Dev · doing · bob");
        let none = Item {
            key: String::new(),
            title: "Draft".into(),
            meta: String::new(),
        };
        let p = page(&hub, &repo, Tab::Board, &none)
            .await
            .unwrap_or_default();
        assert_eq!(p.title, "Draft");
    }
}
