//! End to end: the real binary, real stdio, speaking MCP.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

static HOME: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(|| {
    let dir = std::env::temp_dir().join(format!("gitkraft-mcp-home-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("home");
    dir
});

fn bin() -> Command {
    let home = &*HOME;
    let mut c = Command::new(env!("CARGO_BIN_EXE_gitkraft-mcp"));
    // a clean slate: no tokens from the environment, nowhere to find a checkout
    c.env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("CODEBERG_TOKEN")
        .env_remove("GITKRAFT_MCP_WRITE")
        .env_remove("GITKRAFT_MCP_FORCE")
        .env_remove("GITKRAFT_MCP_REPO")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        // an empty home: no config dir, no `gh` login, no credential store of the real user
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("APPDATA", home)
        .current_dir(home);
    c
}

/// Runs a session: initialize, then `calls`; returns the answer of each call.
fn session(args: &[&str], calls: &[Value]) -> Vec<Value> {
    let mut child = bin()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    let stdin = child.stdin.take().expect("stdin");
    let mut stdin = Some(stdin);
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
    let mut rpc = |req: &Value| -> Value {
        let w = stdin.as_mut().expect("open");
        writeln!(w, "{req}").expect("write");
        w.flush().expect("flush");
        if req.get("id").is_none() {
            return Value::Null;
        }
        let mut line = String::new();
        stdout.read_line(&mut line).expect("read");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("bad reply {line:?}: {e}"))
    };
    let init = rpc(
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}),
    );
    assert_eq!(
        init["result"]["serverInfo"]["name"], "gitkraft-mcp",
        "{init}"
    );
    rpc(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    let answers = calls.iter().map(&mut rpc).collect();
    drop(rpc);
    drop(stdin.take());
    let _ = child.wait();
    answers
}

fn names(answer: &Value) -> Vec<String> {
    answer["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn list() -> Value {
    json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})
}

#[test]
fn version_and_help() {
    let out = bin().arg("--version").output().expect("run");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        format!("gitkraft-mcp {}", env!("CARGO_PKG_VERSION"))
    );
    let out = bin().arg("--help").output().expect("run");
    assert!(String::from_utf8_lossy(&out.stdout).contains("--allow-force"));
}

#[test]
fn bad_arguments_are_a_usage_error() {
    assert_eq!(
        bin().arg("--nope").output().expect("run").status.code(),
        Some(2)
    );
    assert_eq!(
        bin()
            .arg("--allow-force")
            .output()
            .expect("run")
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn read_only_by_default_and_write_tools_appear_with_the_flag() {
    let ro = names(&session(&[], &[list()])[0]);
    assert!(
        ro.contains(&"ci_runs".to_string()) && !ro.contains(&"merge_pull_request".to_string()),
        "{ro:?}"
    );
    let rw = names(&session(&["--write"], &[list()])[0]);
    assert!(rw.contains(&"merge_pull_request".to_string()) && rw.contains(&"ci_runs".to_string()));
    assert!(rw.len() > ro.len());
}

#[test]
fn read_tools_are_marked_read_only() {
    let a = &session(&["--write"], &[list()])[0];
    for t in a["result"]["tools"].as_array().expect("tools") {
        let name = t["name"].as_str().unwrap_or_default();
        let ro = t["annotations"]["readOnlyHint"].as_bool();
        let writes = [
            "comment",
            "review",
            "merge_pull_request",
            "update_branch",
            "cancel_auto_merge",
            "set_open",
            "edit_labels",
            "ci_rerun",
            "ci_cancel",
            "dispatch_workflow",
        ];
        assert_eq!(ro, Some(!writes.contains(&name)), "{name}: {t}");
    }
}

#[test]
fn a_tool_call_answers_and_an_unsound_one_is_an_error() {
    let a = session(
        &[],
        &[
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"accounts","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_pull_requests","arguments":{"repo":"nonsense"}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"merge_pull_request","arguments":{"number":1}}}),
        ],
    );
    let text = a[0]["result"]["content"][0]["text"].as_str().expect("text");
    assert_eq!(
        serde_json::from_str::<Value>(text).expect("json")["write"],
        false
    );
    assert!(
        a[1]["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("host/owner/name")),
        "{}",
        a[1]
    );
    assert!(
        a[2].get("error").is_some(),
        "no such tool read-only: {}",
        a[2]
    );
}
