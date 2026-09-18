# MCP servers

MCP (Model Context Protocol) is an open standard for connecting AI apps to other programs. An MCP server offers
tools, e.g. "search GitHub", "read a file", "create a calendar entry". Cephi starts the servers listed in
**`%APPDATA%\Cephi\mcp.json`** and offers their tools to the AI next to its own.

The file has the **same format as Claude Desktop**, so the setup snippets most servers publish can be pasted
as they are. Changes apply as soon as the file is saved. `/tools` shows every server with its status and tools.

## Examples

```json
{
  "$schema": "./schemas/mcp.schema.json",
  "mcpServers": {
    "files": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "C:\\Users\\me\\Documents"]
    },
    "memory": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-memory"],
      "permission": "auto"
    },
    "github": {
      "url": "https://api.githubcopilot.com/mcp/",
      "headers": { "Authorization": "Bearer {{secret:github_token}}" },
      "autoApprove": ["search_code", "get_file_contents"]
    }
  }
}
```

- **Local servers** (`command`, `args`, `env`, `cwd`) run as a hidden background process. `npx` needs Node.js,
  `uvx` needs [uv](https://docs.astral.sh/uv/). The first start of an `npx` server downloads it, which can take
  a moment.
- **Remote servers** (`url`, `headers`) are reached over HTTP (streamable HTTP).
- **API keys** never go into the file: write `{{secret:name}}` in `env` or `headers`. The key then appears under
  ⋯ → *Keys* (section *Custom tools*), where you enter it once.
- **Directories and catalogs:** [modelcontextprotocol.io](https://modelcontextprotocol.io),
  [awesome-mcp-servers](https://github.com/wong2/awesome-mcp-servers). Gateways that bundle many apps behind one
  URL work the same way as any remote server.

## Permissions

| Setting | Effect |
|---|---|
| (default) | Every call shows a confirmation card with the server, tool and arguments |
| `"autoApprove": ["tool", …]` | These tools run without asking (good for read-only ones) |
| `"permission": "auto"` | All tools of this server run without asking |
| `"disabled": true` | Server is not started |

## Names

The AI sees a server's tools as `<server>__<tool>`, e.g. `github__search_code`, and the description starts with
`[server]`. Server names: letters, digits, `-` and `_`.

## Not supported yet

- Servers that require a browser login (OAuth), e.g. some hosted SaaS servers. Use their API-key variant if they
  have one.
- MCP resources and prompts (only tools are used).

## Troubleshooting

- A server that fails to start shows up as a configuration problem in the panel and under `/tools`, with the reason.
- Server output (stderr) is written to the Cephi log: `%LOCALAPPDATA%\io.github.eyvind-desch.cephi\logs\`.
