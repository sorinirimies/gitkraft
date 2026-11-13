//! `gitkraft-tui commit-message`: a drafted message for what is staged, in the project's style.

use std::path::Path;

use anyhow::{Result, anyhow, bail};
use gitkraft_hub::ai::{AiConfig, Llm, Remote};
use gitkraft_hub::commit_ai::{self, RawStyle};
use gitkraft_platforms::credentials::SystemKeyring;
use gitkraft_platforms::{CachedClient, DynClient, ReqwestClient};

/// The global settings folder (`<config dir>/gitkraft`).
fn global_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("gitkraft"))
}

/// Writes the starter `.gitkraft/ai.toml` unless one exists.
///
/// # Errors
/// The file cannot be written.
pub fn init(dir: &Path) -> Result<String> {
    let path = dir.join(".gitkraft").join("ai.toml");
    if path.exists() {
        bail!("{} exists already", path.display());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, commit_ai::EXAMPLE)?;
    Ok(format!(
        "wrote {}: edit it to fit the project\n",
        path.display()
    ))
}

/// The prompt that would be sent, or the drafted message (with rule warnings on stderr-style lines
/// after it as `! …`).
///
/// # Errors
/// Nothing staged, a broken style file, or the model call.
pub async fn message(
    llm: Option<&impl Llm>,
    dir: &Path,
    global: Option<&Path>,
    json: bool,
) -> Result<String> {
    let style = RawStyle::load(dir, global).map_err(|e| anyhow!(e))?;
    let input =
        commit_ai::gather(dir, style.examples_from_history.unwrap_or(8)).map_err(|e| anyhow!(e))?;
    let Some(llm) = llm else {
        return Ok(format!("{}\n", commit_ai::user_prompt(&input)));
    };
    let s = commit_ai::suggest(llm, &style, &input).await?;
    if json {
        return Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(
                &serde_json::json!({"message": s.message, "warnings": s.warnings})
            )?
        ));
    }
    let mut out = s.message;
    for w in &s.warnings {
        out.push_str(&format!("! {w}\n"));
    }
    Ok(out)
}

/// The model from the environment and the keyring.
pub(crate) fn real_llm() -> Result<Remote> {
    let env = |n: &str| std::env::var(n).ok();
    let config = AiConfig::from_env(&env);
    let key = config.key(&env, &SystemKeyring)?;
    let client = DynClient::new(CachedClient::new(
        ReqwestClient::new().map_err(|e| anyhow!(e.to_string()))?,
    ));
    Ok(Remote::new(client, config, key))
}

/// A drafted message for the repository at `dir`, blocking; for a background thread of a UI.
///
/// # Errors
/// Nothing staged, a broken style file, no key, or the model call.
pub fn draft(dir: &Path) -> Result<commit_ai::Suggestion> {
    let style = RawStyle::load(dir, global_dir().as_deref()).map_err(|e| anyhow!(e))?;
    let input =
        commit_ai::gather(dir, style.examples_from_history.unwrap_or(8)).map_err(|e| anyhow!(e))?;
    let llm = real_llm()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    Ok(rt.block_on(commit_ai::suggest(&llm, &style, &input))?)
}

/// Runs the command for real: the model from the environment and the keyring.
///
/// # Errors
/// See [`message`]; also a missing key.
pub fn run(dir: &Path, dry_run: bool, init_file: bool, json: bool) -> Result<String> {
    if init_file {
        return init(dir);
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    if dry_run {
        return rt.block_on(message(None::<&Remote>, dir, global_dir().as_deref(), json));
    }
    let llm = real_llm()?;
    rt.block_on(message(Some(&llm), dir, global_dir().as_deref(), json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_hub::ai::FakeLlm;

    fn staged_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("dir");
        let repo = git2::Repository::init(dir.path()).expect("init");
        std::fs::write(dir.path().join("a.txt"), "hello\n").expect("write");
        let mut idx = repo.index().expect("index");
        idx.add_path(Path::new("a.txt")).expect("add");
        idx.write().expect("write");
        dir
    }

    #[test]
    fn init_writes_a_starter_once() {
        let dir = tempfile::tempdir().expect("dir");
        assert!(init(dir.path()).expect("init").contains("ai.toml"));
        assert!(init(dir.path()).is_err(), "never overwrites");
        assert!(RawStyle::load(dir.path(), None).is_ok());
    }

    #[tokio::test]
    async fn the_message_follows_the_projects_template() {
        let dir = staged_repo();
        std::fs::create_dir(dir.path().join(".gitkraft")).expect("dir");
        std::fs::write(
            dir.path().join(".gitkraft/ai.toml"),
            "[commit]\ntemplate = \"{subject}\\n\\nBy gitkraft\"\n",
        )
        .expect("write");
        let llm = FakeLlm::saying(r#"{"type":"feat","scope":"","subject":"add hello","body":""}"#);
        let out = message(Some(&llm), dir.path(), None, false)
            .await
            .expect("message");
        assert_eq!(out, "add hello\n\nBy gitkraft\n");
        assert!(llm.prompts()[0].1.contains("+hello"));
    }

    #[tokio::test]
    async fn a_dry_run_prints_the_prompt_and_nothing_staged_is_an_error() {
        let dir = staged_repo();
        let out = message(None::<&FakeLlm>, dir.path(), None, false)
            .await
            .expect("prompt");
        assert!(out.contains("Staged diff:") && out.contains("+hello"));
        let empty = tempfile::tempdir().expect("dir");
        git2::Repository::init(empty.path()).expect("init");
        assert!(
            message(None::<&FakeLlm>, empty.path(), None, false)
                .await
                .is_err()
        );
    }
}
