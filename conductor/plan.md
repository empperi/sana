# Objective
Migrate the local code indexer from a brittle, custom monkey-patched MCP server to the official `ViperJuice/Code-Index-MCP` server, executed via a Docker container.

# Background & Motivation
The current code indexer setup requires a manual patch (`patch_indexer.py`) at build time to add deep Rust indexing support using `tree-sitter-rust`. `ViperJuice/Code-Index-MCP` natively supports over 48 languages including Rust without needing manual monkey patches. Migrating to it cleans up the workspace and improves reliability.

# Key Files & Context
- `.gemini/settings.json` (MCP server configuration)
- `docker-compose.yml` (current code-indexer service)
- `Dockerfile.mcp-index` (to be removed)
- `patch_indexer.py` (to be removed)
- `README.md` & `GEMINI.md` (documentation updates)

# Proposed Solution
1. **Remove Old Tooling:** Delete `Dockerfile.mcp-index` and `patch_indexer.py`.
2. **Remove Docker Compose Service:** Remove the `code-indexer` service block from `docker-compose.yml`.
3. **Configure the New MCP Server:** Update `.gemini/settings.json` to execute `ghcr.io/viperjuice/code-index-mcp:latest` directly via `docker run`, mounting the workspace for indexing.
4. **Update Documentation:** Revise documentation references in `README.md` and `GEMINI.md` to reflect the changes.

# Alternatives Considered
- Running the ViperJuice MCP server via Native Python (`pip`) or `npx` (Node.js). The user opted for Docker to avoid local environment dependencies and keep the setup fully portable.

# Implementation Steps
1. **Remove Legacy Files:** Delete `patch_indexer.py` and `Dockerfile.mcp-index`.
2. **Update `docker-compose.yml`:** Remove the `code-indexer` service.
3. **Update `.gemini/settings.json`:** Replace the `mcpServers.code-indexer` section with the following `docker run` command array pointing to the `ghcr.io/viperjuice/code-index-mcp:latest` image:
```json
{
  "mcpServers": {
    "code-indexer": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-v",
        "/Users/ncollin/code/cinia/sana:/workspace",
        "ghcr.io/viperjuice/code-index-mcp:latest"
      ]
    }
  }
}
```
4. **Update `README.md`:** Remove the step referring to `docker-compose --profile dev up -d code-indexer` and update any references to the indexer startup.
5. **Update `GEMINI.md`:** Ensure instructions correctly mention the new MCP server context and any specific configuration if necessary.

# Verification
- Once implemented, verify that Gemini CLI connects to `code-indexer` without errors.
- Attempt to use `search_code_advanced` or `get_symbol_body` from the new indexer to test Tree-sitter AST parsing.

# Migration & Rollback
- Since the `.gemini/settings.json` and `docker-compose.yml` modifications are lightweight, they can be easily reverted via Git in case of any issues with the new Docker image.
