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

#[cfg(test)]
mod tests {
    use super::*;

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
