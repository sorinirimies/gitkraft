# Roadmap

Ideas not built yet, roughly in order. Anything here is a proposal, not a promise.

## Verification (biggest risk)

Everything below has only run against fake HTTP responses. Run each against a real server
and fix what differs:

- GitHub GraphQL: merge, auto-merge, merge queue, discussions, Projects v2 boards.
- GitLab: cancel auto-merge, wiki, issue boards, day breakdown.
- Gitea / Forgejo / Codeberg: Actions endpoints, wiki, activity feed.
- Woodpecker CI.
- MCP write tools.
- GUI and web hub pages, day-click in the TUI and GUI, web "Check out" button.

## Boards and wiki, next steps

Done: moving a card between columns (GitHub Status field, GitLab list labels), creating and
editing wiki pages (GitLab / Gitea REST), replying inside a discussion thread. Still open:

- GitHub wikis via the `.wiki.git` clone (they have no API).
- Mark a discussion comment as the answer.
- Reorder cards inside a column; create cards.
- Clickable items in the calendar day panel.

## Several remotes

Done: `Project` (the remotes of a checkout), `Scope`, branch state per remote, mirrored CI runs
folded by commit (`gitkraft-hub/src/project.rs`), shown in the TUI, GUI, web, CLI and MCP.
Open: push to every remote in one go (`push --all-remotes`, with a per-remote result), PR/issue
mirroring across forges, and a per-remote default branch.

## MCP

- Resources: a PR, a CI run and a log as readable resources (prompts are done; resources mostly
  repeat the tools, so only worth it for clients that attach them as context).

## AI features (last)

Principles: drafts only (the AI fills a text box, the user sends), opt-in per account and per
action, secrets redacted and size-capped before anything leaves the machine, UI never blocks.

Provider: a small `Llm` trait (fakeable, no network in tests) with two backends:

- Anthropic.
- Any OpenAI-compatible endpoint with a configurable base URL. This covers OpenAI and local
  servers (Ollama, LM Studio, vLLM), so private repos can stay on the machine.

Key: env (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`), then the OS keyring via
`gitkraft-tui ai-key anthropic|openai` (key on stdin). Never a config file. The config holds provider, model, base URL and
which features are on (today: `GITKRAFT_AI_PROVIDER`, `GITKRAFT_AI_MODEL`, `GITKRAFT_AI_BASE_URL`).

Features, in order of value:

1. **CI failure explainer.** *Done in `gitkraft-hub/src/ai.rs` and `gitkraft-tui explain RUN [JOB] [--dry-run]`; TUI key `E` done; GUI and web buttons still to do (no MCP tool: an MCP client is already a model and can read `ci_job_log`).* Failing step's log tail (already located by `first_failure`) to
   cause and suggested fix. Smallest slice that proves the whole pipeline.
2. **PR summary** and "what happened in this thread while I was away". *Done in `gitkraft-hub/src/review_ai.rs` with the per-host, per-action opt-in (`ai-policy.toml`, `gitkraft-tui ai-allow`) and CLI `summarize`, `catch-up`; TUI, GUI and web buttons still to do.*
3. **Review draft**: inline comment suggestions that land in the comment prompt for editing. *Done as CLI `review-draft`; the UI hand-off into the comment prompt is still to do.*
4. **Commit message** from staged changes, in a per-project style (`.gitkraft/ai.toml`): *done* in `gitkraft-hub/src/commit_ai.rs`, `gitkraft-tui commit-message` and `g` in the TUI staging pane; GUI button and PR description still to do.
5. **Inbox triage**: act now / FYI / ignore. Easiest to get wrong, so last. *Done as CLI `triage` (only opted-in hosts are sent; unrated items are said out loud); no UI yet.*

Code lives in one module, `gitkraft-hub/src/ai.rs`, shared by TUI, GUI and web.

## Other ideas

- Saved searches and per-repo dashboards shared as a file.
- Stale PR / issue nudges in the notification daemon.
- Review-requested SLA: highlight PRs waiting longest on you.
- Per-repo CI flakiness view (jobs that fail then pass on re-run).
- Release and tag overview with draft release notes from merged PR titles.
