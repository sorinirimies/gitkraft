//! OpenSSH public keys: parsing and fingerprints.

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use sha2::{Digest, Sha256};

/// One line of a `.pub` file or an `authorized_keys`-style entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey {
    /// `ssh-ed25519`, `ssh-rsa`, `ecdsa-sha2-nistp256`, `sk-ssh-ed25519@openssh.com`…
    pub kind: String,
    /// The base64 key blob.
    pub blob: String,
    /// The trailing comment, if any.
    pub comment: String,
}

impl PublicKey {
    /// Parses `<type> <base64> [comment]`; leading options and blank lines are not accepted.
    pub fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split_whitespace();
        let kind = parts.next()?;
        let blob = parts.next()?;
        let known =
            kind.starts_with("ssh-") || kind.starts_with("ecdsa-sha2-") || kind.starts_with("sk-");
        if !known || STANDARD.decode(blob).is_err() {
            return None;
        }
        Some(Self {
            kind: kind.to_string(),
            blob: blob.to_string(),
            comment: parts.collect::<Vec<_>>().join(" "),
        })
    }

    /// `SHA256:…`, the form `ssh-keygen -l` and every platform show.
    pub fn fingerprint(&self) -> String {
        let bytes = STANDARD.decode(&self.blob).unwrap_or_default();
        let digest = Sha256::digest(bytes);
        format!("SHA256:{}", STANDARD_NO_PAD.encode(digest))
    }

    /// `<type> <base64>`: what platforms compare, without the comment.
    pub fn bare(&self) -> String {
        format!("{} {}", self.kind, self.blob)
    }

    /// The full line with its comment.
    pub fn line(&self) -> String {
        if self.comment.is_empty() {
            self.bare()
        } else {
            format!("{} {}", self.bare(), self.comment)
        }
    }
}

/// A `.pub` file found on disk next to its private key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalKey {
    /// The private key path (the `.pub` path without its extension).
    pub path: std::path::PathBuf,
    /// The public key.
    pub public: PublicKey,
}

/// Every parseable `*.pub` in `dir`, sorted by path.
pub fn local_keys(dir: &std::path::Path) -> Vec<LocalKey> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut keys: Vec<LocalKey> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "pub"))
        .filter_map(|p| {
            let text = std::fs::read_to_string(&p).ok()?;
            let public = PublicKey::parse(text.lines().next()?)?;
            Some(LocalKey {
                path: p.with_extension(""),
                public,
            })
        })
        .collect();
    keys.sort_by(|a, b| a.path.cmp(&b.path));
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    // A well-formed ed25519 line (made up; the tests only need valid base64).
    const ED25519: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJ0r5GmJcQGHlZ4gkUXZv9m+i1m8c4pQXnTQ4v8nY2ka me@laptop";

    #[test]
    fn parses_type_blob_and_comment() {
        let k = PublicKey::parse(ED25519).expect("key");
        assert_eq!(
            (k.kind.as_str(), k.comment.as_str()),
            ("ssh-ed25519", "me@laptop")
        );
        assert_eq!(
            k.bare(),
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJ0r5GmJcQGHlZ4gkUXZv9m+i1m8c4pQXnTQ4v8nY2ka"
        );
        assert_eq!(k.line(), ED25519);
    }

    #[test]
    fn the_fingerprint_is_sha256_of_the_blob_in_unpadded_base64() {
        let k = PublicKey::parse(ED25519).expect("key");
        let fp = k.fingerprint();
        assert!(fp.starts_with("SHA256:") && fp.len() == 7 + 43, "{fp}");
        assert!(!fp.ends_with('='));
        assert_eq!(
            fp,
            PublicKey::parse(&format!("{} other comment", k.bare()))
                .expect("key")
                .fingerprint()
        );
    }

    #[test]
    fn rejects_garbage_private_keys_and_bad_base64() {
        assert!(PublicKey::parse("").is_none());
        assert!(PublicKey::parse("hello world").is_none());
        assert!(PublicKey::parse("-----BEGIN OPENSSH PRIVATE KEY-----").is_none());
        assert!(PublicKey::parse("ssh-ed25519 !!!notbase64").is_none());
        assert!(PublicKey::parse("ssh-ed25519").is_none());
    }

    #[test]
    fn finds_pub_files_and_skips_the_rest() {
        let dir = tempfile::tempdir().expect("tmp");
        std::fs::write(dir.path().join("id_ed25519.pub"), format!("{ED25519}\n")).expect("w");
        std::fs::write(dir.path().join("id_ed25519"), "PRIVATE").expect("w");
        std::fs::write(dir.path().join("broken.pub"), "nonsense").expect("w");
        std::fs::write(dir.path().join("config"), "Host *").expect("w");
        let keys = local_keys(dir.path());
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].path, dir.path().join("id_ed25519"));
        assert!(local_keys(&dir.path().join("missing")).is_empty());
    }
}
