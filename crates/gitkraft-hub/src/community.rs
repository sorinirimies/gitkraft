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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Item {
    /// Discussion number or wiki slug: what [`page`] takes.
    pub key: String,
    /// Title.
    pub title: String,
    /// Category, author, comment count; empty for wiki pages.
    pub meta: String,
    /// The board a card is on (boards only).
    pub board: String,
    /// The column a card is in (boards only).
    pub column: String,
}

/// One comment of a [`Page`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PageComment {
    /// Platform id: what a reply names.
    pub id: u64,
    /// Who wrote it.
    pub author: String,
    /// Markdown.
    pub body: String,
    /// It answers another comment of the page.
    pub reply: bool,
}

impl PageComment {
    /// `author` and `body` with an arrow in front of a reply, as the text front-ends print it.
    #[must_use]
    pub fn heading(&self) -> String {
        format!("{}{}", if self.reply { "  ↳ " } else { "" }, self.author)
    }
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
    /// The comments, each thread's replies right after it; wiki pages have none.
    pub comments: Vec<PageComment>,
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
                        board: String::new(),
                        column: String::new(),
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
                                board: board.name.clone(),
                                column: col.name.clone(),
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
                        board: String::new(),
                        column: String::new(),
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
                        .map(|c| PageComment {
                            id: c.id,
                            author: c.author.login,
                            body: c.body,
                            reply: c.parent.is_some(),
                        })
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
                        .map(|c| PageComment {
                            id: c.id,
                            author: c.author.login,
                            body: c.body,
                            reply: false,
                        })
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

/// Answers a comment of a discussion.
///
/// # Errors
/// No account, not GitHub, or the comment does not exist.
pub async fn reply<C: HttpClient + Clone>(
    hub: &Hub<C>,
    repo: &RepoId,
    number: u64,
    comment_id: u64,
    body: &str,
) -> Result<String, String> {
    if body.trim().is_empty() {
        return Err("a reply needs some text".into());
    }
    hub.backend(&repo.host)
        .ok_or_else(|| format!("no account for {}", repo.host))?
        .reply_to_discussion(repo, number, comment_id, body.trim())
        .await
        .map(|()| format!("reply posted to comment {comment_id}"))
        .map_err(|e| e.to_string())
}

/// Creates (`slug` absent) or edits a wiki page.
///
/// # Errors
/// No account, a platform without a wiki API, an empty title, or the platform refuses.
pub async fn save_wiki<C: HttpClient + Clone>(
    hub: &Hub<C>,
    repo: &RepoId,
    slug: Option<&str>,
    title: &str,
    content: &str,
    message: &str,
) -> Result<String, String> {
    if title.trim().is_empty() {
        return Err("a wiki page needs a title".into());
    }
    let message = if message.trim().is_empty() {
        match slug {
            Some(_) => format!("Update {}", title.trim()),
            None => format!("Create {}", title.trim()),
        }
    } else {
        message.trim().to_string()
    };
    hub.backend(&repo.host)
        .ok_or_else(|| format!("no account for {}", repo.host))?
        .save_wiki_page(repo, slug, title.trim(), content, &message)
        .await
        .map(|()| format!("saved wiki page {}", title.trim()))
        .map_err(|e| e.to_string())
}

/// The column of `board` that `wanted` names: exact (any case), else the one that starts with it,
/// else the one that contains it.
///
/// # Errors
/// None or several match; the message lists the columns.
pub fn pick_column<'a>(
    board: &'a gitkraft_platforms::model::Board,
    wanted: &str,
) -> Result<&'a gitkraft_platforms::model::BoardColumn, String> {
    let w = wanted.trim().to_lowercase();
    let names = || {
        board
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if w.is_empty() {
        return Err(format!("name a column: {}", names()));
    }
    for test in [
        (|n: &str, w: &str| n == w) as fn(&str, &str) -> bool,
        |n, w| n.starts_with(w),
        |n, w| n.contains(w),
    ] {
        let hits: Vec<_> = board
            .columns
            .iter()
            .filter(|c| test(&c.name.to_lowercase(), &w))
            .collect();
        match hits.as_slice() {
            [one] => return Ok(one),
            [] => {}
            _ => return Err(format!("'{wanted}' matches several columns: {}", names())),
        }
    }
    Err(format!("no column '{wanted}': {}", names()))
}

