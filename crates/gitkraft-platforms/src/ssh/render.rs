//! Plain-text lines about setup steps and connection tests, shared by every front-end.

use super::setup::{Report, Step};
use super::tools::Probe;

/// Text of a finished setup.
pub fn render_report(r: &Report, host: &str) -> String {
    let mut out = String::new();
    for s in &r.steps {
        let line = match s {
            Step::KeyCreated(p) => format!("✔ created key {}", p.display()),
            Step::KeyReused(p) => format!("✔ using key {}", p.display()),
            Step::Uploaded { id } => format!("✔ registered on {host} (key id {id})"),
            Step::AlreadyRegistered { title } => {
                format!("✔ already registered on {host} as “{title}”")
            }
            Step::ConfigBackedUp(p) => format!("✔ saved your old config as {}", p.display()),
            Step::ConfigWritten(p) => format!("✔ {} points {host} at the key", p.display()),
            Step::AgentLoaded => "✔ key is in the ssh-agent".into(),
            Step::AgentSkipped(why) => format!(
                "! not loaded into the agent ({why}); run `ssh-add {}`",
                r.key_path.display()
            ),
            Step::Probed(p) => describe_probe(p, host),
        };
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!("fingerprint {}\n", r.fingerprint));
    out
}

/// One line about a connection test.
pub fn describe_probe(p: &Probe, host: &str) -> String {
    match p {
        Probe::Authenticated { user: Some(u) } => format!("✔ ssh to {host} works (as {u})"),
        Probe::Authenticated { user: None } => format!("✔ ssh to {host} works"),
        Probe::Denied => format!("✘ {host} refused the key (permission denied)"),
        Probe::HostKey => format!(
            "✘ {host} is not a trusted host yet: run `ssh -T git@{host}` once and compare the fingerprint with the platform's published one"
        ),
        Probe::Unreachable => format!("✘ cannot reach {host}"),
        Probe::Other(t) => format!("? {host} said: {t}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendering_a_report_covers_every_step() {
        let r = Report {
            steps: vec![
                Step::KeyCreated("/k".into()),
                Step::Uploaded { id: "3".into() },
                Step::ConfigBackedUp("/c.bak".into()),
                Step::ConfigWritten("/c".into()),
                Step::AgentSkipped("no agent".into()),
                Step::Probed(Probe::Denied),
            ],
            key_path: "/k".into(),
            fingerprint: "SHA256:x".into(),
        };
        let t = render_report(&r, "github.com");
        for want in [
            "created key /k",
            "registered on github.com (key id 3)",
            "saved your old config",
            "points github.com",
            "ssh-add /k",
            "refused the key",
            "fingerprint SHA256:x",
        ] {
            assert!(t.contains(want), "{want} in {t}");
        }
    }

    #[test]
    fn probes_read_well() {
        assert!(describe_probe(&Probe::HostKey, "h").contains("ssh -T git@h"));
        assert!(
            describe_probe(
                &Probe::Authenticated {
                    user: Some("me".into())
                },
                "h"
            )
            .contains("as me")
        );
        assert!(describe_probe(&Probe::Unreachable, "h").contains("cannot reach"));
        assert!(describe_probe(&Probe::Other("x".into()), "h").starts_with('?'));
    }
}
