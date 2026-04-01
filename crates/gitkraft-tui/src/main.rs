use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Try to pick up a repo path from the first CLI argument, or fall back to
    // the current working directory if it looks like a Git repository.
    let repo_path = std::env::args().nth(1).map(PathBuf::from).or_else(|| {
        let cwd = std::env::current_dir().ok()?;
        // Check if cwd (or any ancestor) contains a .git dir
        if cwd.join(".git").exists() {
            Some(cwd)
        } else {
            None
        }
    });

    gitkraft_tui::run(repo_path).await
}
