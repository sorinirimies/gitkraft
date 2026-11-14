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
    if tab == Tab::Wiki && is_github(hub, repo) {
        let (w, r) = (hub.wiki_git().clone(), repo.clone());
        let pages =
            crate::wiki_git::blocking(move || w.sync(&r).map(|d| crate::wiki_git::pages(&d)))
                .await??;
        return Ok(pages
            .into_iter()
            .map(|p| Item {
                board: String::new(),
                column: String::new(),
                key: p.slug,
                title: p.title,
                meta: String::new(),
            })
            .collect());
    }
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
    if tab == Tab::Wiki && is_github(hub, repo) {
        let (w, r, slug) = (hub.wiki_git().clone(), repo.clone(), item.key.clone());
        let text = crate::wiki_git::blocking(move || {
            let dir = w.sync(&r)?;
            crate::wiki_git::read(&dir, &slug)
        })
        .await??;
        return Ok(Page {
            key: item.key.clone(),
            title: item.key.replace('-', " "),
            text,
            comments: Vec::new(),
        });
    }
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

/// Marks a comment of a discussion as its answer.
///
/// # Errors
/// No account, not GitHub, or the comment is not a top-level one of that discussion.
pub async fn mark_answer<C: HttpClient + Clone>(
    hub: &Hub<C>,
    repo: &RepoId,
    number: u64,
    comment_id: u64,
) -> Result<String, String> {
    hub.backend(&repo.host)
        .ok_or_else(|| format!("no account for {}", repo.host))?
        .mark_discussion_answer(repo, number, comment_id)
        .await
        .map(|()| format!("comment {comment_id} marked as the answer"))
        .map_err(|e| e.to_string())
}

/// Reads which comment to mark as the answer: `N`, the 1-based place of the comment on the page.
///
/// # Errors
/// Not a number, or a number outside the page.
pub fn parse_answer(typed: &str, page: &Page) -> Result<u64, String> {
    let n = typed
        .trim()
        .trim_start_matches(['[', '#'])
        .trim_end_matches(']');
    let n: usize = n
        .parse()
        .map_err(|_| format!("'{typed}' is not a comment number"))?;
    n.checked_sub(1)
        .and_then(|i| page.comments.get(i))
        .map(|c| c.id)
        .ok_or_else(|| {
            format!(
                "there is no comment {n}: the page has {}",
                page.comments.len()
            )
        })
}

fn is_github<C: HttpClient + Clone>(hub: &Hub<C>, repo: &RepoId) -> bool {
    hub.kind_of(&repo.host) == Some(gitkraft_platforms::PlatformKind::GitHub)
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
    if is_github(hub, repo) {
        // a GitHub wiki is a git repository: the page is a file, the save a commit and a push
        let name = slug.map_or_else(|| title.trim().replace(' ', "-"), str::to_string);
        let (w, r, content, message) = (
            hub.wiki_git().clone(),
            repo.clone(),
            content.to_string(),
            message.clone(),
        );
        return crate::wiki_git::blocking(move || w.save(&r, &name, &content, &message)).await?;
    }
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

/// Which way a card goes inside its column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shift {
    /// One place up.
    Up,
    /// One place down.
    Down,
    /// To the top.
    Top,
}

impl Shift {
    /// `up`, `down` or `top`.
    ///
    /// # Errors
    /// Anything else.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            "top" => Ok(Self::Top),
            other => Err(format!("'{other}' is not up, down or top")),
        }
    }
}

/// Adds a card titled `title` to a board, in the column `column` names (none: the board's default).
/// `board` picks one board by name or prefix; without it the first board is used.
///
/// # Errors
/// No account, an empty title, no board or column, a platform without boards, or it refuses.
pub async fn new_card<C: HttpClient + Clone>(
    hub: &Hub<C>,
    repo: &RepoId,
    board: Option<&str>,
    title: &str,
    column: Option<&str>,
) -> Result<String, String> {
    if title.trim().is_empty() {
        return Err("a card needs a title".into());
    }
    let b = hub
        .backend(&repo.host)
        .ok_or_else(|| format!("no account for {}", repo.host))?;
    let boards = b.boards(repo).await.map_err(|e| e.to_string())?;
    let board = boards
        .iter()
        .find(|bd| {
            board.is_none_or(|w| bd.name.to_lowercase().starts_with(&w.trim().to_lowercase()))
        })
        .ok_or_else(|| format!("{} has no such board", repo.full_name()))?;
    let to = column
        .filter(|c| !c.trim().is_empty())
        .map(|c| pick_column(board, c))
        .transpose()?;
    b.create_card(repo, board, title.trim(), to)
        .await
        .map(|()| match to {
            Some(c) => format!("card added to {} on {}", c.name, board.name),
            None => format!("card added to {}", board.name),
        })
        .map_err(|e| e.to_string())
}

