//! Pulls compiler errors and test failures out of a log, so the UI can list
//! them above the raw text and jump to the file.

use crate::log::{LineKind, LogLine};

/// How bad it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// A warning.
    Warning,
    /// An error or failed test.
    Error,
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    /// Severity.
    pub severity: Severity,
    /// Message.
    pub message: String,
    /// Source file, when the log names one.
    pub file: Option<String>,
    /// 1-based line in `file`.
    pub line: Option<u32>,
    /// 1-based column.
    pub column: Option<u32>,
    /// Index into the parsed log lines, to jump to.
    pub log_index: usize,
}

/// Finds annotations in `lines`.
pub fn extract(lines: &[LogLine]) -> Vec<Annotation> {
    let mut out: Vec<Annotation> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let t = l.text.trim();
        // rustc: `error[E0425]: msg` / `warning: msg`, location on a following ` --> file:l:c` line.
        if let Some((sev, msg)) = rustc_header(t) {
            let loc = lines
                .iter()
                .skip(i + 1)
                .take(3)
                .find_map(|n| n.text.trim().strip_prefix("--> ").and_then(parse_location));
            let (file, line, column) =
                loc.map_or((None, None, None), |(f, l, c)| (Some(f), Some(l), c));
            out.push(Annotation {
                severity: sev,
                message: msg.to_string(),
                file,
                line,
                column,
                log_index: i,
            });
            continue;
        }
        if let Some(msg) = t
            .strip_prefix("thread '")
            .and_then(|r| r.split_once("' panicked at "))
        {
            let (file, line, column) = parse_location(msg.1.trim_end_matches(':'))
                .map_or((None, None, None), |(f, l, c)| (Some(f), Some(l), c));
            out.push(Annotation {
                severity: Severity::Error,
                message: format!("{} panicked", msg.0),
                file,
                line,
                column,
                log_index: i,
            });
            continue;
        }
        if let Some(name) = t
            .strip_prefix("test ")
            .and_then(|r| r.strip_suffix(" ... FAILED"))
        {
            out.push(Annotation {
                severity: Severity::Error,
                message: format!("test {name} failed"),
                file: None,
                line: None,
                column: None,
                log_index: i,
            });
            continue;
        }
        if let Some(rest) = t.strip_prefix("FAILED ") {
            // pytest short summary: `FAILED tests/test_a.py::test_x - AssertionError`
            let (id, msg) = rest.split_once(" - ").unwrap_or((rest, "failed"));
            let file = id
                .split("::")
                .next()
                .filter(|f| f.contains('.'))
                .map(str::to_string);
            out.push(Annotation {
                severity: Severity::Error,
                message: format!("{id}: {msg}"),
                file,
                line: None,
                column: None,
                log_index: i,
            });
            continue;
        }
        if let Some(name) = t.strip_prefix("--- FAIL: ") {
            out.push(Annotation {
                severity: Severity::Error,
                message: format!("go test {}", name.split(' ').next().unwrap_or(name)),
                file: None,
                line: None,
                column: None,
                log_index: i,
            });
            continue;
        }
        if let Some(rest) = t.strip_prefix("e: ").or_else(|| t.strip_prefix("w: ")) {
            // Kotlin / Gradle: `e: file:///path/Foo.kt:12:5 message`
            let sev = if t.starts_with("e:") {
                Severity::Error
            } else {
                Severity::Warning
            };
            let rest = rest.trim_start_matches("file://");
            let (loc, msg) = rest.split_once(' ').unwrap_or((rest, ""));
            let (file, line, column) =
                parse_location(loc).map_or((None, None, None), |(f, l, c)| (Some(f), Some(l), c));
            out.push(Annotation {
                severity: sev,
                message: msg.to_string(),
                file,
                line,
                column,
                log_index: i,
            });
            continue;
        }
        if let Some(a) = generic(t, i) {
            out.push(a);
            continue;
        }
        if l.kind == LineKind::Error && !t.is_empty() {
            out.push(Annotation {
                severity: Severity::Error,
                message: t.to_string(),
                file: None,
                line: None,
                column: None,
                log_index: i,
            });
        }
    }
    dedupe(out)
}

/// `error: x` / `error[E0001]: x` / `warning: x`, but not the summary lines
/// (`error: could not compile`, `warning: 3 warnings emitted`).
fn rustc_header(t: &str) -> Option<(Severity, &str)> {
    let (sev, rest) = if let Some(r) = t.strip_prefix("error") {
        (Severity::Error, r)
    } else {
        (Severity::Warning, t.strip_prefix("warning")?)
    };
    let rest = if let Some(r) = rest.strip_prefix('[') {
        r.split_once(']')?.1
    } else {
        rest
    };
    let msg = rest.strip_prefix(": ")?;
    let summary = msg.starts_with("could not compile")
        || msg.starts_with("aborting due to")
        || msg.contains("warnings emitted")
        || msg.contains("warning emitted")
        || msg.starts_with("build failed")
        || msg.starts_with("test failed");
    (!summary).then_some((sev, msg))
}

