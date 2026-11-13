//! The text a user types to start a workflow by hand.

/// What to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatch {
    /// Workflow file (empty on GitLab).
    pub workflow: String,
    /// Branch or tag.
    pub git_ref: String,
    /// `key=value` inputs (GitLab: variables).
    pub inputs: Vec<(String, String)>,
}

/// `[WORKFLOW] REF [key=value …]`. On platforms without workflow files
/// (`needs_workflow == false`) the first word is the ref.
pub fn parse(text: &str, needs_workflow: bool) -> Result<Dispatch, String> {
    let mut words = text.split_whitespace();
    let workflow = if needs_workflow {
        words
            .next()
            .ok_or("name the workflow file, e.g. ci.yml")?
            .to_string()
    } else {
        String::new()
    };
    let git_ref = words
        .next()
        .ok_or("give the branch or tag to run on")?
        .to_string();
    let mut inputs = Vec::new();
    for w in words {
        let (k, v) = w
            .split_once('=')
            .filter(|(k, _)| !k.is_empty())
            .ok_or_else(|| format!("'{w}' is not key=value"))?;
        inputs.push((k.to_string(), v.to_string()));
    }
    Ok(Dispatch {
        workflow,
        git_ref,
        inputs,
    })
}

/// What the workflow file allows, applied to what the user typed: defaults fill what is missing, an
/// unknown key, a bad value or a missing required input is an error.
pub fn resolve(
    declared: &[gitkraft_ci::WorkflowInput],
    given: &[(String, String)],
) -> Result<Vec<(String, String)>, String> {
    if declared.is_empty() {
        return Ok(given.to_vec());
    }
    for (k, _) in given {
        if !declared.iter().any(|d| d.name == *k) {
            let names: Vec<&str> = declared.iter().map(|d| d.name.as_str()).collect();
            return Err(format!(
                "unknown input '{k}'; this workflow takes: {}",
                names.join(", ")
            ));
        }
    }
    let mut out = Vec::new();
    for d in declared {
        let value = given
            .iter()
            .find(|(k, _)| *k == d.name)
            .map(|(_, v)| v.clone())
            .or_else(|| d.default.clone());
        match value {
            Some(v) => {
                if let Some(p) = d.problem(&v) {
                    return Err(p);
                }
                out.push((d.name.clone(), v));
            }
            None if d.required => {
                let what = if d.description.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", d.description)
                };
                return Err(format!("needs input {}=…{what}", d.name));
            }
            None => {}
        }
    }
    Ok(out)
}

