# Changelog

All notable changes to this project are documented here. It is generated from
[Conventional Commits](https://www.conventionalcommits.org) by
[git-cliff](https://git-cliff.org) — do not edit it by hand.

## [1.6.0](https://github.com/sorinirimies/gitkraft/compare/v1.5.0...v1.6.0) — 2026-11-20

### ✨ Features

- **web:** Repository page (history graph, changes, branches, stashes, remotes); feat: Dodai gradient mesh in web and GUI; feat(tui): Dodai themes; fresh installs start on Dodai Dark; Dodai families lose the 'Dodai ·' prefix ([`0069762`](https://github.com/sorinirimies/gitkraft/commit/0069762f48f07765abf71220813ff4c284d10011))
- Log search in the GUI and web, label editing in the web; fix(gui): one notification poll at a time ([`1a77b75`](https://github.com/sorinirimies/gitkraft/commit/1a77b75122e9784c819e3c15ea5126b304533c25))
- Chamfered Dodai panels in the GUI (dodai-iced), and gitkraft-web --demo with made-up data and screenshots ([`5254d39`](https://github.com/sorinirimies/gitkraft/commit/5254d392442d3f5baa12cec1b4eb58b9270e3b86))
- **gui:** Remaining radii follow the Dodai shape; docs: 105 themes ([`39b63f8`](https://github.com/sorinirimies/gitkraft/commit/39b63f89d5fe54d1f84537cb85c9d3ac1d4ea266))
- Dodai shape in the GUI, glow at 0.15, and self-hosted forges on http or another port (accounts.toml url) ([`0dd9d2d`](https://github.com/sorinirimies/gitkraft/commit/0dd9d2dfebd88057f42e71da05cfbb9542900b2c))
- **web:** Dodai design system themes with a live theme picker; themed calendar; shape, glow and fonts from dodai-topcoat ([`6accf3c`](https://github.com/sorinirimies/gitkraft/commit/6accf3cfe128ce6484c0ddf634c1952e3c530c5e))
- AppImage and Flatpak login entry, per-distro tray hints incl. GNOME on Arch and COSMIC ([`17a0f28`](https://github.com/sorinirimies/gitkraft/commit/17a0f28d51a8dc81fb522af8bc13e6c32fbb0ce8))
- Tray icon and menu, start at login, and one-instance agent schedule lease ([`f404847`](https://github.com/sorinirimies/gitkraft/commit/f40484731f1de9378f44137afda11b466a526c09))
- Open pull requests, issues, CI runs and notifications in the browser (TUI b, GUI button, web link) ([`39f6100`](https://github.com/sorinirimies/gitkraft/commit/39f6100f08abc7987cf36b6582a9d5b9f0edcf90))

### 🐛 Bug fixes

- **demo:** A calendar day in the demo opens with what was done that day (the fake GraphQL answered every query with the calendar) ([`04918ae`](https://github.com/sorinirimies/gitkraft/commit/04918aedf0dedc02e6a70d03e3686f93059440b7))
- **tray:** Use as_chunks for RGBA→ARGB (clippy chunks_exact_to_as_chunks) ([`652577f`](https://github.com/sorinirimies/gitkraft/commit/652577fac21df302923abd089a87240861f0aedc))
- **gui:** Drop an unreachable match arm ([`8b03fb5`](https://github.com/sorinirimies/gitkraft/commit/8b03fb594340328cd4d372b9cf4f9ef30aa72b3d))

### ⚡ Performance

- **gui:** Draw only the visible lines of a CI log, cap file preview at 5 MB; perf(web): lazy-render log lines ([`eef8979`](https://github.com/sorinirimies/gitkraft/commit/eef8979d176d14255606025f8c65744fc68cb527))
- Drain terminal events before drawing, system theme read off the UI thread, async git watcher and ticks in the GUI, allocation-free theme lookup ([`cda35e8`](https://github.com/sorinirimies/gitkraft/commit/cda35e8bc2cf1c2b9f65f6bc3985153cdb0645bc))

### 🧪 Tests

- **gui:** Pin the theme instead of reading the saved settings ([`8d386f7`](https://github.com/sorinirimies/gitkraft/commit/8d386f74efef6e9b01e29493b46759e1784b04ff))

### 🔧 Chores

- **flatpak:** 25.08 runtime, richer AppStream metadata and 1.6.0 release entry, submission notes ([`2bfe2c8`](https://github.com/sorinirimies/gitkraft/commit/2bfe2c800ad7101512f1bbf114f32580d0251299))
- **packaging:** Tray permissions for Flatpak and Snap, sandbox-safe tray name, AppStream keywords ([`0fafadf`](https://github.com/sorinirimies/gitkraft/commit/0fafadf91f60ddec297f52a680888e7629530d17))
- **deny:** Ignore the chrono unmaintained notice (RUSTSEC-2026-0543) ([`5283235`](https://github.com/sorinirimies/gitkraft/commit/52832355fed8c2640d169b2d53acf7621e18d9b9))

## [1.5.0](https://github.com/sorinirimies/gitkraft/compare/v1.4.1...v1.5.0) — 2026-11-18

### ✨ Features

- **tui:** Header tabs and hints, theme list and settings menu react to clicks ([`ce18007`](https://github.com/sorinirimies/gitkraft/commit/ce180077749f5012b585f58d4ef9432493062965))
- **tui:** Mouse on the main screen (click, double-click, wheel) and right-click action menus; editor picker is clickable ([`24def66`](https://github.com/sorinirimies/gitkraft/commit/24def66ccfe99e19149118cc02a6c15ec5ae5c43))
- **tui:** Right-click a row in the hub for an actions menu; palette and menus are clickable ([`4d23ed2`](https://github.com/sorinirimies/gitkraft/commit/4d23ed29774014b5908a1c25009a764938b31abb))
- **tui:** Editor picker with 22 editors, custom command, installed/terminal tags, reachable from settings and the hub ([`8176fde`](https://github.com/sorinirimies/gitkraft/commit/8176fde57f6ac6d685f9a0c45a0ec34b1d594eb8))
- **tui:** Shift+A toggles the Actions pane (? still works) ([`f5d9a47`](https://github.com/sorinirimies/gitkraft/commit/f5d9a47cefdc1a95f95c75455c845f744f4b7a98))
- **tui:** Actions pane hidden until ?; About becomes Info on Shift+I ([`3c4e014`](https://github.com/sorinirimies/gitkraft/commit/3c4e0149d063397c79edfafad3795fd26926d997))
- **tui:** Quiet hub status line; ? opens an actions menu for the current screen ([`ea469ae`](https://github.com/sorinirimies/gitkraft/commit/ea469aea1b7376372cf98251707d05d33455c5ec))
- Agents CLI (reports, --nth, switches both ways, --clear-globs) and MCP agent tools; AgentsState::in_dirs loads; docs ([`f68bed9`](https://github.com/sorinirimies/gitkraft/commit/f68bed9566eeb17b63dbb2a461ef02d6e143b64b))
- Report history in the GUI and web; tests for the unread badge and a web run ([`aa145dc`](https://github.com/sorinirimies/gitkraft/commit/aa145dc40a63ebdd55e7adba1d5846a45b589356))
- Edit an agent job's instructions and globs in the TUI, GUI and web ([`192a112`](https://github.com/sorinirimies/gitkraft/commit/192a112695caeb4c7aff4ba07f59236a9b94c2fa))
- Agents view in the GUI and web with shared state and actions; in-app scheduler in all three ([`98cc9ac`](https://github.com/sorinirimies/gitkraft/commit/98cc9ace8427cccc468580c5f196d6e4e47e20ca))
- Agents view in the TUI hub: jobs, repository picker, switches, findings chart, last report, in-app scheduler (tui-checkbox, tui-chart, tui-skeleton, tui-spinner) ([`6d7d4e8`](https://github.com/sorinirimies/gitkraft/commit/6d7d4e8e97b37604b63b5d13edbf67de4ddb0f5c))
- Agent-watch core and CLI: scheduled CI checks over any forge, project selector, pluggable agent harness, reports, inbox and system notifications ([`6225920`](https://github.com/sorinirimies/gitkraft/commit/62259202747a92b51523d3a910db3873dd2f74c6))
- Push the current branch to every forge remote with one result each (CLI push-all, MCP push_all_remotes) ([`8e4f8fa`](https://github.com/sorinirimies/gitkraft/commit/8e4f8faa92207e0de129dfa9d9c531cfab98c80e))
- GitHub wikis through their .wiki.git repository: list, read and save in every client ([`fd41f10`](https://github.com/sorinirimies/gitkraft/commit/fd41f101a90a894ae462c11425e4fb9abb970781))
- Create and reorder board cards, clickable calendar-day items in every client ([`6a3dcf2`](https://github.com/sorinirimies/gitkraft/commit/6a3dcf23eeb0e6083143efb9ab64e90371f8a522))
- Answer buttons in GUI and web, review drafts land in the comment box, catch-up takes a start date everywhere ([`a3405c2`](https://github.com/sorinirimies/gitkraft/commit/a3405c247d34145314fcc3eb2bec3127385c0dc2))
- Mark a discussion comment as the answer (platform, hub, CLI, MCP, TUI, web action) ([`e7013ff`](https://github.com/sorinirimies/gitkraft/commit/e7013ff5a868e9ad75202297a0eb16994c4733b8))
- Ssh status follows the user's own ssh config; settings menu and AI entries in the hub palette ([`19d90a2`](https://github.com/sorinirimies/gitkraft/commit/19d90a23c696a43ed02a3be8aad6e3ba71739ede))
- **tui,gui:** AI inbox triage behind the host opt-in ([`b926d99`](https://github.com/sorinirimies/gitkraft/commit/b926d99cc7579a55ca3f0087a69623db7a1cecc5))
- **tui,gui:** AI PR summary, catch-up and review notes behind the host opt-in ([`74c3d1e`](https://github.com/sorinirimies/gitkraft/commit/74c3d1e97d00bbcae310d2f8500b0c34b247f93c))
- **web:** AI summary, catch-up, review draft and inbox triage buttons behind the host opt-in; shared triage report in the hub ([`0d6121d`](https://github.com/sorinirimies/gitkraft/commit/0d6121d8fea855a9ae097df9a385d5217414aa38))
- **cli:** AI summarize, catch-up, review-draft and triage behind a per-host, per-action opt-in (ai-allow) ([`640bddb`](https://github.com/sorinirimies/gitkraft/commit/640bddb408332f29493215ebf04aca6f29ba7deb))
- Several forge remotes per project: remote switcher, branch state per remote, mirrored CI runs, remotes CLI and MCP tools in every client ([`f6256a4`](https://github.com/sorinirimies/gitkraft/commit/f6256a4f1bf52a27b418a8d28e6c541c6fdb21a3))
- **hub:** AI drafts for pull requests and the inbox with a per-host opt-in policy ([`1d84a7a`](https://github.com/sorinirimies/gitkraft/commit/1d84a7a81b247d9442f53af922d11f59c071d5a7))
- Discussion thread replies, wiki create/edit and board card moves across platforms, TUI, GUI, web, CLI and MCP; shared argument parsing in the hub ([`7ade990`](https://github.com/sorinirimies/gitkraft/commit/7ade99028e0125db6e33b2dc61b02bb3768beb73))
- Client parity for AI explain and commit draft (GUI, web), web artifacts download and workflow dispatch, shared hub helpers ([`d004a20`](https://github.com/sorinirimies/gitkraft/commit/d004a20f0921bbee5a2860cb79c3a67d0376ec35))
- **tui:** Animated settings menu with eased slide, theme, reduced motion and commit style rows ([`2708322`](https://github.com/sorinirimies/gitkraft/commit/2708322bb5fdaf55a8e305a4c41915c54282dade))
- **ai:** Commit messages in a per-project style (.gitkraft/ai.toml), TUI draft key and CI explain key ([`df6d012`](https://github.com/sorinirimies/gitkraft/commit/df6d012dfaddc6f48ded1b8bbe02dae619bc3ddb))
- **ai:** CI failure explainer with redaction, Anthropic and OpenAI-compatible backends, explain and ai-key commands ([`52a6ba9`](https://github.com/sorinirimies/gitkraft/commit/52a6ba9a93381853ccfd1cd1887f2cb80de24573))
- **tui:** Clap command line with scriptable read/write commands, --json and shell completions ([`c1d19c2`](https://github.com/sorinirimies/gitkraft/commit/c1d19c2bef1ea32f7215f121c9e4cdced1e7a6da))
- **mcp:** Prompts for failing CI, review queue, PR review, catch-up and triage ([`51e3438`](https://github.com/sorinirimies/gitkraft/commit/51e34384d98a94069f5ea533b032f3d67a375829))
- Show when a PR branch is behind its target; add docs/ROADMAP.md ([`757dccf`](https://github.com/sorinirimies/gitkraft/commit/757dccffbd7c4fcbc38696367aaa003703d383e9))
- Boards (GitHub Projects v2, GitLab issue boards) in every front-end and the MCP server ([`dd275a5`](https://github.com/sorinirimies/gitkraft/commit/dd275a596958351cfcf3dfdb1913056baeefa688))
- **gui:** Discussions and wiki tab ([`f0de12e`](https://github.com/sorinirimies/gitkraft/commit/f0de12e3223f4e7f755acf237cf31dbe0dabd32c))
- **tui:** Discussions and wiki view; shared community lists in the hub ([`b6478b8`](https://github.com/sorinirimies/gitkraft/commit/b6478b8ec15fa91404f2a40a9314dd0df79489da))
- **web:** Discussions and wiki page ([`db3d19c`](https://github.com/sorinirimies/gitkraft/commit/db3d19c729bd9997b4bf112486b7187145b5747f))
- **platforms:** Wikis (GitLab, Gitea family) and GitHub discussions, with MCP tools ([`7314f35`](https://github.com/sorinirimies/gitkraft/commit/7314f359c3945f3a8bf384a47166ef0ef0bd8dde))
- **hub:** Click a day of the contribution calendar to see what you did, like GitHub ([`b01194f`](https://github.com/sorinirimies/gitkraft/commit/b01194f8e3746592c08ad79f948dc1a2510f6391))
- **web:** Command palette and PR worktree checkout ([`ad6f48c`](https://github.com/sorinirimies/gitkraft/commit/ad6f48c296c31db8c1807504dcd645e41e68a4cb))
- **mcp:** Gitkraft-mcp, an MCP server for PRs, issues and CI of every forge; read-only unless --write ([`40c2aa6`](https://github.com/sorinirimies/gitkraft/commit/40c2aa6b6e4322fed9b901d904144a2d9ca7222d))
- **hub:** Update branch, auto-merge, merge queue, force merge and cancel across GitHub, GitLab and Gitea in TUI, GUI and web ([`a9ddbbc`](https://github.com/sorinirimies/gitkraft/commit/a9ddbbca58872916bf371fecb524625ed9556416))
- **hub:** Edit labels of a PR or issue (GitHub, Gitea, GitLab) from the TUI and GUI ([`7bee16a`](https://github.com/sorinirimies/gitkraft/commit/7bee16afb669f0d978624d8f18ea23303a9f71fe))
- Gitkraft-tui clone over the chosen transport (clone_via is now used) ([`a3e5616`](https://github.com/sorinirimies/gitkraft/commit/a3e561634c328579da1e63f19e12acc3d9cd76f0))
- **ci:** Woodpecker CI adapter (runs, steps, logs, restart, cancel, dispatch) for Codeberg and others ([`a2818dc`](https://github.com/sorinirimies/gitkraft/commit/a2818dca8466a4a67a907e491ee53d14b2d548d8))
- **hub:** Workflow inputs read from the file; inline comments on removed lines, in the GUI and the TUI palette ([`db39dd4`](https://github.com/sorinirimies/gitkraft/commit/db39dd464f31075cd095da8863d2d3ca5e229803))
- **web:** SSH keys page; per-run token on every action (CSRF); share ssh_dir and machine_name in hub ([`5b1b0ac`](https://github.com/sorinirimies/gitkraft/commit/5b1b0ac3f89bb031fb5aff6acb4fc366ce082612))
- **gui:** Command palette and PR worktree checkout; share fuzzy match and checkout in the hub crate ([`1dafe49`](https://github.com/sorinirimies/gitkraft/commit/1dafe49a4fbcac9ef0373c44751a5bb62bd2235e))
- **hub:** Inline review comments from a diff cursor (GitHub, Gitea, GitLab) ([`d3e2d6f`](https://github.com/sorinirimies/gitkraft/commit/d3e2d6f6004e1bf8e9c1506c9d22022ff032d21f))
- **ci:** List and download artifacts (GitHub, Gitea, GitLab); keys in TUI and GUI ([`771f2a2`](https://github.com/sorinirimies/gitkraft/commit/771f2a22ff832f2c3a31405d2b12bf3ba1bcbede))
- Login with OAuth device flow or token into the system keyring; PR worktree checkout; run workflow by hand; command palette; docs ([`d806aad`](https://github.com/sorinirimies/gitkraft/commit/d806aad04c0b67fcc57412bff86fb1a50238ad58))
- **ssh:** Setup, key list/revoke, ~/.ssh/config block, probe, ssh/https switch; keys screens in TUI and GUI; git credential token lookup ([`48d90f0`](https://github.com/sorinirimies/gitkraft/commit/48d90f0a7cdaba55616f933d08809a6c42fe94cf))
- **gui:** Hub screen with sections, drag to reorder, PR/issue/CI detail, log viewer, pulse and contribution calendar, notification bell ([`fa2f96d`](https://github.com/sorinirimies/gitkraft/commit/fa2f96d0795ae00febb4803c7066df3aff0323f2))
- **web:** Gitkraft-web on localhost with dashboard, PR/issue/CI pages, log viewer, pulse calendar, notifications and JSON actions ([`5d3d019`](https://github.com/sorinirimies/gitkraft/commit/5d3d019d645751c0c5d21dc72def09f0efe84846))
- **tui:** Image preview, markdown rendering in PR and issue bodies ([`09cf808`](https://github.com/sorinirimies/gitkraft/commit/09cf8086fea0173d781ee69ad21b49e05a1f79ce))
- **tui:** Hub screen with dashboard sections, PR/issue/CI detail, log viewer, pulse, bell, mouse and drag; notify CLI ([`44a25eb`](https://github.com/sorinirimies/gitkraft/commit/44a25eb2be45f383f13bf35a09130f5101b28e00))
- **hub:** Notification poller cycle with seed, intervals, CI alerts, snooze and per-project failure ([`ac50a56`](https://github.com/sorinirimies/gitkraft/commit/ac50a5692e396c7fac1433afcb938e9ad6e8e74e))
- **hub:** Accounts, sections to data, PR detail, pulse, merged contributions ([`b55051b`](https://github.com/sorinirimies/gitkraft/commit/b55051bcbb7b94a00a0f34b502c3c82af75122a5))
- **platforms,settings:** Cross-repo search by involvement; dashboard sections; config file paths ([`b3b19f7`](https://github.com/sorinirimies/gitkraft/commit/b3b19f7141719047c24466d2334dfaa53d331ddd))
- **ci:** Runs, jobs, steps and logs for GitHub/Gitea/GitLab; log parser, annotations, timing, stats, events ([`3dfcf65`](https://github.com/sorinirimies/gitkraft/commit/3dfcf65d2ccf41e964e6383077599da50dc0c696))
- **platforms:** Notification poller, alerts, inbox, delivery rules, desktop notifier ([`d37d03b`](https://github.com/sorinirimies/gitkraft/commit/d37d03b0c6ef0b7a7f6cc709d2e69a2d2167eedd))
- **platforms:** GitHub, GitLab and Gitea adapters behind one model; per-project notify config ([`3d3f40a`](https://github.com/sorinirimies/gitkraft/commit/3d3f40adf17fc70a083acdb4cdcce55f994b348c))

### 🐛 Bug fixes

- **hub:** An engine save keeps inbox entries other processes filed since it loaded ([`4a02012`](https://github.com/sorinirimies/gitkraft/commit/4a020129e84736b8ebb4ef71cfeb0a2e35b73f34))
- Refuse an empty agent job name; delete job and unread badge in GUI and web; README ([`be1d720`](https://github.com/sorinirimies/gitkraft/commit/be1d72092053bc2cb6251b4665ebb4142c1afa1d))
- **ssh:** Probe with the login and port of the remote URLs instead of always git@host ([`2e57b38`](https://github.com/sorinirimies/gitkraft/commit/2e57b38a356fb78e51070005f3d73015886b4fd7))
- **github:** Job logs (415), discussion detail query, and checks of Actions-only commits, found against the real API ([`a1304c4`](https://github.com/sorinirimies/gitkraft/commit/a1304c4de0b8244792c22895af3c9cab0148748d))
- **core:** Fetch and push through the system git so ssh keys and credential helpers work; explain ssh failures ([`766e06b`](https://github.com/sorinirimies/gitkraft/commit/766e06bd87470a870698f993b0754d5c4c934229))
- **hub:** Clippy ([`c8c6f8a`](https://github.com/sorinirimies/gitkraft/commit/c8c6f8aaa33a1d777126a4f617f7fe983dfcb5f3))
- **platforms:** Drop unused import ([`fc1e0a9`](https://github.com/sorinirimies/gitkraft/commit/fc1e0a903211da6072c35f100a80cbf6e42b5ef7))

### ♻️ Refactor

- Extract settings and theme into gitkraft-settings ([`ddf5e1e`](https://github.com/sorinirimies/gitkraft/commit/ddf5e1ed8060a18cebe3a82466999c3887645077))

### 📚 Documentation

- Agent-watch ran against a live pi ([`040dd3c`](https://github.com/sorinirimies/gitkraft/commit/040dd3c644ba8abad8cafe95c69c13a8aada2540))
- **core:** Describe settings crate and the system-git transport exception ([`f19e208`](https://github.com/sorinirimies/gitkraft/commit/f19e20890de505f6c58602cfe1fb4fd877205521))
- The hub, accounts, dashboard and notification config ([`7eecf1e`](https://github.com/sorinirimies/gitkraft/commit/7eecf1e110e78e911755d68418647947c710e4de))

### 🧪 Tests

- **core:** Editor install check must not rely on /bin/sh (fails on Windows) ([`cb88aa5`](https://github.com/sorinirimies/gitkraft/commit/cb88aa5e3a5828ac7ab29c79e44fd726b45e924e))

### 📦 Dependencies

- Nightly cargo update 2026-11-18 ([`3e6cadf`](https://github.com/sorinirimies/gitkraft/commit/3e6cadf211de558cd47d1e090a7362a19402c84d))
- Nightly cargo update 2026-11-17 ([`82e2abb`](https://github.com/sorinirimies/gitkraft/commit/82e2abbc90ad4df2469a1d1d62ad1ead9b325838))
- Nightly cargo update 2026-11-16 ([`d36dfce`](https://github.com/sorinirimies/gitkraft/commit/d36dfce73cdb09750420148094631b4cba687e08))
- Nightly cargo update 2026-11-15 ([`98a7de2`](https://github.com/sorinirimies/gitkraft/commit/98a7de286a0bcfec81ae757636266a507ac8b82b))
- Nightly cargo update 2026-11-14 ([`a7b2a21`](https://github.com/sorinirimies/gitkraft/commit/a7b2a21ce2e20fcb2e99bcd8670eae4b1a3eebfa))
- Drop ratatui-image's image-defaults (pulled rav1e and an NCSA licence) ([`f60e582`](https://github.com/sorinirimies/gitkraft/commit/f60e582fa7322551448c1ba093350acd798535fb))
- Nightly cargo update 2026-11-13 ([`fef7609`](https://github.com/sorinirimies/gitkraft/commit/fef76098f3acdf3a9542484225db8f943e407f46))
- Nightly cargo update 2026-11-12 ([`5914e29`](https://github.com/sorinirimies/gitkraft/commit/5914e29c1f014ec92537beb2ca4d87b8fa5529a6))
- Nightly cargo update 2026-11-11 ([`91959ff`](https://github.com/sorinirimies/gitkraft/commit/91959ff8d8faa3c13034ee7c4679e9ea4fcb08da))

### 🔄 CI and release

- Gitleaks ignores the fake secrets in the redaction tests ([`aac7d13`](https://github.com/sorinirimies/gitkraft/commit/aac7d134de570a499324fa41a388080ff0faab6c))
- Bump gitea MSRV to 1.91 ([`00cf091`](https://github.com/sorinirimies/gitkraft/commit/00cf091c47bcc7fbbef513386b0fc4ee8fbdee6a))
- Raise MSRV to 1.91 (notify-types 2.3.0 requires it) ([`536bf08`](https://github.com/sorinirimies/gitkraft/commit/536bf0859b5f42d8520ed442554b761889133cca))

### 🔧 Chores

- **deny:** Allow CDLA-Permissive-2.0 (webpki-root-certs, via reqwest's rustls verifier) ([`107e9ca`](https://github.com/sorinirimies/gitkraft/commit/107e9ca764198189eb1114ff4dbe6c3656046113))

## [1.4.1](https://github.com/sorinirimies/gitkraft/compare/v1.4.0...v1.4.1) — 2026-11-10

### 🐛 Bug fixes

- **tui:** De-flake git_state_changed refresh test (refresh thread can finish before assert) ([`2a5e336`](https://github.com/sorinirimies/gitkraft/commit/2a5e33617480d1ce8a08a494e092397c53e6c8d1))

### 📦 Dependencies

- Bump tui-nuances to 0.1.11 ([`f682566`](https://github.com/sorinirimies/gitkraft/commit/f6825663ddb361efb82953962f0b33501db635f7))
- Nightly cargo update 2026-11-10 ([`d2b8472`](https://github.com/sorinirimies/gitkraft/commit/d2b8472a6b5a370c1450efb42a659fea4ac8a788))
- Nightly cargo update 2026-11-09 ([`eed33a2`](https://github.com/sorinirimies/gitkraft/commit/eed33a2e03524174665842814a757c4aacea22ea))
- Nightly cargo update 2026-11-08 ([`2aa8e10`](https://github.com/sorinirimies/gitkraft/commit/2aa8e10ce9971560f11a1018e9a6f23b35852960))

### 🔄 CI and release

- Add Forgejo Actions workflows for Codeberg runners ([`f034149`](https://github.com/sorinirimies/gitkraft/commit/f034149b299fd25d8769b7f9fafce6ffe9c04887))

## [1.4.0](https://github.com/sorinirimies/gitkraft/compare/v1.3.0...v1.4.0) — 2026-11-07

### ✨ Features

- New app icon for macOS, Windows and Linux ([`34a5165`](https://github.com/sorinirimies/gitkraft/commit/34a5165f5909680bdb03562fa5826deaf65c18e4))

### 🐛 Bug fixes

- **watcher:** An event on the watched .git itself is not a change (macOS delivered its creation late) ([`c5da1c0`](https://github.com/sorinirimies/gitkraft/commit/c5da1c08ac814e8ea25e8c5d9609f6f489118575))

### 🧪 Tests

- Normalize CRLF in stash_pop assertion ([`5fb7c67`](https://github.com/sorinirimies/gitkraft/commit/5fb7c67e587af61514a3f8e45f35022510e5148c))
- Normalize CRLF in restore tests for Windows ([`97bdc17`](https://github.com/sorinirimies/gitkraft/commit/97bdc1769a326c19084d373945adb4573dd2b6a2))
- **watcher:** Tolerate macOS FSEvents replay of setup write ([`9f93188`](https://github.com/sorinirimies/gitkraft/commit/9f931885ce80ebf7be4fd69dcb0fabb063a2de55))

### 📦 Dependencies

- Nightly cargo update 2026-11-07 ([`8706965`](https://github.com/sorinirimies/gitkraft/commit/870696503269b16630fe1f6229ed56d2c4bd39ed))
- Nightly cargo update 2026-11-06 ([`54ce31e`](https://github.com/sorinirimies/gitkraft/commit/54ce31ee4ae3e82d5b3b7531b1f02dbf666aa7ce))

### 🔄 CI and release

- **gitea:** Check the MSRV on Gitea too, with the same command as GitHub ([`36c05d7`](https://github.com/sorinirimies/gitkraft/commit/36c05d7df00ac48034e6fb9c329ae2d583c7db6c))
- **release:** Each install line in the release notes says which binary it is (GUI, TUI, web) ([`1d14a2f`](https://github.com/sorinirimies/gitkraft/commit/1d14a2f9dfe92292de9b44b4df3ac9d91b9157a7))
- **packages:** .deb and .rpm package names are valid Debian names (no underscores) ([`4487cb3`](https://github.com/sorinirimies/gitkraft/commit/4487cb332788f57095ee8514ae0e188943db8fe8))
- **packages:** AppImages use the downloaded appimagetool; which also matched the script's own command ([`0c14472`](https://github.com/sorinirimies/gitkraft/commit/0c144720ba2089b1814b0afdce77cc2f61ec55c0))
- **packages:** Alien gets an absolute .deb path, so the .rpm build finds it ([`937ba14`](https://github.com/sorinirimies/gitkraft/commit/937ba1446cef753e78b24dfbe23f20a868ff6f52))

## [1.3.0](https://github.com/sorinirimies/gitkraft/compare/v1.2.15...v1.3.0) — 2026-11-05

### 🐛 Bug fixes

- **ci:** The nightly downgrade guard compares within a semver series ([`096432e`](https://github.com/sorinirimies/gitkraft/commit/096432ebb1aa580331a178fa80f2a7c5657c6ecd))
- **gui,tui:** Name the window on Wayland and X11 (app_id = desktop id); TUI clipboard works on Wayland without XWayland ([`7dbd2ae`](https://github.com/sorinirimies/gitkraft/commit/7dbd2ae8607867b44b7505dfc5ec1eb6ddbd8e5f))

### ♻️ Refactor

- Edition 2024, MSRV 1.90 and the shared lints; Debug on every public type; one libgit2 memory cap in core; crate READMEs that exist; drop unused dependencies ([`225cdcb`](https://github.com/sorinirimies/gitkraft/commit/225cdcb5e3660d34ed4d5ea460d690522605bed7))

### 🧪 Tests

- Define main after every test ([`ee74241`](https://github.com/sorinirimies/gitkraft/commit/ee74241f33134c8c394fec35e208b0e8605dc34e))

### 📦 Dependencies

- Nightly cargo update 2026-11-05 ([`f3fbb60`](https://github.com/sorinirimies/gitkraft/commit/f3fbb60922775fd6cf07ac741e89c98b446c99f5))
- Nightly dependency upgrade 2026-11-04 ([`b792c8c`](https://github.com/sorinirimies/gitkraft/commit/b792c8cf2230513083e94b791f81beaf12179a8d))

### 🔄 CI and release

- Download archives to a file so curl retries a dropped transfer, instead of piping into tar ([`8955e6a`](https://github.com/sorinirimies/gitkraft/commit/8955e6a205ebdb9bf60d29d8a69b07f898ef1b5c))
- **wayland:** Start the desktop app on headless GNOME (mutter) - weston has no seat, which gpui needs - inside one session bus, as nobody when root ([`6524173`](https://github.com/sorinirimies/gitkraft/commit/652417304029e17a28298e4955f4221fb8988947))
- Start the desktop app on a headless Wayland compositor; StartupWMClass follows the desktop id ([`8071c6b`](https://github.com/sorinirimies/gitkraft/commit/8071c6ba549970046adff1765e87be2eddd748df))
- The Dependabot sweep asks for a rebase on conflicts and keeps going after a failed merge ([`6ec1aa0`](https://github.com/sorinirimies/gitkraft/commit/6ec1aa0c82b053ba658173359d1aa63db6f3b2e8))
- Merge green Dependabot PRs, majors included, with a nightly sweep; nightly commits skip ignored lockfiles ([`eb88d16`](https://github.com/sorinirimies/gitkraft/commit/eb88d166651e867d607b9f9836ffaf828277c343))
- Create-rust-project --desktop packaging (deb/rpm, AppImage, DMG, installer and AUR from project.nu); Snap and Flathub releases in Nushell; secrets scan, unused-dependency gate ([`98bf304`](https://github.com/sorinirimies/gitkraft/commit/98bf304a8afe1d46aaf06cbba9ae501c5c12d006))

### 🔧 Chores

- **deny:** List pin-utils (RUSTSEC-2026-0441) in deny.toml; no upgrade path ([`883137e`](https://github.com/sorinirimies/gitkraft/commit/883137e17a399ab35c4571a8e58420e0168b334b))
- Stop tracking the local fuzz corpus (added by mistake in the 1.2.14 bump) ([`0c20e51`](https://github.com/sorinirimies/gitkraft/commit/0c20e515282ce2da62b7e09bbef21aee531961e6))

## [1.2.15](https://github.com/sorinirimies/gitkraft/compare/v1.2.14...v1.2.15) — 2026-11-03

### 📦 Dependencies

- Nightly dependency upgrade 2026-11-03 ([`03874e7`](https://github.com/sorinirimies/gitkraft/commit/03874e7dc915ece36e785e1f103ab50d70932a39))

## [1.2.14](https://github.com/sorinirimies/gitkraft/compare/v1.2.12...v1.2.14) — 2026-11-02

### 📚 Documentation

- Drop the per-repo agent rules skill; the shared skills cover it ([`b502a87`](https://github.com/sorinirimies/gitkraft/commit/b502a879e48195d807a1bdfbb080e24acd2f9c5d))
- This repo's agent rules as a project skill in .agents/skills ([`522a8f9`](https://github.com/sorinirimies/gitkraft/commit/522a8f9c1510bb57be27345c013b645a7af7c949))

### 🔄 CI and release

- Ignore unmaintained zbus/zvariant advisories, drop stale git2 ignores ([`8116a2e`](https://github.com/sorinirimies/gitkraft/commit/8116a2edd5f510fd4bb38d226efa897466b11b72))

### 🔧 Chores

- Align just git workflows with tokenburn ([`305a39d`](https://github.com/sorinirimies/gitkraft/commit/305a39dd09b9d249757d9c8cfce53fc60a67d161))

## [1.2.12](https://github.com/sorinirimies/gitkraft/compare/v1.2.8...v1.2.12) — 2026-11-01

### ✨ Features

- **tui:** Source all themes from tui-nuances (47 built-ins) ([`556256b`](https://github.com/sorinirimies/gitkraft/commit/556256b60850df9951b58a9fdbf09d1585897c2e))

### 🐛 Bug fixes

- **gui:** Drop unnecessary unsafe after objc2 update ([`e547ecc`](https://github.com/sorinirimies/gitkraft/commit/e547eccea3c145fa5c7d4cda24eb1d84cfe067b2))
- **tui:** Take graph lane colours from the theme palette ([`7970c40`](https://github.com/sorinirimies/gitkraft/commit/7970c4052cda7b0588fe4acfd5e2642422b5d990))

### 📚 Documentation

- Move agent rules from .zed/rules.md to AGENTS.md ([`9d2f5d8`](https://github.com/sorinirimies/gitkraft/commit/9d2f5d8ad377bfb6dd74ed0013665d006d76b21b))

### 📦 Dependencies

- Tui-file-explorer 4.0 (tui-nuances-only themes) ([`0cbfe26`](https://github.com/sorinirimies/gitkraft/commit/0cbfe26986834700845f8fefe113b237758a1ed8))
- Bump tui-nuances to 0.1.10 ([`9af42e8`](https://github.com/sorinirimies/gitkraft/commit/9af42e85e8b0c2df7ec6522921ede8cba3bb57f6))
- Nightly dependency upgrade 2026-10-20 ([`c8d3af3`](https://github.com/sorinirimies/gitkraft/commit/c8d3af3d1ef33c6aa38375e4dae264019dfa690b))
- Nightly dependency upgrade 2026-10-18 ([`d57a364`](https://github.com/sorinirimies/gitkraft/commit/d57a3640073f7d3feac7277038bcac67d8094d2c))
- Nightly dependency upgrade 2026-10-17 ([`797a3f8`](https://github.com/sorinirimies/gitkraft/commit/797a3f82727dca98b2ece5fa6ce5cd16692241de))
- Nightly dependency upgrade 2026-10-16 ([`ce018eb`](https://github.com/sorinirimies/gitkraft/commit/ce018eba6ada5a0af6a4601118a77d3ebbfa3758))

### 🔄 CI and release

- Build docs on stable and update dependencies ([`5fdd575`](https://github.com/sorinirimies/gitkraft/commit/5fdd575b108391f4dfe8a6a634683deb5246da42))
- Nightly merges lockfile-only updates; bump + release only when a Cargo.toml requirement changes ([`f64a1ba`](https://github.com/sorinirimies/gitkraft/commit/f64a1ba27e6e42d8c706b72943b60d6f020dab45))
- Nightly dependency update pushes lockfile only, no version bump or release ([`04eaac7`](https://github.com/sorinirimies/gitkraft/commit/04eaac7d3b96c1887069a32157b56629dcbc0916))
- Harden cargo network retries/timeouts in release workflow ([`d942dc6`](https://github.com/sorinirimies/gitkraft/commit/d942dc686bdc663ba0cc491b7b97e921c2c94612))

### 🔧 Chores

- Remove AGENTS.md ([`d35886b`](https://github.com/sorinirimies/gitkraft/commit/d35886b0fc0b94b39934aef4de58850d8201d8fd))

## [1.2.8](https://github.com/sorinirimies/gitkraft/compare/v1.2.6...v1.2.8) — 2026-10-15

### ✨ Features

- **gui:** Move Settings and About into the header next to Fetch/Open/Close; keep version in tab bar ([`63b6c11`](https://github.com/sorinirimies/gitkraft/commit/63b6c112f32829405ac48c35cbdbe1f28f047f88))
- **tui:** Use tui-file-explorer (lightweight, no cli) for the o directory browser ([`283f56f`](https://github.com/sorinirimies/gitkraft/commit/283f56f802a4450bce2b9b3480cdc4bfd06d0ae9))

### 🔄 CI and release

- Prebuilt Nushell, cargo-deny + cargo-audit security workflow ([`a61c269`](https://github.com/sorinirimies/gitkraft/commit/a61c269978c22a5aaf9867184c3fbe61d1955160))

### 🔧 Chores

- **justfile:** Push/pull current branch instead of hardcoded main ([`c9f190f`](https://github.com/sorinirimies/gitkraft/commit/c9f190f4beaebbe66afa5f20baac27e7fb766d17))

## [1.2.6](https://github.com/sorinirimies/gitkraft/compare/v1.2.5...v1.2.6) — 2026-10-14

### ✨ Features

- **gui:** Vertical compact mode for short windows ([`10cbce2`](https://github.com/sorinirimies/gitkraft/commit/10cbce23cd0193f623afb59c7114f6214e5265d5))
- **tui:** Header shortcut hints adapt to terminal width ([`62d60c4`](https://github.com/sorinirimies/gitkraft/commit/62d60c40942603a8950ac6a66fb6983fffea6a22))
- Responsive TUI layout, clamped GUI dialogs and sidebar scrim, System theme option (GUI + TUI) ([`13f68c8`](https://github.com/sorinirimies/gitkraft/commit/13f68c863c658a3fa5e03063a644b3f859af19ee))
- **gui:** Deeper responsive layout — adaptive commit columns, diff min width, compact toolbars, stacked staging, clamped file list ([`7ac763b`](https://github.com/sorinirimies/gitkraft/commit/7ac763b9405dbf9be0a9a4f43c989eb7913db5d3))

## [1.2.5](https://github.com/sorinirimies/gitkraft/compare/v1.2.4...v1.2.5) — 2026-10-14

### ✨ Features

- **gui:** Responsive layout with clamped panes and overlay sidebar on narrow windows ([`41d367d`](https://github.com/sorinirimies/gitkraft/commit/41d367d51457c0bb0e190fbf2ba18ed304439b6e))

## [1.2.4](https://github.com/sorinirimies/gitkraft/compare/v1.2.2...v1.2.4) — 2026-10-13

### 🔄 CI and release

- Split fuzz into own workflow; add audit & fuzz badges to README ([`9c0d964`](https://github.com/sorinirimies/gitkraft/commit/9c0d964ce73e05db6a49b7027302d56ba3b5c815))

## [1.2.2](https://github.com/sorinirimies/gitkraft/compare/v1.2.1...v1.2.2) — 2026-10-13

### 🐛 Bug fixes

- Avoid panics in graph column indexing and short_oid_str on multibyte input ([`f90db58`](https://github.com/sorinirimies/gitkraft/commit/f90db582112c16f6dd8855cd094035c85522e62e))
- **watcher:** Ignore inotify access/lock events to stop refresh loop; chore(deps): update lockfile; ci: fix nightly deps toolchain setup ([`74f1dd1`](https://github.com/sorinirimies/gitkraft/commit/74f1dd1827a20d5605e96cdf15845ee76d30939a))

### 🔄 CI and release

- Bump actions/upload-artifact from 4 to 7 ([`4bd7e60`](https://github.com/sorinirimies/gitkraft/commit/4bd7e60d19841791b6a66e09e94a3f9eead8c899))
- **release:** Run cargo audit in release gate; add audit and fuzz-smoke to just check-release ([`417bca3`](https://github.com/sorinirimies/gitkraft/commit/417bca39e9d7f89b7099ecfe28f5cb19472104f3))
- **security:** Add cargo-audit and cargo-fuzz targets with nightly workflows ([`5588c20`](https://github.com/sorinirimies/gitkraft/commit/5588c2047236fa27bc8bf2dc7cd07619b2dae4b4))
- **packaging:** Add snap and flathub publishing, icons, desktop/metainfo, AUR desktop entry ([`6fc6734`](https://github.com/sorinirimies/gitkraft/commit/6fc6734a1e2673831b712c834355a00c60697239))

### 💄 Style

- Cargo fmt watcher ([`bbfb0da`](https://github.com/sorinirimies/gitkraft/commit/bbfb0da4ea8b3ef8766806fac7481305c69590f7))

## [1.2.1](https://github.com/sorinirimies/gitkraft/compare/v1.2.0...v1.2.1) — 2026-10-10

### 🐛 Bug fixes

- **gui:** Unresolved rustdoc link to macos ([`2659630`](https://github.com/sorinirimies/gitkraft/commit/2659630718bac8f4e691a1bc2051be5989583b8c))

### 🔄 CI and release

- **gitea:** Install alien, fakeroot and arm64 libc headers for release builds ([`6c570d8`](https://github.com/sorinirimies/gitkraft/commit/6c570d8cf17faa3bc00696c30e82d421661f2f52))

## [1.2.0](https://github.com/sorinirimies/gitkraft/compare/v1.1.6...v1.2.0) — 2026-10-09

### ✨ Features

- Add version label, settings and about menu items to GUI and TUI ([`6ef9b52`](https://github.com/sorinirimies/gitkraft/commit/6ef9b52d9e4252ca94c5c501132cb05d3db3d158))

### 🐛 Bug fixes

- **gui:** Add the macOS Window menu (window module) so Zoom, Move & Resize and the tiling shortcuts work ([`228282d`](https://github.com/sorinirimies/gitkraft/commit/228282d0b2fae10f7f366fea42e47b44c92b279c))

### 📚 Documentation

- Refresh README — fix stale info, expand keyboard shortcuts, no preview changes ([`eaf6263`](https://github.com/sorinirimies/gitkraft/commit/eaf6263265737eec69fdafddd535ebfbe87cb7d0))
- Regenerate VHS demo GIFs, add GUI screenshots + recording to README ([`a51ab9a`](https://github.com/sorinirimies/gitkraft/commit/a51ab9aca54a06e7e19d4641c0118efadc27b672))

### 📦 Dependencies

- Bump tui-file-explorer 1.1.6 -> 1.1.11, tui-checkbox 0.4.4 -> 0.4.6 ([`7838d23`](https://github.com/sorinirimies/gitkraft/commit/7838d23cda5438b0c89d850be2673d589a5c7e7c))

### 🔧 Chores

- Update dependencies ([`df60e03`](https://github.com/sorinirimies/gitkraft/commit/df60e0320c5add90ecec0f31ed59757fbc61817a))

## [1.1.6](https://github.com/sorinirimies/gitkraft/compare/v1.1.5...v1.1.6) — 2026-08-02

### 🔄 CI and release

- Replace rpmbuild with alien for .rpm packaging (cross-arch never worked) ([`ef75650`](https://github.com/sorinirimies/gitkraft/commit/ef7565045d5f688f3fbece9e4935838ef6a850d6))
- Anchor all installer.nsi relative paths to the repo root via !cd ([`e71c67b`](https://github.com/sorinirimies/gitkraft/commit/e71c67bb9c33a20ab125bda1f413704c342f221f))
- Drop rpmbuild --target, fix cross-arch aarch64 .rpm builds for real ([`3f5b486`](https://github.com/sorinirimies/gitkraft/commit/3f5b4868c5ecaed0facdaf3d657b302146a24af6))
- Resolve Windows installer icon to an absolute path ([`9c6fcb6`](https://github.com/sorinirimies/gitkraft/commit/9c6fcb623e1d207c3d0f82b3e9ed576c03f824c1))
- Replace PowerShell/System.Drawing icon fallback with a verified static ICO ([`f2ff23c`](https://github.com/sorinirimies/gitkraft/commit/f2ff23cd94c918c3d03a540adb40928203d3cab9))
- Fix Windows icon fallback size, AppImage FUSE, and rpm cross-arch build ([`abc06fe`](https://github.com/sorinirimies/gitkraft/commit/abc06feb600234976f784a75715d23b2108848de))
- Fix runtime bug in package_linux.nu; add Windows icon fallback ([`6826488`](https://github.com/sorinirimies/gitkraft/commit/682648805662adbd9234c7986559182828645dde))
- Fix nu parse errors in package_linux.nu; add test coverage ([`4650715`](https://github.com/sorinirimies/gitkraft/commit/465071578c9ebb314510e98ebf26e47dd2e6dc16))

## [1.1.5](https://github.com/sorinirimies/gitkraft/compare/v1.1.4...v1.1.5) — 2026-08-02

### 🐛 Bug fixes

- **ci:** Fix remaining 3 build failures (Nu date, Windows makensis path) ([`9597a3f`](https://github.com/sorinirimies/gitkraft/commit/9597a3fce6bf95543960e00c1c91f3a5b00c5a97))

### 🔄 CI and release

- Bump actions/upload-artifact from 4 to 7 ([`0eb8163`](https://github.com/sorinirimies/gitkraft/commit/0eb81635169b4c57ec0ddef4a2f3e97b2d96c0ae))
- Bump KSXGitHub/github-actions-deploy-aur from 2 to 3 ([`a5487d4`](https://github.com/sorinirimies/gitkraft/commit/a5487d40544054e4e9d4136abff7975c0701be5a))
- Bump actions/download-artifact from 4 to 8 ([`65c1bfc`](https://github.com/sorinirimies/gitkraft/commit/65c1bfc80efc40aa01d2e52d55a1e79b4b066756))
- Bump actions/checkout from 4 to 7 ([`e2ac235`](https://github.com/sorinirimies/gitkraft/commit/e2ac235d984408660a8fad4c5d6c67103b9f2b0e))

### 🔧 Chores

- Align git-cliff and changelog setup with tui-file-explorer ([`f15214c`](https://github.com/sorinirimies/gitkraft/commit/f15214c48e5af6214eadd0ff5d586d98193ed192))

## [1.1.4](https://github.com/sorinirimies/gitkraft/compare/v1.1.3...v1.1.4) — 2026-06-26

### 🐛 Bug fixes

- **ci:** Resolve all 4 release build failures ([`6f40365`](https://github.com/sorinirimies/gitkraft/commit/6f403654d388f0c8b8f81608f336009a795a7d8b))

## [1.1.3](https://github.com/sorinirimies/gitkraft/compare/v1.1.2...v1.1.3) — 2026-06-26

### ✨ Features

- Multi-platform packaging and distribution ([`0d9e3f4`](https://github.com/sorinirimies/gitkraft/commit/0d9e3f47fef4ba5a6c5d1b74d50a63872f987a74))

### 🐛 Bug fixes

- Resolve clippy errors blocking release gate ([`b586884`](https://github.com/sorinirimies/gitkraft/commit/b586884319e3aed777aba5a5d46fce0ec2b0e996))

### ♻️ Refactor

- Harden safety, memory, and reliability across all crates ([`506a9d8`](https://github.com/sorinirimies/gitkraft/commit/506a9d832a1d454104e4445d71abdc7d9b47246b))

### 🔄 CI and release

- Split release workflows by platform capability ([`2d1a2d2`](https://github.com/sorinirimies/gitkraft/commit/2d1a2d28087eb820cdc59d1ffa214f79f6622f6e))

## [1.0.7](https://github.com/sorinirimies/gitkraft/compare/v1.0.6...v1.0.7) — 2026-05-20

### 📦 Dependencies

- Nightly dependency upgrade 2026-05-16 ([`10c0dd0`](https://github.com/sorinirimies/gitkraft/commit/10c0dd0d62413e13e030bee02dcfe91a235b5809))
- Nightly dependency upgrade 2026-05-14 ([`6a0cd89`](https://github.com/sorinirimies/gitkraft/commit/6a0cd893eb46ce08a0a4c6bfb396df458e4687ab))

## [1.0.5](https://github.com/sorinirimies/gitkraft/compare/v1.0.4...v1.0.5) — 2026-05-13

### 📦 Dependencies

- Nightly dependency upgrade 2026-05-12 ([`d223b84`](https://github.com/sorinirimies/gitkraft/commit/d223b8483caf1fcb5b5b436f9703aeb40ed5235c))

## [0.9.0](https://github.com/sorinirimies/gitkraft/compare/v0.8.8...v0.9.0) — 2026-05-02

### 🐛 Bug fixes

- Broken rustdoc link to private render_main function ([`f1e6bb8`](https://github.com/sorinirimies/gitkraft/commit/f1e6bb856896ead7c611c1d5c296afbf9388ced2))

### 📦 Dependencies

- Nightly dependency upgrade 2026-05-02 ([`a239f00`](https://github.com/sorinirimies/gitkraft/commit/a239f00fb86d9a094ac0ba72ba93b5a97b7330e0))
- Nightly dependency upgrade 2026-05-01 ([`1405a5c`](https://github.com/sorinirimies/gitkraft/commit/1405a5c44d6b7aa656634329c1388cafde68e55c))

## [0.8.8](https://github.com/sorinirimies/gitkraft/compare/v0.8.6...v0.8.8) — 2026-04-30

### 📦 Dependencies

- Nightly dependency upgrade 2026-04-30 ([`184c5b8`](https://github.com/sorinirimies/gitkraft/commit/184c5b84c790f5101ffb5113d10ab8fa0011fc23))

## [0.8.4](https://github.com/sorinirimies/gitkraft/compare/v0.8.2...v0.8.4) — 2026-04-28

### 📦 Dependencies

- Nightly dependency upgrade 2026-04-28 ([`4386a45`](https://github.com/sorinirimies/gitkraft/commit/4386a452a84e2857dcb66154351270176e6ef57c))

## [0.7.7](https://github.com/sorinirimies/gitkraft/compare/v0.7.4...v0.7.7) — 2026-04-24

### 📦 Dependencies

- Nightly dependency upgrade 2026-04-24 ([`39ddc4d`](https://github.com/sorinirimies/gitkraft/commit/39ddc4d6912de3522450ae03e7b39b6d0cf8b413))

## [0.7.4](https://github.com/sorinirimies/gitkraft/compare/v0.7.3...v0.7.4) — 2026-04-23

### ✨ Features

- Auto-focus search input, add tests for new features ([`fadbffb`](https://github.com/sorinirimies/gitkraft/commit/fadbffb929837b4dc90dac8a5c643ed6261e7971))
- **gui:** 'Diff Selected' button for multi-file diff in search ([`11b54a4`](https://github.com/sorinirimies/gitkraft/commit/11b54a42e074d50852f10e7c301ddfcbf71daba5))
- **gui:** Diff search results against working tree ([`af882a8`](https://github.com/sorinirimies/gitkraft/commit/af882a8003567fa6576ef9d84840ad0fbaef5e01))

### 🐛 Bug fixes

- **gui:** Prevent search dialog from dismissing when clicking diff content ([`c7c44ce`](https://github.com/sorinirimies/gitkraft/commit/c7c44ce016f7b2719b6fb69689e3baa661f7b1ed))
- **tui:** Open repo in new tab when current tab already has one ([`a0e21e4`](https://github.com/sorinirimies/gitkraft/commit/a0e21e4f749f63c21146129f7f691a41002f49ad))

### 📚 Documentation

- Regenerate all TUI VHS GIFs with updated layout ([`624e4c8`](https://github.com/sorinirimies/gitkraft/commit/624e4c8d4992656a0856d0fd50b2717b6a007615))

## [0.7.1](https://github.com/sorinirimies/gitkraft/compare/v0.6.6...v0.7.1) — 2026-04-23

### 📚 Documentation

- Rewrite tui-tabs tape to open a real second repo ([`e581c7c`](https://github.com/sorinirimies/gitkraft/commit/e581c7c3c69c687fc6c9a1713954f72ebce5fdf2))
- Add TUI theme selector and multi-repo tabs VHS tapes ([`17bbe95`](https://github.com/sorinirimies/gitkraft/commit/17bbe958aa9ae69456a7e225a17b411d150ba0d2))
- Hide cargo run from TUI tapes and remove tui-build GIF ([`6783015`](https://github.com/sorinirimies/gitkraft/commit/67830158217eb53acae65ba3eb1347a808eab464))
- Remove non-UI GIFs from Preview section ([`4f5b6f3`](https://github.com/sorinirimies/gitkraft/commit/4f5b6f38ba6934b0b93c7d99e7c66adcbdf708a2))
- Add VHS-generated preview GIFs tracked with Git LFS ([`73c2c65`](https://github.com/sorinirimies/gitkraft/commit/73c2c65dbb0731ef74bd38561352b592643b3bce))

### 📦 Dependencies

- Nightly dependency upgrade 2026-04-23 ([`df7be02`](https://github.com/sorinirimies/gitkraft/commit/df7be02e6e24b420205e64fb83ff5869e131cea6))
- Nightly dependency upgrade 2026-04-22 ([`8fddd31`](https://github.com/sorinirimies/gitkraft/commit/8fddd31e69e85ad474991a7c9352c5d9a4ef5eb7))

### 🔧 Chores

- Add .zed/rules.md with project conventions for agents ([`2baa4f5`](https://github.com/sorinirimies/gitkraft/commit/2baa4f5933a0072cc53841769bcee80719d7c002))

## [0.3.2](https://github.com/sorinirimies/gitkraft/compare/v0.3.1...v0.3.2) — 2026-04-05

### 🐛 Bug fixes

- **ci:** Remove macOS x86_64 build target (no macos-13 runner available) ([`793adfd`](https://github.com/sorinirimies/gitkraft/commit/793adfd32df1aa37a730f6c5325733615012495f))

## [0.3.1](https://github.com/sorinirimies/gitkraft/compare/v0.3.0...v0.3.1) — 2026-04-04

### 🐛 Bug fixes

- **ci:** Use vcpkg zlib on Windows to fix libz-sys build ([`66f3ef9`](https://github.com/sorinirimies/gitkraft/commit/66f3ef995d5020e67683f4dec96b21ab75c13ec8))

## [0.3.0](https://github.com/sorinirimies/gitkraft/compare/v0.2.3...v0.3.0) — 2026-04-04

### 🔄 CI and release

- Bump actions/upload-artifact from 4 to 7 ([`9ebe780`](https://github.com/sorinirimies/gitkraft/commit/9ebe7806e31a212bdfe3ed7b1aaac319be505b07))
- Bump actions/checkout from 5 to 6 ([`2417443`](https://github.com/sorinirimies/gitkraft/commit/2417443e3e6af402a9fb2af123a339a6767a5120))
- Bump actions/download-artifact from 4 to 8 ([`a380d4b`](https://github.com/sorinirimies/gitkraft/commit/a380d4b51ff38e1683b22e7ee2e138f84c2cf9a8))

## [0.1.2](https://github.com/sorinirimies/gitkraft/releases/tag/v0.1.2) — 2026-04-04


