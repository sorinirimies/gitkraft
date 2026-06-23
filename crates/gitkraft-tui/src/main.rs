use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    // Limit libgit2's memory-mapped window to prevent GB-scale RAM usage
    // on large repositories.
    unsafe {
        git2::opts::set_mwindow_size(64 * 1024 * 1024).ok();
        git2::opts::set_mwindow_mapped_limit(256 * 1024 * 1024).ok();
    }

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

    gitkraft_tui::run(repo_path)
}
