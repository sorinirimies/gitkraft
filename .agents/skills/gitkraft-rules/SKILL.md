---
name: gitkraft-rules
description: Rules for work in the gitkraft repository that the code can't show: constraints, ownership and must-keep behaviour. Use before changing, fixing, reviewing or extending code here, together with the rust-in-projects skill.
---

# gitkraft: rules the code can't show

Hard constraints for this repository. The general conventions (orientation, tests,
scripting, releasing) are in the `rust-in-projects` skill; this file holds only what is
specific to gitkraft.

* **Git logic lives only in `gitkraft-core/src/features/<name>/ops.rs`.** The GUI
  and TUI call it; Git operations run as tasks from `commands.rs`.
* **Themes:** 27 themes shared through core. Never add a theme to only one
  front-end.
* **Errors:** `anyhow` + `.context()`, no `thiserror`. The GUI `Message` enum
  stays flat.
