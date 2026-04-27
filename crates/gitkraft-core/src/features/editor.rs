//! Editor configuration — which editor/IDE to launch for file editing.

use serde::{Deserialize, Serialize};

/// Supported editors and IDEs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Editor {
    /// No editor configured.
    #[default]
    None,
    Helix,
    Neovim,
    Vim,
    Nano,
    Micro,
    Emacs,
    VSCode,
    Zed,
    Sublime,
    RustRover,
    IntelliJIdea,
    WebStorm,
    PyCharm,
    GoLand,
    CLion,
    Fleet,
    AndroidStudio,
    /// A user-supplied binary name or path.
    Custom(String),
}

/// All named editor variants (excluding None and Custom) for UI pickers.
pub const EDITOR_NAMES: &[&str] = &[
    "Helix",
    "Neovim",
    "Vim",
    "Nano",
    "Micro",
    "Emacs",
    "VS Code",
    "Zed",
    "Sublime Text",
    "RustRover",
    "IntelliJ IDEA",
    "WebStorm",
    "PyCharm",
    "GoLand",
    "CLion",
    "Fleet",
    "Android Studio",
];

impl Editor {
    /// Return the launch binary for this editor. Returns `None` for `Editor::None`.
    pub fn binary(&self) -> Option<String> {
        self.binary_candidates().into_iter().next()
    }

    /// Return the ordered list of binary names to try when launching this editor.
    ///
    /// Most editors have exactly one binary name.  Helix has two (`helix` on
    /// Linux, `hx` on macOS) so we return both and let the caller try them in
    /// order, stopping at the first one that is found in `$PATH`.
    pub fn binary_candidates(&self) -> Vec<String> {
        match self {
            Editor::None => vec![],
            Editor::Helix => {
                // macOS (Homebrew) installs Helix as `hx`.
                // Linux package managers install it as `helix` (Arch, Debian,
                // Fedora) or occasionally `hx`.  We try the platform default
                // first, then the alternative.
                if cfg!(target_os = "macos") {
                    vec!["hx".into(), "helix".into()]
                } else {
                    vec!["helix".into(), "hx".into()]
                }
            }
            Editor::Neovim => vec!["nvim".into()],
            Editor::Vim => vec!["vim".into()],
            Editor::Nano => vec!["nano".into()],
            Editor::Micro => vec!["micro".into()],
            Editor::Emacs => vec!["emacs".into()],
            Editor::VSCode => vec!["code --reuse-window".into()],
            Editor::Zed => vec!["zed".into()],
            Editor::Sublime => vec!["subl".into()],
            Editor::RustRover => vec!["rustrover".into()],
            Editor::IntelliJIdea => vec!["idea".into()],
            Editor::WebStorm => vec!["webstorm".into()],
            Editor::PyCharm => vec!["pycharm".into()],
            Editor::GoLand => vec!["goland".into()],
            Editor::CLion => vec!["clion".into()],
            Editor::Fleet => vec!["fleet".into()],
            Editor::AndroidStudio => vec!["studio".into()],
            Editor::Custom(s) => vec![s.clone()],
        }
    }

    /// Returns `true` for editors that run inside a terminal (TTY required).
    ///
    /// These editors cannot be spawned in the background from a TUI
    /// application — the TUI must suspend itself first, run the editor
    /// synchronously, then resume.
    pub fn is_terminal_editor(&self) -> bool {
        matches!(
            self,
            Editor::Helix
                | Editor::Neovim
                | Editor::Vim
                | Editor::Nano
                | Editor::Micro
                | Editor::Emacs
        )
    }

