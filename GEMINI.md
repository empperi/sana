\# Instructions on code style

@./AGENTS.md



# MCP

- Whenever I ask you to 'verify the UI' or 'check the page' always use the playwright MCP server to navigate and confirm the changes visually
- For high-level project mapping, symbol lookups (functions, structs), and advanced searching, always prioritize using the `code-indexer` MCP tools (e.g., `get_file_summary`, `get_symbol_body`, `search_code_advanced`).
- If the `code-indexer` is disconnected (meaning the dev docker container is not running), fall back to standard `grep_search` and `read_file` tools.
- Periodically check the project path in `code-indexer` using `set_project_path(path="/project")` to ensure the index is up-to-date after large changes.