/// `path:line:col` or `path:line` → (path, line, col).
fn parse_location(s: &str) -> Option<(String, u32, Option<u32>)> {
    let s = s.trim();
    let mut parts = s.rsplitn(3, ':');
    let last = parts.next()?;
    let mid = parts.next()?;
    match (mid.parse::<u32>(), last.parse::<u32>(), parts.next()) {
        (Ok(line), Ok(col), Some(file)) => {
            (!file.is_empty()).then(|| (file.to_string(), line, Some(col)))
        }
        _ => {
            // `path:line`
            let (file, line) = s.rsplit_once(':')?;
            let line = line.parse::<u32>().ok()?;
            (!file.is_empty() && !file.contains(' ')).then(|| (file.to_string(), line, None))
        }
    }
}

/// gcc / clang / tsc / eslint-style: `path:line:col: error: message`.
fn generic(t: &str, log_index: usize) -> Option<Annotation> {
    for (marker, severity) in [
        (": error", Severity::Error),
        (": warning", Severity::Warning),
    ] {
        let Some(pos) = t.find(marker) else { continue };
        let (loc, rest) = t.split_at(pos);
        let (file, line, column) = parse_location(loc)?;
        let message = rest[marker.len()..]
            .trim_start_matches(|c: char| c == ':' || c.is_whitespace())
            .to_string();
        return Some(Annotation {
            severity,
            message,
            file: Some(file),
            line: Some(line),
            column,
            log_index,
        });
    }
    None
}

/// A rustc error is often echoed by the `##[error]` summary; one finding per message.
fn dedupe(mut v: Vec<Annotation>) -> Vec<Annotation> {
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut out: Vec<Annotation> = Vec::with_capacity(v.len());
    for a in v.drain(..) {
        match index.get(&a.message) {
            // Keep the entry that knows the file.
            Some(&i) => {
                if out[i].file.is_none() && a.file.is_some() {
                    out[i] = a;
                }
            }
            None => {
                index.insert(a.message.clone(), out.len());
                out.push(a);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::parse_log;

    fn run(raw: &str) -> Vec<Annotation> {
        extract(&parse_log(raw))
    }

    #[test]
    fn rustc_error_gets_its_location() {
        let a = run("error[E0425]: cannot find value `x`\n  --> src/main.rs:3:5\n   |\n");
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].message, "cannot find value `x`");
        assert_eq!(
            (a[0].file.as_deref(), a[0].line, a[0].column),
            (Some("src/main.rs"), Some(3), Some(5))
        );
        assert_eq!(a[0].severity, Severity::Error);
    }

    #[test]
    fn rustc_summary_lines_are_not_findings() {
        assert!(run("error: could not compile `x` (lib) due to 1 previous error\nwarning: 3 warnings emitted").is_empty());
    }

    #[test]
    fn warnings_keep_their_severity() {
        let a = run("warning: unused variable: `y`\n --> src/lib.rs:9:9");
        assert_eq!((a[0].severity, a[0].line), (Severity::Warning, Some(9)));
    }

    #[test]
    fn rust_test_failures_and_panics() {
        let a = run(
            "test a::b ... ok\ntest a::c ... FAILED\nthread 'a::c' panicked at src/a.rs:12:9:\nassertion failed",
        );
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].message, "test a::c failed");
        assert_eq!(
            (a[1].file.as_deref(), a[1].line, a[1].column),
            (Some("src/a.rs"), Some(12), Some(9))
        );
    }

    #[test]
    fn pytest_go_and_kotlin() {
        let a = run(
            "FAILED tests/test_a.py::test_x - AssertionError: no\n--- FAIL: TestFoo (0.01s)\ne: file:///w/src/Main.kt:12:5 Unresolved reference: foo\nw: file:///w/src/Main.kt:3:1 unused",
        );
        assert_eq!(a.len(), 4);
        assert_eq!(a[0].file.as_deref(), Some("tests/test_a.py"));
        assert_eq!(a[1].message, "go test TestFoo");
        assert_eq!(
            (a[2].file.as_deref(), a[2].line, a[2].severity),
            (Some("/w/src/Main.kt"), Some(12), Severity::Error)
        );
        assert_eq!(a[3].severity, Severity::Warning);
    }

    #[test]
    fn gcc_style_and_marked_errors() {
        let a = run(
            "src/a.c:10:3: error: expected ';'\nlib/x.ts:4:1: warning: unused\n##[error]Process completed with exit code 1.",
        );
        assert_eq!(a.len(), 3);
        assert_eq!(
            (a[0].file.as_deref(), a[0].line, a[0].column),
            (Some("src/a.c"), Some(10), Some(3))
        );
        assert_eq!(a[0].message, "expected ';'");
        assert_eq!(a[2].file, None);
        assert_eq!(a[2].log_index, 2);
    }

    #[test]
    fn ordinary_text_with_colons_is_ignored() {
        assert!(
            run("Downloading: 10:20\nTime: 12:30:45 error budget ok\nhttp://x:80/y").is_empty()
        );
    }

    #[test]
    fn duplicates_collapse() {
        let a = run("error: boom\n --> a.rs:1:1\n##[error]boom");
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn location_parser_edges() {
        assert_eq!(
            parse_location("a.rs:1:2"),
            Some(("a.rs".into(), 1, Some(2)))
        );
        assert_eq!(parse_location("a.rs:7"), Some(("a.rs".into(), 7, None)));
        assert_eq!(
            parse_location("C:\\x\\a.rs:7:1"),
            Some(("C:\\x\\a.rs".into(), 7, Some(1)))
        );
        assert_eq!(parse_location("nothing"), None);
        assert_eq!(parse_location(":1:2"), None);
    }
}