    /// macOS application bundle name for GUI editors.
    /// Returns `None` for terminal editors (they can't be activated via `open -a`).
    #[cfg(target_os = "macos")]
    fn macos_app_name(&self) -> Option<&'static str> {
        match self {
            Editor::VSCode => Some("Visual Studio Code"),
            Editor::Zed => Some("Zed"),
            Editor::Sublime => Some("Sublime Text"),
            Editor::RustRover => Some("RustRover"),
            Editor::IntelliJIdea => Some("IntelliJ IDEA"),
            Editor::WebStorm => Some("WebStorm"),
            Editor::PyCharm => Some("PyCharm"),
            Editor::GoLand => Some("GoLand"),
            Editor::CLion => Some("CLion"),
            Editor::Fleet => Some("Fleet"),
            Editor::AndroidStudio => Some("Android Studio"),
            // Terminal editors and Helix/Neovim/Vim/Nano/Micro/Emacs don't
            // have a stable macOS bundle name we can rely on for `open -a`.
            _ => None,
        }
    }

    /// Display name for the editor.
    pub fn display_name(&self) -> &str {
        match self {
            Editor::None => "None",
            Editor::Helix => "Helix",
            Editor::Neovim => "Neovim",
            Editor::Vim => "Vim",
            Editor::Nano => "Nano",
            Editor::Micro => "Micro",
            Editor::Emacs => "Emacs",
            Editor::VSCode => "VS Code",
            Editor::Zed => "Zed",
            Editor::Sublime => "Sublime Text",
            Editor::RustRover => "RustRover",
            Editor::IntelliJIdea => "IntelliJ IDEA",
            Editor::WebStorm => "WebStorm",
            Editor::PyCharm => "PyCharm",
            Editor::GoLand => "GoLand",
            Editor::CLion => "CLion",
            Editor::Fleet => "Fleet",
            Editor::AndroidStudio => "Android Studio",
            Editor::Custom(_) => "Custom",
        }
    }

    /// Get editor by index into EDITOR_NAMES.
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Editor::Helix,
            1 => Editor::Neovim,
            2 => Editor::Vim,
            3 => Editor::Nano,
            4 => Editor::Micro,
            5 => Editor::Emacs,
            6 => Editor::VSCode,
            7 => Editor::Zed,
            8 => Editor::Sublime,
            9 => Editor::RustRover,
            10 => Editor::IntelliJIdea,
            11 => Editor::WebStorm,
            12 => Editor::PyCharm,
            13 => Editor::GoLand,
            14 => Editor::CLion,
            15 => Editor::Fleet,
            16 => Editor::AndroidStudio,
            _ => Editor::None,
        }
    }

    /// Open a file in this editor as a **background** process.
    ///
    /// Stdin/stdout/stderr are detached so the caller is not blocked.
    /// This is correct for **GUI editors** (VS Code, Zed, IntelliJ, …).
    ///
    /// **Do not call this for terminal editors from a running TUI** — use the
    /// `pending_editor_open` mechanism in `App` instead so the TUI can
    /// suspend itself before handing the terminal to the editor.
    ///
    /// On macOS, GUI editors are opened via `open -a "App Name" file` so the
    /// existing application window is activated and brought to the front.
    pub fn open_file(&self, file_path: &std::path::Path) -> anyhow::Result<()> {
        #[cfg(target_os = "macos")]
        if let Some(app_name) = self.macos_app_name() {
            std::process::Command::new("open")
                .arg("-a")
                .arg(app_name)
                .arg(file_path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| anyhow::anyhow!("failed to run 'open -a {}': {}", app_name, e))?;
            return Ok(());
        }

        // Try each binary candidate in order, stopping at the first found.
        let candidates = self.binary_candidates();
        if candidates.is_empty() {
            anyhow::bail!("no editor configured — select one from the editor picker");
        }
        let mut last_err =
            anyhow::anyhow!("no editor configured — select one from the editor picker");
        for bin in &candidates {
            let parts: Vec<&str> = bin.split_whitespace().collect();
            let Some((cmd, args)) = parts.split_first() else {
                continue;
            };
            match std::process::Command::new(cmd)
                .args(args.iter())
                .arg(file_path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(_) => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    last_err = anyhow::anyhow!("'{}' not found in PATH", cmd);
                    continue;
                }
                Err(e) => return Err(anyhow::anyhow!("failed to launch '{}': {}", cmd, e)),
            }
        }
        Err(last_err)
    }

    /// Open a file in this editor, falling back to the system default opener
    /// (`xdg-open` / `open` / `start`) when no editor is configured or the
    /// configured editor fails to launch.
    ///
    /// Always succeeds as long as the system has a default file handler.
    /// Returns the method used: `"editor"`, `"system default"`, or an error.
    pub fn open_file_or_default(&self, file_path: &std::path::Path) -> anyhow::Result<String> {
        if !matches!(self, Editor::None) {
            match self.open_file(file_path) {
                Ok(()) => return Ok(self.display_name().to_string()),
                Err(e) => {
                    tracing::warn!(
                        "configured editor failed ({e}), falling back to system default"
                    );
                }
            }
        }
        open_file_default(file_path)?;
        Ok("system default".to_string())
    }
}

