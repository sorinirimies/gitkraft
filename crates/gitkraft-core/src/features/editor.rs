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
        match self {
            Editor::None => Option::None,
            Editor::Helix => Some(Self::resolve_helix()),
            Editor::Neovim => Some("nvim".into()),
            Editor::Vim => Some("vim".into()),
            Editor::Nano => Some("nano".into()),
            Editor::Micro => Some("micro".into()),
            Editor::Emacs => Some("emacs".into()),
            Editor::VSCode => Some("code --reuse-window".into()),
            Editor::Zed => Some("zed".into()),
            Editor::Sublime => Some("subl".into()),
            Editor::RustRover => Some("rustrover".into()),
            Editor::IntelliJIdea => Some("idea".into()),
            Editor::WebStorm => Some("webstorm".into()),
            Editor::PyCharm => Some("pycharm".into()),
            Editor::GoLand => Some("goland".into()),
            Editor::CLion => Some("clion".into()),
            Editor::Fleet => Some("fleet".into()),
            Editor::AndroidStudio => Some("studio".into()),
            Editor::Custom(s) => Some(s.clone()),
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

    /// Open a file in this editor. Returns an error if the editor is not found.
    pub fn open_file(&self, file_path: &std::path::Path) -> anyhow::Result<()> {
        let bin = self.binary().ok_or_else(|| {
            anyhow::anyhow!("no editor configured — select one from the editor picker")
        })?;

        let parts: Vec<&str> = bin.split_whitespace().collect();
        let (cmd, args) = parts
            .split_first()
            .ok_or_else(|| anyhow::anyhow!("empty editor binary"))?;

        let mut command = std::process::Command::new(cmd);
        command.args(args.iter());
        command.arg(file_path);
        // Detach stdin/stdout/stderr so the editor runs independently
        command.stdin(std::process::Stdio::null());
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());
        command
            .spawn()
            .map_err(|e| anyhow::anyhow!("failed to launch '{}': {}", cmd, e))?;

        Ok(())
    }

    /// Probe `$PATH` for the Helix binary name.
    fn resolve_helix() -> String {
        for candidate in &["hx", "helix"] {
            if std::process::Command::new(candidate)
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok()
            {
                return candidate.to_string();
            }
        }
        "hx".to_string()
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
