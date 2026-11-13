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

- Move a card between columns (GitHub `updateProjectV2ItemFieldValue`, GitLab label change).
- Edit and create wiki pages (GitLab / Gitea REST); GitHub wikis via the `.wiki.git` clone.
- Reply to a discussion comment thread; mark an answer.
- Clickable items in the calendar day panel.

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

1. **CI failure explainer.** *Done in `gitkraft-hub/src/ai.rs` and `gitkraft-tui explain RUN [JOB] [--dry-run]`; TUI/GUI/web buttons and an MCP tool are still to do.* Failing step's log tail (already located by `first_failure`) to
   cause and suggested fix. Smallest slice that proves the whole pipeline.
2. **PR summary** and "what happened in this thread while I was away".
3. **Review draft**: inline comment suggestions that land in the comment prompt for editing.
4. **Commit message / PR description** from staged changes.
5. **Inbox triage**: act now / FYI / ignore. Easiest to get wrong, so last.

Code lives in one module, `gitkraft-hub/src/ai.rs`, shared by TUI, GUI and web.

## Other ideas

- Saved searches and per-repo dashboards shared as a file.
- Stale PR / issue nudges in the notification daemon.
- Review-requested SLA: highlight PRs waiting longest on you.
- Per-repo CI flakiness view (jobs that fail then pass on re-run).
- Release and tag overview with draft release notes from merged PR titles.
