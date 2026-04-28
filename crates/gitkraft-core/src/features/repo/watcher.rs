//! Background git-state watcher.
//!
//! Watches the `.git` directory with the OS's native file-system notification
//! API (inotify on Linux, FSEvents on macOS, ReadDirectoryChangesW on Windows)
//! and calls a callback after each debounced burst of changes.
//!
//! Falls back to a 5-second poll when:
//! - the `notify` watcher can't be set up (e.g. network file system), or
//! - no events arrive within 5 seconds (ensures the UI refreshes even when
//!   notifications are missed).

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Spawn a background thread that watches `git_dir` for changes and calls
/// `on_change` after each debounced burst.
///
/// The thread exits when `on_change` returns `false` (the caller signals it
/// should stop — typically because a channel send failed).
///
/// # Debouncing
///
/// A single git operation (commit, checkout, stash, …) writes multiple files
/// to `.git` in quick succession.  The watcher drains all buffered events and
/// then sleeps 300 ms so the entire burst counts as one refresh.
///
/// # Fallback poll
///
/// If no events arrive within 5 seconds the callback is also fired, ensuring
/// the UI reflects external changes on network file systems or in CI
/// environments where inotify is unreliable.
pub fn spawn_git_watcher<F>(git_dir: PathBuf, on_change: F) -> JoinHandle<()>
where
    F: Fn() -> bool + Send + 'static,
{
    thread::spawn(move || {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

        let (raw_tx, raw_rx) = mpsc::channel::<notify::Result<notify::Event>>();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = raw_tx.send(res);
            },
            Config::default(),
        )
        .ok();

        if let Some(ref mut w) = watcher {
            let _ = w.watch(&git_dir, RecursiveMode::Recursive);
        }

        loop {
            // Block until a notify event arrives or 5 seconds elapse (fallback poll).
            let _ = raw_rx.recv_timeout(Duration::from_secs(5));
            // Drain any extra events so a rapid burst counts as one refresh.
            while raw_rx.try_recv().is_ok() {}
            // Debounce: give git time to finish writing all its index files.
            thread::sleep(Duration::from_millis(300));
            while raw_rx.try_recv().is_ok() {}

            // Call the callback; stop the thread if it returns false.
            if !on_change() {
                break;
            }
        }
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn wait_for<F: Fn() -> bool>(condition: F, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if condition() {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        false
    }

    #[test]
    fn watcher_calls_callback_when_git_head_changes() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();
        // Create a minimal HEAD file so the directory looks like a git repo.
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let fired = Arc::new(Mutex::new(false));
        let fired_clone = Arc::clone(&fired);

        let _handle = spawn_git_watcher(git_dir.clone(), move || {
            *fired_clone.lock().unwrap() = true;
            false // stop after the first callback
        });

        // Give the watcher a moment to set up before triggering a change.
        thread::sleep(Duration::from_millis(200));

        // Simulate a branch checkout by rewriting HEAD.
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/feature\n").unwrap();

        // The callback fires after the 300 ms debounce — wait up to 2 s.
        assert!(
            wait_for(|| *fired.lock().unwrap(), Duration::from_secs(2)),
            "watcher did not call on_change within 2 seconds after HEAD changed"
        );
    }

    #[test]
    fn watcher_fires_fallback_poll_when_no_events() {
        // Use a path that doesn't exist — notify will fail to watch it, so no
        // events will ever arrive.  The fallback 5-second poll must still fire.
        let dir = tempfile::tempdir().unwrap();
        let fake_git = dir.path().join(".git_nonexistent");

        let fired = Arc::new(Mutex::new(false));
        let fired_clone = Arc::clone(&fired);

        let _handle = spawn_git_watcher(fake_git, move || {
            *fired_clone.lock().unwrap() = true;
            false
        });

        // The fallback triggers after 5 s + 300 ms debounce — wait up to 7 s.
        assert!(
            wait_for(|| *fired.lock().unwrap(), Duration::from_secs(7)),
            "fallback poll did not fire within 7 seconds"
        );
    }

    #[test]
    fn watcher_thread_exits_when_callback_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();

        let handle = spawn_git_watcher(git_dir.clone(), move || {
            false // immediately request exit on first call
        });

        // Thread must finish within 7 s (5 s fallback + 300 ms debounce + margin).
        assert!(
            wait_for(|| handle.is_finished(), Duration::from_secs(7)),
            "watcher thread did not exit after callback returned false"
        );
    }
}