/// Moves the card of issue or PR `number` to the column `column` names. `board` picks one board when
/// the card sits on several (a name or a prefix of it); the boards are read fresh, so the ids are not stale.
///
/// # Errors
/// No account, no such card or column, a platform without boards, or the platform refuses.
pub async fn move_card<C: HttpClient + Clone>(
    hub: &Hub<C>,
    repo: &RepoId,
    board: Option<&str>,
    number: u64,
    column: &str,
) -> Result<String, String> {
    let b = hub
        .backend(&repo.host)
        .ok_or_else(|| format!("no account for {}", repo.host))?;
    let boards = b.boards(repo).await.map_err(|e| e.to_string())?;
    let (board, card) = boards
        .iter()
        .filter(|bd| {
            board.is_none_or(|w| bd.name.to_lowercase().starts_with(&w.trim().to_lowercase()))
        })
        .find_map(|bd| {
            bd.columns
                .iter()
                .flat_map(|c| &c.cards)
                .find(|c| c.number == Some(number))
                .map(|c| (bd, c))
        })
        .ok_or_else(|| format!("#{number} is on no board of {}", repo.full_name()))?;
    let to = pick_column(board, column)?;
    b.move_card(repo, board, card, to)
        .await
        .map(|()| format!("#{number} moved to {} on {}", to.name, board.name))
        .map_err(|e| e.to_string())
}

/// Reads what a user typed to answer a comment of an open page: `N text`, where `N` is the 1-based
/// place of the comment in [`Page::comments`], as the text front-ends number them.
///
/// # Errors
/// No number, a number outside the page, or no text.
pub fn parse_reply(typed: &str, page: &Page) -> Result<(u64, String), String> {
    let typed = typed.trim();
    let (n, text) = typed
        .split_once(char::is_whitespace)
        .ok_or("type the comment's number, then your reply: `2 thanks!`")?;
    let n: usize = n
        .trim_start_matches(['[', '#'])
        .trim_end_matches(']')
        .parse()
        .map_err(|_| format!("'{n}' is not a comment number"))?;
    let comment = n
        .checked_sub(1)
        .and_then(|i| page.comments.get(i))
        .ok_or_else(|| {
            format!(
                "there is no comment {n}: the page has {}",
                page.comments.len()
            )
        })?;
    let text = text.trim();
    if text.is_empty() {
        return Err("a reply needs some text".into());
    }
    Ok((comment.id, text.to_string()))
}

/// A wiki page written to a file so any editor can change it; [`upload_draft`] sends it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiDraft {
    /// The file to edit.
    pub path: std::path::PathBuf,
    /// The repository.
    pub repo: RepoId,
    /// The page being edited; `None` for a new page.
    pub slug: Option<String>,
    /// The title (a new page's title is what the user typed).
    pub title: String,
    original: String,
}

impl WikiDraft {
    /// Writes `content` of page `slug` (absent: a new page called `title`) under the temp folder.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn write(
        repo: &RepoId,
        slug: Option<&str>,
        title: &str,
        content: &str,
    ) -> Result<Self, String> {
        let dir = std::env::temp_dir()
            .join("gitkraft-wiki")
            .join(crate::artifacts::safe_name(&repo.key().replace('/', "_")));
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = crate::artifacts::safe_name(slug.unwrap_or(title));
        let path = dir.join(format!("{name}.md"));
        std::fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self {
            path,
            repo: repo.clone(),
            slug: slug.map(str::to_string),
            title: title.to_string(),
            original: content.to_string(),
        })
    }

    /// The edited text, or `None` when it is as it was.
    ///
    /// # Errors
    /// The file cannot be read.
    pub fn changed(&self) -> Result<Option<String>, String> {
        let now = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("{}: {e}", self.path.display()))?;
        Ok((now != self.original).then_some(now))
    }
}