/// Moves the card of issue or PR `number` inside its column. The board is read fresh.
///
/// # Errors
/// No account, no such card, already at that end, a platform that cannot order, or it refuses.
pub async fn reorder_card<C: HttpClient + Clone>(
    hub: &Hub<C>,
    repo: &RepoId,
    board: Option<&str>,
    number: u64,
    shift: Shift,
) -> Result<String, String> {
    let b = hub
        .backend(&repo.host)
        .ok_or_else(|| format!("no account for {}", repo.host))?;
    let boards = b.boards(repo).await.map_err(|e| e.to_string())?;
    let (bd, col, i) = boards
        .iter()
        .filter(|bd| {
            board.is_none_or(|w| bd.name.to_lowercase().starts_with(&w.trim().to_lowercase()))
        })
        .find_map(|bd| {
            bd.columns.iter().find_map(|c| {
                c.cards
                    .iter()
                    .position(|k| k.number == Some(number))
                    .map(|i| (bd, c, i))
            })
        })
        .ok_or_else(|| format!("#{number} is on no board of {}", repo.full_name()))?;
    let after = match shift {
        Shift::Top if i == 0 => return Err(format!("#{number} is already first")),
        Shift::Up if i == 0 => return Err(format!("#{number} is already first")),
        Shift::Down if i + 1 == col.cards.len() => {
            return Err(format!("#{number} is already last"));
        }
        Shift::Top => None,
        Shift::Up => i.checked_sub(2).and_then(|j| col.cards.get(j)),
        Shift::Down => col.cards.get(i + 1),
    };
    b.reorder_card(bd, &col.cards[i], after)
        .await
        .map(|()| {
            let way = match shift {
                Shift::Up => "up",
                Shift::Down => "down",
                Shift::Top => "to the top",
            };
            format!("#{number} moved {way} in {}", col.name)
        })
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

    #[test]
    fn an_answer_is_named_by_the_place_of_the_comment() {
        let p = thread();
        assert_eq!(parse_answer("2", &p), Ok(91));
        assert_eq!(parse_answer(" [3] ", &p), Ok(92));
        assert!(parse_answer("9", &p).is_err_and(|e| e.contains("page has 3")));
        assert!(parse_answer("two", &p).is_err_and(|e| e.contains("not a comment number")));
        assert!(parse_answer("0", &p).is_err());
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

    fn last_body(fake: &FakeClient) -> String {
        String::from_utf8_lossy(
            fake.requests()
                .last()
                .and_then(|r| r.body.clone())
                .unwrap_or_default()
                .as_slice(),
        )
        .into_owned()
    }

    const THREE_CARDS: &str = r#"{"data":{"repository":{"projectsV2":{"nodes":[{"id":"P","title":"Dev",
        "field":{"id":"F","options":[{"id":"O1","name":"Todo"},{"id":"O2","name":"Doing"}]},
        "items":{"nodes":[
        {"id":"I1","fieldValueByName":{"name":"Todo"},"content":{"number":1,"title":"a","url":"u","assignees":{"nodes":[]}}},
        {"id":"I2","fieldValueByName":{"name":"Todo"},"content":{"number":2,"title":"b","url":"u","assignees":{"nodes":[]}}},
        {"id":"I3","fieldValueByName":{"name":"Todo"},"content":{"number":3,"title":"c","url":"u","assignees":{"nodes":[]}}}]}}]}}}}"#;

    #[tokio::test]
    async fn a_card_is_put_after_the_right_neighbour_and_the_ends_say_so() {
        let repo = RepoId::new("github.com", "o", "r");
        let ok = r#"{"data":{"updateProjectV2ItemPosition":{"items":{"nodes":[]}}}}"#;
        for (number, shift, after) in [
            (3, Shift::Up, "\"a\":\"I1\""),
            (2, Shift::Up, "\"a\":null"),
            (1, Shift::Down, "\"a\":\"I2\""),
            (3, Shift::Top, "\"a\":null"),
        ] {
            let (hub, fake) = make_hub(PlatformKind::GitHub, "github.com");
            fake.push_json("https://api.github.com/graphql", THREE_CARDS);
            fake.push_json("https://api.github.com/graphql", ok);
            let done = reorder_card(&hub, &repo, None, number, shift).await;
            assert!(done.is_ok(), "{number} {shift:?}: {done:?}");
            let sent = last_body(&fake);
            assert!(sent.contains(after), "{number} {shift:?}: {sent}");
        }
        let (hub, fake) = make_hub(PlatformKind::GitHub, "github.com");
        fake.push_json("https://api.github.com/graphql", THREE_CARDS);
        let e = reorder_card(&hub, &repo, None, 1, Shift::Up)
            .await
            .unwrap_err();
        assert!(e.contains("already first"), "{e}");
        let (hub, fake) = make_hub(PlatformKind::GitHub, "github.com");
        fake.push_json("https://api.github.com/graphql", THREE_CARDS);
        let e = reorder_card(&hub, &repo, None, 3, Shift::Down)
            .await
            .unwrap_err();
        assert!(e.contains("already last"), "{e}");
        assert_eq!(Shift::parse("UP"), Ok(Shift::Up));
        assert!(Shift::parse("sideways").is_err());
    }

    #[tokio::test]
    async fn a_new_card_lands_in_the_named_column_and_needs_a_title() {
        let repo = RepoId::new("github.com", "o", "r");
        let (hub, fake) = make_hub(PlatformKind::GitHub, "github.com");
        fake.push_json("https://api.github.com/graphql", THREE_CARDS);
        fake.push_json(
            "https://api.github.com/graphql",
            r#"{"data":{"addProjectV2DraftIssue":{"projectItem":{"id":"I9"}}}}"#,
        );
        fake.push_json(
            "https://api.github.com/graphql",
            r#"{"data":{"updateProjectV2ItemFieldValue":{"projectV2Item":{"id":"I9"}}}}"#,
        );
        let done = new_card(&hub, &repo, None, " Ship it ", Some("doing")).await;
        assert_eq!(done, Ok("card added to Doing on Dev".to_string()));
        let sent = last_body(&fake);
        assert!(
            sent.contains("\"i\":\"I9\"") && sent.contains("\"o\":\"O2\""),
            "{sent}"
        );
        let e = new_card(&hub, &repo, None, "  ", None).await.unwrap_err();
        assert!(e.contains("title"), "{e}");
    }

    /// A git whose clone is a folder holding one page, and which remembers what was run.
    #[derive(Debug, Default)]
    struct WikiFakeGit(std::sync::Mutex<Vec<String>>);

    impl crate::wiki_git::GitRunner for WikiFakeGit {
        fn run(&self, _d: Option<&std::path::Path>, args: &[&str]) -> Result<String, String> {
            self.0.lock().expect("lock").push(args.join(" "));
            if args[0] == "clone" {
                let dest = std::path::Path::new(args.last().copied().unwrap_or_default());
                std::fs::create_dir_all(dest.join(".git")).map_err(|e| e.to_string())?;
                std::fs::write(dest.join("Home.md"), "# Welcome\n").map_err(|e| e.to_string())?;
            }
            Ok(String::new())
        }
    }

    #[tokio::test]
    async fn a_github_wiki_is_read_and_saved_through_its_git_repository() {
        let tmp = tempfile::tempdir().expect("tmp");
        let git = std::sync::Arc::new(WikiFakeGit::default());
        let (hub, fake) = make_hub(PlatformKind::GitHub, "github.com");
        let hub = hub.with_wiki_git(crate::wiki_git::WikiGit {
            git: git.clone(),
            cache: tmp.path().to_path_buf(),
        });
        let repo = RepoId::new("github.com", "o", "r");
        let items = list(&hub, &repo, Tab::Wiki).await.expect("list");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key, "Home");
        let p = page(&hub, &repo, Tab::Wiki, &items[0]).await.expect("page");
        assert_eq!(p.text, "# Welcome\n");
        let done = save_wiki(&hub, &repo, None, "Getting Started", "hi\n", "").await;
        assert_eq!(done, Ok("saved wiki page Getting-Started".to_string()));
        let runs = git.0.lock().expect("lock").clone();
        assert!(
            runs.iter()
                .any(|c| c.starts_with("add -- Getting-Started.md")),
            "{runs:?}"
        );
        assert!(
            runs.last().is_some_and(|c| c.starts_with("push")),
            "{runs:?}"
        );
        assert!(fake.requests().is_empty(), "no API call: the wiki is git");
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