/// Starts the workflow: reads its inputs from the file, applies [`resolve`], dispatches. Returns what to tell the user.
pub async fn start<C: gitkraft_platforms::HttpClient + Clone>(
    ci: &gitkraft_ci::CiBackend<C>,
    repo: &gitkraft_platforms::RepoId,
    d: &Dispatch,
) -> Result<String, String> {
    let inputs = if d.workflow.is_empty() {
        d.inputs.clone()
    } else {
        // A file that cannot be read is not a reason to refuse: the platform checks the inputs too.
        let declared = ci
            .workflow_inputs(repo, &d.workflow, &d.git_ref)
            .await
            .unwrap_or_default();
        resolve(&declared, &d.inputs)?
    };
    ci.dispatch(repo, &d.workflow, &d.git_ref, &inputs)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "started {} on {}",
        if d.workflow.is_empty() {
            "a pipeline"
        } else {
            &d.workflow
        },
        d.git_ref
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared() -> Vec<gitkraft_ci::WorkflowInput> {
        gitkraft_ci::workflow_inputs(
            "on:\n  workflow_dispatch:\n    inputs:\n      level:\n        type: choice\n        options: [a]\n        default: a\n        options:\n          - a\n          - b\n      tag:\n        required: true\n        description: release tag\n",
        )
    }

    #[test]
    fn declared_inputs_fill_defaults_and_reject_mistakes() {
        let kv = |k: &str, v: &str| (k.to_string(), v.to_string());
        let d = declared();
        assert_eq!(
            resolve(&d, &[kv("tag", "v1")]),
            Ok(vec![kv("level", "a"), kv("tag", "v1")])
        );
        assert_eq!(
            resolve(&d, &[kv("tag", "v1"), kv("level", "b")]),
            Ok(vec![kv("level", "b"), kv("tag", "v1")])
        );
        assert!(
            resolve(&d, &[])
                .expect_err("missing")
                .contains("needs input tag=… (release tag)")
        );
        assert!(
            resolve(&d, &[kv("tag", "x"), kv("level", "z")])
                .expect_err("bad")
                .contains("one of")
        );
        assert!(
            resolve(&d, &[kv("nope", "x")])
                .expect_err("unknown")
                .contains("takes: level, tag")
        );
        // Nothing declared: pass the user's inputs on.
        assert_eq!(resolve(&[], &[kv("x", "1")]), Ok(vec![kv("x", "1")]));
    }

    #[tokio::test]
    async fn start_reads_the_workflow_file_and_sends_the_resolved_inputs() {
        use gitkraft_platforms::testing::FakeClient;
        use gitkraft_platforms::{Account, PlatformKind, RepoId, Secret};
        let fake = FakeClient::new();
        fake.push_json(
            "https://api.github.com/repos/o/r/contents/.github/workflows/ci.yml",
            r#"{"content":"b246CiAgd29ya2Zsb3dfZGlzcGF0Y2g6CiAgICBpbnB1dHM6CiAgICAgIGxldmVsOgogICAgICAgIGRlZmF1bHQ6IGluZm8KICAgICAgdGFnOgogICAgICAgIHJlcXVpcmVkOiB0cnVlCg=="}"#,
        );
        fake.push_status(
            "https://api.github.com/repos/o/r/actions/workflows/ci.yml/dispatches",
            204,
            "",
        );
        let ci = gitkraft_ci::CiBackend::new(
            &Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("t")),
            fake.clone(),
        );
        let repo = RepoId::new("github.com", "o", "r");
        let d = |inputs: &[(&str, &str)]| Dispatch {
            workflow: "ci.yml".into(),
            git_ref: "main".into(),
            inputs: inputs
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
        };
        // Required input missing: refused before anything is dispatched.
        let err = start(&ci, &repo, &d(&[])).await.expect_err("missing tag");
        assert!(err.contains("needs input tag"), "{err}");
        assert!(
            fake.requests()
                .iter()
                .all(|r| !r.url.ends_with("/dispatches"))
        );
        let ok = start(&ci, &repo, &d(&[("tag", "v1")]))
            .await
            .expect("started");
        assert_eq!(ok, "started ci.yml on main");
        let sent = fake.requests();
        let body = sent
            .iter()
            .find(|r| r.url.ends_with("/dispatches"))
            .and_then(|r| r.body.clone())
            .expect("dispatched");
        assert_eq!(
            String::from_utf8_lossy(&body),
            r#"{"inputs":{"level":"info","tag":"v1"},"ref":"main"}"#
        );
    }

    #[test]
    fn text_is_workflow_ref_and_inputs() {
        let d = |w: &str, r: &str, i: &[(&str, &str)]| Dispatch {
            workflow: w.into(),
            git_ref: r.into(),
            inputs: i.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect(),
        };
        assert_eq!(
            parse("ci.yml master level=debug dry=true", true),
            Ok(d(
                "ci.yml",
                "master",
                &[("level", "debug"), ("dry", "true")]
            ))
        );
        assert_eq!(
            parse("  release/1.x  ", false),
            Ok(d("", "release/1.x", &[]))
        );
        assert_eq!(
            parse("main FOO=bar", false),
            Ok(d("", "main", &[("FOO", "bar")]))
        );
        assert!(parse("", true).expect_err("empty").contains("workflow"));
        assert!(
            parse("ci.yml", true)
                .expect_err("no ref")
                .contains("branch or tag")
        );
        assert!(parse("", false).is_err());
        assert!(
            parse("ci.yml main novalue", true)
                .expect_err("bad")
                .contains("key=value")
        );
        assert!(parse("ci.yml main =x", true).is_err());
    }
}
