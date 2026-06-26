#!/usr/bin/env nu

# ──────────────────────────────────────────────────────────────
#  GitKraft – CI Release Notes Generator
# ──────────────────────────────────────────────────────────────
#  Called by the release workflow AFTER the tag has been pushed
#  and checked out.  Generates:
#    - CHANGELOG.md  (full history, updated in place)
#    - RELEASE_NOTES.md  (single-release body used by softprops/action-gh-release)
#
#  Key: the workflow runs at the tag commit, so ALL commits up to
#  HEAD are already tagged.  --unreleased returns nothing at that
#  point.  We use --latest to get the commits belonging to this
#  tag vs the previous one.
#
#  Usage:
#    nu scripts/ci/release_notes.nu v1.2.3
# ──────────────────────────────────────────────────────────────

def main [raw_tag: string] {
    let version = ($raw_tag | str replace --regex '^v' '')
    let tag     = $"v($version)"

    print $"(ansi cyan)═══ Release Notes — ($tag) ═══(ansi reset)"

    # ── 1. Regenerate the full CHANGELOG.md ───────────────────
    if (which git-cliff | is-not-empty) {
        print "  Regenerating CHANGELOG.md…"
        run-external "git-cliff" "--output" "CHANGELOG.md"
        print "  ✔ CHANGELOG.md updated"
    } else {
        print "  ⚠ git-cliff not found — CHANGELOG.md not updated"
    }

    # ── 2. Extract per-release notes via --latest ─────────────
    # --latest gives commits between the previous tag and this one.
    # --unreleased would return nothing because HEAD is already tagged.
    let cliff_changes = if (which git-cliff | is-not-empty) {
        let result = (do { run-external "git-cliff" "--latest" "--strip" "header" } | complete)
        if $result.exit_code == 0 and ($result.stdout | str trim | is-not-empty) {
            $result.stdout | str trim
        } else {
            "- See commit history for details."
        }
    } else {
        "- See commit history for details."
    }

    # ── 3. Build RELEASE_NOTES.md ─────────────────────────────
    let notes = [
        $"# GitKraft ($version)"
        ""
        "## Installation"
        ""
        "```sh"
        "# Desktop GUI"
        "cargo install gitkraft"
        ""
        "# Terminal UI"
        "cargo install gitkraft-tui"
        "```"
        ""
        $"Or download pre-built binaries for your platform from this release."
        ""
        "## What's Changed"
        ""
        $cliff_changes
        ""
        "## Crates"
        ""
        "| Crate | Version |"
        "|-------|---------|"
        $"| `gitkraft`      | ($version) |"
        $"| `gitkraft-tui`  | ($version) |"
        $"| `gitkraft-core` | ($version) |"
    ] | str join "\n"

    $notes | save --force RELEASE_NOTES.md
    print "  ✔ RELEASE_NOTES.md written"
    print $"(ansi green)✅ Release artifacts ready for ($tag)(ansi reset)"
}
