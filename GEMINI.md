
@./AGENTS.md

Always check in startup in which system you are running on: Powershell or Unix based shell such as zsh or bash.
Remember this for all further CLI commands.

# MCP

- Whenever I ask you to 'verify the UI' or 'check the page' always use the playwright MCP server to navigate and confirm the changes visually.
- For high-level project mapping, symbol lookups (functions, structs), and advanced searching, always prioritize using the `code-indexer` (ViperJuice/Code-Index-MCP) server.
- The indexer runs in a Docker container (`ghcr.io/viperjuice/code-index-mcp:latest`) and is automatically invoked via `.gemini/settings.json`.
- If the indexer is unavailable or fails, fall back to standard `grep_search` and `read_file` tools.
- Periodically use the indexer's `reindex` or `search` tools to ensure the context remains up-to-date after large changes.
