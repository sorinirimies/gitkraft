//! Background git-state watcher.
//!
//! Watches the `.git` directory with the OS's native file-system notification
//! API (inotify on Linux, FSEvents on macOS, ReadDirectoryChangesW on Windows)
//! and calls a callback after each debounced burst of changes.
//!
//! Reactive events fire within ~300 ms of any `.git` change.
//! A configurable fallback poll fires when no events arrive within the timeout
//! (default 60 s) — useful for network file systems or CI environments where
//! inotify events may not be delivered.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Spawn a background thread that watches `git_dir` for changes and calls
/// `on_change` after each debounced burst.
///
/// Uses a 60-second fallback poll so that the UI eventually reflects external
/// changes even on network file systems. Use [`spawn_git_watcher_with_fallback`]
/// when a custom fallback interval is needed (e.g. in tests).
pub fn spawn_git_watcher<F>(git_dir: PathBuf, on_change: F) -> JoinHandle<()>
where
    F: Fn() -> bool + Send + 'static,
{
    spawn_git_watcher_with_fallback(git_dir, Duration::from_secs(60), on_change)
}

/// Like [`spawn_git_watcher`] but with a custom fallback poll `timeout`.
///
/// Useful in tests to keep waiting times short.
pub fn spawn_git_watcher_with_fallback<F>(
    git_dir: PathBuf,
    fallback: Duration,
    on_change: F,
) -> JoinHandle<()>
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
            // Block until a notify event arrives or the fallback timeout elapses.
            let _ = raw_rx.recv_timeout(fallback);
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
        // events will ever arrive.  Uses a 2-second fallback (not the production
        // 60-second default) to keep the test fast.
        let dir = tempfile::tempdir().unwrap();
        let fake_git = dir.path().join(".git_nonexistent");

        let fired = Arc::new(Mutex::new(false));
        let fired_clone = Arc::clone(&fired);

        let _handle =
            spawn_git_watcher_with_fallback(fake_git, Duration::from_secs(2), move || {
                *fired_clone.lock().unwrap() = true;
                false
            });

        // 2 s fallback + 300 ms debounce + margin.
        assert!(
            wait_for(|| *fired.lock().unwrap(), Duration::from_secs(4)),
            "fallback poll did not fire within 4 seconds"
        );
    }

    #[test]
    fn watcher_thread_exits_when_callback_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();

        // 2-second fallback so the thread exits quickly in CI.
        let handle =
            spawn_git_watcher_with_fallback(git_dir.clone(), Duration::from_secs(2), move || {
                false // immediately request exit on first call
            });

        // Thread must finish within 4 s (2 s fallback + 300 ms debounce + margin).
        assert!(
            wait_for(|| handle.is_finished(), Duration::from_secs(4)),
            "watcher thread did not exit after callback returned false"
        );
    }
}
