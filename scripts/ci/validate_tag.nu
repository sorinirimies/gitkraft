#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# GitKraft — Validate release tag
# ──────────────────────────────────────────────────────────────────────────────
# Validates that a tag matches the vX.Y.Z pattern and outputs the tag and the
# bare version string.  Used by CI release workflows so validation logic lives
# in one place rather than being duplicated across GitHub / Gitea YAML.
#
# Usage:
#   nu scripts/ci/validate_tag.nu <tag>
#
# Examples:
#   nu scripts/ci/validate_tag.nu v0.5.0
#   nu scripts/ci/validate_tag.nu v1.2.3
#
# On success, prints the tag and version and exits 0.
# On failure, prints an error and exits 1.
#
# When running inside GitHub / Gitea Actions the caller should capture the
# output and write it to $GITHUB_OUTPUT:
#
#   RESULT=$(nu scripts/ci/validate_tag.nu "$TAG")
#   echo "$RESULT" >> "$GITHUB_OUTPUT"
# ──────────────────────────────────────────────────────────────────────────────

# Validate a tag string and return the tag + bare version.
export def validate [tag: string]: nothing -> record<tag: string, version: string> {
    if ($tag | is-empty) {
        print $"(ansi red)❌ Tag is empty — nothing to validate.(ansi reset)"
        exit 1
    }

    let pattern = '^v\d+\.\d+\.\d+$'
    if ($tag | find --regex $pattern | is-empty) {
        print $"(ansi red)❌ Tag '($tag)' does not match vX.Y.Z — aborting.(ansi reset)"
        exit 1
    }

    let version = ($tag | str replace 'v' '')

    print $"(ansi green)✅ Tag ($tag) \(version ($version)\) is valid.(ansi reset)"

    { tag: $tag, version: $version }
}

# ── Main ──────────────────────────────────────────────────────────────────────

def main [tag: string] {
    let result = (validate $tag)

    # When running in CI, emit lines compatible with $GITHUB_OUTPUT.
    # The caller can append stdout directly:
    #   nu scripts/ci/validate_tag.nu "$TAG" >> "$GITHUB_OUTPUT"
    print $"tag=($result.tag)"
    print $"version=($result.version)"
}
