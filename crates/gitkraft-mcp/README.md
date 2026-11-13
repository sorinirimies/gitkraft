# gitkraft-mcp

[GitKraft](https://github.com/sorinirimies/gitkraft) as an MCP server: pull requests, issues, notifications and
CI of GitHub, GitLab, Gitea, Codeberg and Woodpecker for AI assistants. **Read-only unless started with
`--write`**; a forced merge also needs `--allow-force`.

```sh
cargo install gitkraft-mcp
```

```json
{ "mcpServers": { "gitkraft": { "command": "gitkraft-mcp", "args": [] } } }
```

See the [main README](https://github.com/sorinirimies/gitkraft#mcp-server-your-forges-for-an-ai-assistant)
for the tools and options.
