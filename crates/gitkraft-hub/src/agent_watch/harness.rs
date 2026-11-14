//! The agents that can look at red CI, and how each is started. Processes only: the model never
//! gets a tool to push, merge or re-run, and the prompt says so.

use std::path::PathBuf;
use std::sync::Mutex;

use super::config::HarnessKind;

/// Runs a program and returns what it printed.
pub trait Exec: Send + Sync {
    /// Runs `program` with `args`; stdin is closed.
    ///
    /// # Errors
    /// The program is missing, fails, or prints nothing.
    fn run(&self, program: &str, args: &[String]) -> Result<String, String>;
}

/// The real thing.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemExec;

impl Exec for SystemExec {
    fn run(&self, program: &str, args: &[String]) -> Result<String, String> {
        let out = std::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("{program}: {e} (is it installed and on the PATH?)"))?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(format!(
                "{program} failed: {}",
                err.trim().lines().last().unwrap_or("no output")
            ));
        }
        Ok(text)
    }
}

/// Records what it was asked to run and answers from a fixed reply.
#[derive(Debug)]
pub struct ScriptedExec {
    /// `program arg arg`, one per call.
    pub calls: Mutex<Vec<String>>,
    /// What every call answers.
    pub reply: Result<String, String>,
}

impl Default for ScriptedExec {
    fn default() -> Self {
        Self {
            calls: Mutex::default(),
            reply: Ok(String::new()),
        }
    }
}

impl Exec for ScriptedExec {
    fn run(&self, program: &str, args: &[String]) -> Result<String, String> {
        if let Ok(mut c) = self.calls.lock() {
            c.push(format!("{program} {}", args.join(" ")));
        }
        self.reply.clone()
    }
}

/// What to start for a harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// The program.
    pub program: String,
    /// Its arguments, the prompt included.
    pub args: Vec<String>,
    /// A file the prompt was written to first (Zed).
    pub prompt_file: Option<PathBuf>,
}

/// The command for `kind`, or `None` for the built-in model.
#[must_use]
pub fn invocation(
    kind: HarnessKind,
    model: Option<&str>,
    prompt: &str,
    scratch: &std::path::Path,
) -> Option<Invocation> {
    let model_args = |flag: &str| -> Vec<String> {
        model
            .filter(|m| !m.is_empty())
            .map(|m| vec![flag.to_string(), m.to_string()])
            .unwrap_or_default()
    };
    match kind {
        HarnessKind::Builtin => None,
        HarnessKind::Pi => Some(Invocation {
            program: "pi".into(),
            args: [
                vec!["-p".to_string()],
                model_args("--model"),
                vec![prompt.to_string()],
            ]
            .concat(),
            prompt_file: None,
        }),
        HarnessKind::Copilot => Some(Invocation {
            program: "copilot".into(),
            args: [
                vec!["-p".to_string(), prompt.to_string()],
                model_args("--model"),
            ]
            .concat(),
            prompt_file: None,
        }),
        HarnessKind::Vscode => Some(Invocation {
            program: "code".into(),
            args: vec!["chat".into(), prompt.to_string()],
            prompt_file: None,
        }),
        HarnessKind::Zed => {
            let file = scratch.join("agent-watch-prompt.md");
            Some(Invocation {
                program: "zed".into(),
                args: vec![file.display().to_string()],
                prompt_file: Some(file),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_harness_gets_its_own_command_and_the_model_flag_only_when_set() {
        let tmp = std::path::Path::new("/tmp/x");
        assert!(invocation(HarnessKind::Builtin, None, "p", tmp).is_none());
        let pi = invocation(HarnessKind::Pi, Some("m/x"), "look", tmp).expect("pi");
        assert_eq!(
            (pi.program.as_str(), pi.args),
            (
                "pi",
                ["-p", "--model", "m/x", "look"].map(String::from).to_vec()
            )
        );
        let pi = invocation(HarnessKind::Pi, None, "look", tmp).expect("pi");
        assert_eq!(pi.args, ["-p", "look"]);
        let cp = invocation(HarnessKind::Copilot, Some("gpt"), "look", tmp).expect("copilot");
        assert_eq!(cp.args, ["-p", "look", "--model", "gpt"]);
        let code = invocation(HarnessKind::Vscode, None, "look", tmp).expect("code");
        assert_eq!(code.args, ["chat", "look"]);
        let zed = invocation(HarnessKind::Zed, None, "look", tmp).expect("zed");
        assert_eq!(zed.prompt_file, Some(tmp.join("agent-watch-prompt.md")));
    }

    #[test]
    fn a_missing_program_says_so() {
        let e = SystemExec
            .run("gitkraft-no-such-program", &[])
            .expect_err("missing");
        assert!(e.contains("installed"), "{e}");
    }
}