/// Sends an edited draft back; a file nobody changed is not sent.
///
/// # Errors
/// The file cannot be read, or [`save_wiki`] fails.
pub async fn upload_draft<C: HttpClient + Clone>(
    hub: &Hub<C>,
    draft: &WikiDraft,
) -> Result<String, String> {
    match draft.changed()? {
        None if draft.slug.is_some() => Ok("no changes to save".into()),
        None => Err("the new page is empty: write something first".into()),
        Some(content) => {
            save_wiki(
                hub,
                &draft.repo,
                draft.slug.as_deref(),
                &draft.title,
                &content,
                "",
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::testing::FakeClient;
    use gitkraft_platforms::{Account, DynClient, PlatformKind, Secret};

    fn make_hub(kind: PlatformKind, host: &str) -> (Hub<DynClient>, FakeClient) {
        let fake = FakeClient::new();
        let mut a = Account::new(kind, host).with_token(Secret::new("t"));
        a.me = Some("me".into());
        (Hub::new(vec![a], &DynClient::new(fake.clone())), fake)
    }

    #[tokio::test]
    async fn discussions_become_titled_items_and_pages_with_comments() {
        let (hub, fake) = make_hub(PlatformKind::GitHub, "github.com");
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
        assert_eq!(
            p.comments,
            vec![PageComment {
                id: 9,
                author: "bob".into(),
                body: "answer".into(),
                reply: false
            }]
        );
    }

    #[tokio::test]
    async fn a_wiki_page_reads_and_an_unsupported_tab_says_why() {
        let (hub, fake) = make_hub(PlatformKind::GitLab, "gitlab.com");
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
        let (hub, fake) = make_hub(PlatformKind::GitLab, "gitlab.com");
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
            ..Item::default()
        };
        let p = page(&hub, &repo, Tab::Board, &none)
            .await
            .unwrap_or_default();
        assert_eq!(p.title, "Draft");
    }

    fn column(name: &str) -> gitkraft_platforms::model::BoardColumn {
        gitkraft_platforms::model::BoardColumn {
            name: name.into(),
            ..Default::default()
        }
    }

    #[test]
    fn a_column_is_found_by_exact_prefix_or_part_of_its_name_and_ambiguity_is_refused() {
        let board = gitkraft_platforms::model::Board {
            columns: vec![
                column("Todo"),
                column("In progress"),
                column("In review"),
                column("Done"),
            ],
            ..Default::default()
        };
        assert_eq!(
            pick_column(&board, "done").map(|c| c.name.as_str()),
            Ok("Done")
        );
        assert_eq!(
            pick_column(&board, "to").map(|c| c.name.as_str()),
            Ok("Todo")
        );
        assert_eq!(
            pick_column(&board, "prog").map(|c| c.name.as_str()),
            Ok("In progress")
        );
        assert!(pick_column(&board, "in").unwrap_err().contains("several"));
        let none = pick_column(&board, "blocked").unwrap_err();
        assert!(
            none.contains("no column") && none.contains("Todo, In progress"),
            "{none}"
        );
        assert!(
            pick_column(&board, " ")
                .unwrap_err()
                .contains("name a column")
        );
    }

    fn thread() -> Page {
        let c = |id, reply| PageComment {
            id,
            reply,
            ..Default::default()
        };
        Page {
            comments: vec![c(90, false), c(91, true), c(92, false)],
            ..Default::default()
        }
    }

    #[test]
    fn a_typed_reply_names_the_comment_by_its_place_on_the_page() {
        let p = thread();
        assert_eq!(
            parse_reply("2 thanks a lot", &p),
            Ok((91, "thanks a lot".into()))
        );
        assert_eq!(parse_reply("[3] ok", &p), Ok((92, "ok".into())));
        assert_eq!(parse_reply("#1  hi ", &p), Ok((90, "hi".into())));
        for (typed, needle) in [
            ("thanks", "number"),
            ("x thanks", "not a comment number"),
            ("0 hi", "no comment 0"),
            ("4 hi", "page has 3"),
        ] {
            let e = parse_reply(typed, &p).unwrap_err();
            assert!(e.contains(needle), "{typed}: {e}");
        }
    }

    #[tokio::test]
    async fn a_card_moves_by_number_a_wiki_page_is_edited_through_a_file_and_replies_need_text() {
        let (hub, fake) = make_hub(PlatformKind::GitHub, "github.com");
        let repo = RepoId::new("github.com", "o", "r");
        let boards = r#"{"data":{"repository":{"projectsV2":{"nodes":[{"id":"P","title":"Dev",
            "field":{"id":"F","options":[{"id":"O1","name":"Todo"},{"id":"O2","name":"Doing"}]},
            "items":{"nodes":[{"id":"I1","fieldValueByName":{"name":"Todo"},"content":{"number":1,"title":"a","url":"u","assignees":{"nodes":[]}}}]}}]}}}}"#;
        fake.push_json("https://api.github.com/graphql", boards);
        fake.push_json(
            "https://api.github.com/graphql",
            r#"{"data":{"updateProjectV2ItemFieldValue":{"projectV2Item":{"id":"I1"}}}}"#,
        );
        let done = move_card(&hub, &repo, None, 1, "doing").await;
        assert_eq!(done, Ok("#1 moved to Doing on Dev".to_string()));
        let sent = String::from_utf8_lossy(
            fake.requests()
                .last()
                .and_then(|r| r.body.clone())
                .unwrap_or_default()
                .as_slice(),
        )
        .into_owned();
        assert!(
            sent.contains("\"o\":\"O2\"") && sent.contains("\"i\":\"I1\""),
            "{sent}"
        );

        let (hub, fake) = make_hub(PlatformKind::GitHub, "github.com");
        fake.push_json("https://api.github.com/graphql", boards);
        let e = move_card(&hub, &repo, None, 99, "doing").await.unwrap_err();
        assert!(e.contains("#99 is on no board"), "{e}");
        let (hub, fake) = make_hub(PlatformKind::GitHub, "github.com");
        fake.push_json("https://api.github.com/graphql", boards);
        let e = move_card(&hub, &repo, Some("other"), 1, "doing")
            .await
            .unwrap_err();
        assert!(e.contains("no board"), "{e}");

        assert!(
            reply(&hub, &repo, 5, 9, "  ")
                .await
                .unwrap_err()
                .contains("some text")
        );
        assert!(
            save_wiki(&hub, &repo, None, " ", "x", "")
                .await
                .unwrap_err()
                .contains("title")
        );
    }

    #[tokio::test]
    async fn an_edited_wiki_draft_is_uploaded_and_an_untouched_one_is_not() {
        let (hub, fake) = make_hub(PlatformKind::Gitea, "codeberg.org");
        let repo = RepoId::new("codeberg.org", "o", "r");
        let draft = WikiDraft::write(&repo, Some("Home"), "Home", "# Hi\n").expect("draft");
        assert!(draft.path.ends_with("Home.md"));
        assert_eq!(
            upload_draft(&hub, &draft).await,
            Ok("no changes to save".into())
        );
        assert!(fake.requests().is_empty());

        std::fs::write(&draft.path, "# Hi there\n").expect("edit");
        fake.push_json("https://codeberg.org/api/v1/repos/o/r/wiki/page/Home", "{}");
        assert_eq!(
            upload_draft(&hub, &draft).await,
            Ok("saved wiki page Home".into())
        );
        let sent = String::from_utf8_lossy(fake.requests()[0].body.as_deref().unwrap_or_default())
            .into_owned();
        assert!(sent.contains("Update Home"), "{sent}");

        let new = WikiDraft::write(&repo, None, "New page", "").expect("draft");
        assert!(
            upload_draft(&hub, &new)
                .await
                .unwrap_err()
                .contains("empty")
        );
    }
}