impl std::fmt::Display for Editor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Open a file in the system's default application (xdg-open, open, etc).
pub fn open_file_default(file_path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(file_path)
            .spawn()
            .map_err(|e| anyhow::anyhow!("xdg-open failed: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(file_path)
            .spawn()
            .map_err(|e| anyhow::anyhow!("open failed: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(file_path)
            .spawn()
            .map_err(|e| anyhow::anyhow!("start failed: {}", e))?;
    }
    Ok(())
}

/// Open a folder in the system's file manager.
pub fn show_in_folder(file_path: &std::path::Path) -> anyhow::Result<()> {
    let folder = if file_path.is_file() {
        file_path.parent().unwrap_or(file_path)
    } else {
        file_path
    };
    open_file_default(folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_binary_returns_correct_values() {
        assert_eq!(Editor::Neovim.binary(), Some("nvim".into()));
        assert_eq!(Editor::VSCode.binary(), Some("code --reuse-window".into()));
        assert_eq!(Editor::None.binary(), None);
        assert_eq!(
            Editor::Custom("my-editor".into()).binary(),
            Some("my-editor".into())
        );
    }

    #[test]
    fn helix_binary_candidates_platform_default_first() {
        let candidates = Editor::Helix.binary_candidates();
        assert_eq!(candidates.len(), 2);
        #[cfg(target_os = "macos")]
        assert_eq!(candidates[0], "hx");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(candidates[0], "helix");
    }

    #[test]
    fn is_terminal_editor_classifies_correctly() {
        assert!(Editor::Helix.is_terminal_editor());
        assert!(Editor::Neovim.is_terminal_editor());
        assert!(Editor::Vim.is_terminal_editor());
        assert!(Editor::Nano.is_terminal_editor());
        assert!(!Editor::VSCode.is_terminal_editor());
        assert!(!Editor::Zed.is_terminal_editor());
        assert!(!Editor::IntelliJIdea.is_terminal_editor());
        assert!(!Editor::None.is_terminal_editor());
    }

    #[test]
    fn binary_candidates_single_for_most_editors() {
        assert_eq!(Editor::Neovim.binary_candidates(), vec!["nvim"]);
        assert_eq!(
            Editor::VSCode.binary_candidates(),
            vec!["code --reuse-window"]
        );
        assert_eq!(Editor::Zed.binary_candidates(), vec!["zed"]);
    }

    #[test]
    fn editor_display_name() {
        assert_eq!(Editor::VSCode.display_name(), "VS Code");
        assert_eq!(Editor::IntelliJIdea.display_name(), "IntelliJ IDEA");
        assert_eq!(Editor::None.display_name(), "None");
    }

    #[test]
    fn editor_from_index_round_trips() {
        for i in 0..EDITOR_NAMES.len() {
            let editor = Editor::from_index(i);
            assert_ne!(editor, Editor::None, "index {i} should not be None");
        }
        assert_eq!(Editor::from_index(999), Editor::None);
    }

    #[test]
    fn editor_names_count_matches() {
        assert_eq!(EDITOR_NAMES.len(), 17);
    }

    #[test]
    fn editor_serialize_deserialize() {
        let editor = Editor::VSCode;
        let json = serde_json::to_string(&editor).unwrap();
        let back: Editor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Editor::VSCode);
    }

    #[test]
    fn vscode_binary_includes_reuse_window() {
        assert_eq!(Editor::VSCode.binary(), Some("code --reuse-window".into()));
    }

    #[test]
    fn editor_display_implements_display_trait() {
        let editor = Editor::Neovim;
        assert_eq!(format!("{editor}"), "Neovim");
    }

    #[test]
    fn custom_editor_preserves_value() {
        let editor = Editor::Custom("/usr/bin/my-editor --flag".into());
        assert_eq!(editor.binary(), Some("/usr/bin/my-editor --flag".into()));
        assert_eq!(editor.display_name(), "Custom");
    }
}
