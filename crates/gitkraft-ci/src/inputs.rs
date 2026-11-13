//! The inputs a workflow asks for when started by hand (`on: workflow_dispatch: inputs:`).
//!
//! A small reader for the shape these files have, not a YAML parser: it follows indentation, which is
//! enough for the keys that matter and never fails on what it does not understand.

/// One input of `workflow_dispatch`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowInput {
    /// Key to pass.
    pub name: String,
    /// What it is for.
    pub description: String,
    /// Must be given when there is no default.
    pub required: bool,
    /// Value used when none is given.
    pub default: Option<String>,
    /// `string`, `boolean`, `choice`, `number` or `environment`.
    pub kind: String,
    /// The allowed values of a `choice`.
    pub options: Vec<String>,
}

impl WorkflowInput {
    /// Why `value` is not acceptable, if it is not.
    pub fn problem(&self, value: &str) -> Option<String> {
        match self.kind.as_str() {
            "boolean" if !matches!(value, "true" | "false") => {
                Some(format!("{} must be true or false", self.name))
            }
            "number" if value.parse::<f64>().is_err() => {
                Some(format!("{} must be a number", self.name))
            }
            "choice" if !self.options.is_empty() && !self.options.iter().any(|o| o == value) => {
                Some(format!(
                    "{} must be one of: {}",
                    self.name,
                    self.options.join(", ")
                ))
            }
            _ => None,
        }
    }
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if let Some(q) = v.chars().next().filter(|c| *c == '"' || *c == '\'')
        && let Some(end) = v[1..].find(q)
    {
        // The quoted text; whatever follows (a comment) is dropped.
        return v[1..1 + end].to_string();
    }
    v.split(" #").next().unwrap_or(v).trim().to_string()
}

/// The inputs `workflow_dispatch` declares in `yaml`; empty when there are none.
pub fn workflow_inputs(yaml: &str) -> Vec<WorkflowInput> {
    let lines: Vec<&str> = yaml
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .collect();
    // Child lines of the block that starts after line `i` (deeper than it).
    let children = |i: usize| -> &[&str] {
        let base = indent(lines[i]);
        let end = lines[i + 1..]
            .iter()
            .position(|l| indent(l) <= base)
            .map_or(lines.len(), |p| i + 1 + p);
        &lines[i + 1..end]
    };
    let Some(wd) = lines
        .iter()
        .position(|l| l.trim_start().trim_end() == "workflow_dispatch:")
    else {
        return Vec::new();
    };
    let block = children(wd);
    // Re-base on the block so `children` works inside it.
    let Some(inputs_at) = block.iter().position(|l| l.trim() == "inputs:") else {
        return Vec::new();
    };
    let inputs_indent = indent(block[inputs_at]);
    let body: Vec<&str> = block[inputs_at + 1..]
        .iter()
        .take_while(|l| indent(l) > inputs_indent)
        .copied()
        .collect();
    let Some(first) = body.first() else {
        return Vec::new();
    };
    let name_indent = indent(first);
    let mut out: Vec<WorkflowInput> = Vec::new();
    let mut in_options = false;
    for line in body {
        let d = indent(line);
        let t = line.trim();
        if d == name_indent {
            in_options = false;
            let Some((name, _)) = t.split_once(':') else {
                continue;
            };
            out.push(WorkflowInput {
                name: unquote(name),
                kind: "string".into(),
                ..WorkflowInput::default()
            });
            continue;
        }
        let Some(cur) = out.last_mut() else { continue };
        if in_options && let Some(item) = t.strip_prefix("- ") {
            cur.options.push(unquote(item));
            continue;
        }
        in_options = false;
        let Some((k, v)) = t.split_once(':') else {
            continue;
        };
        let v = unquote(v);
        match k.trim() {
            "description" => cur.description = v,
            "required" => cur.required = v == "true",
            "default" => cur.default = Some(v),
            "type" => cur.kind = v,
            "options" => in_options = true,
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const WF: &str = r#"
name: Release
on:
  push:
    branches: [main]
  workflow_dispatch:
    inputs:
      level:
        description: 'Log level'   # shown in the UI
        required: true
        type: choice
        options:
          - debug
          - "info"
        default: info
      dry:
        description: Dry run
        type: boolean
        default: false
      tag:
        required: true
jobs:
  build:
    runs-on: ubuntu-latest
    inputs:
      not: this
"#;

    #[test]
    fn inputs_of_workflow_dispatch_are_read_with_their_details() {
        let got = workflow_inputs(WF);
        assert_eq!(got.len(), 3, "{got:?}");
        assert_eq!(got[0].name, "level");
        assert_eq!(got[0].description, "Log level");
        assert!(got[0].required);
        assert_eq!(
            (got[0].kind.as_str(), got[0].default.as_deref()),
            ("choice", Some("info"))
        );
        assert_eq!(got[0].options, ["debug", "info"]);
        assert_eq!(
            (got[1].kind.as_str(), got[1].default.as_deref()),
            ("boolean", Some("false"))
        );
        assert!(!got[1].required);
        assert_eq!(
            (
                got[2].name.as_str(),
                got[2].required,
                got[2].default.clone()
            ),
            ("tag", true, None)
        );
        assert_eq!(got[2].kind, "string");
    }

    #[test]
    fn workflows_without_inputs_give_none() {
        assert!(workflow_inputs("on: push\njobs: {}\n").is_empty());
        assert!(workflow_inputs("on:\n  workflow_dispatch:\njobs: {}\n").is_empty());
        assert!(workflow_inputs("on:\n  workflow_dispatch:\n    inputs:\njobs: {}\n").is_empty());
        assert!(workflow_inputs("").is_empty());
    }

    #[test]
    fn values_are_checked_against_the_type() {
        let got = workflow_inputs(WF);
        assert!(got[0].problem("info").is_none());
        assert!(got[0].problem("loud").expect("bad").contains("debug, info"));
        assert!(got[1].problem("true").is_none());
        assert!(got[1].problem("yes").is_some());
        assert!(got[2].problem("anything").is_none());
        let n = WorkflowInput {
            name: "n".into(),
            kind: "number".into(),
            ..Default::default()
        };
        assert!(n.problem("3.5").is_none() && n.problem("x").is_some());
    }
}
