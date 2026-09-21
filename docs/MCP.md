# MCP servers

MCP (Model Context Protocol) is an open standard for connecting AI apps to other programs. An MCP server offers
tools, e.g. "search GitHub", "read a file", "create a calendar entry". Cephi starts the servers listed in
**`%APPDATA%\Cephi\mcp.json`** and offers their tools to the AI next to its own.

**Without editing the file:** ⋯ → *Settings* → *MCP* (or `/mcp`). *Add server* takes a name and the command or URL;
*Paste snippet* fills the form from the JSON a server's README shows (Claude Desktop, Cursor or VS Code format).
Values marked with the lock (tokens, keys; names like `*_TOKEN` are marked automatically) go to the Windows
Credential Manager and the file only gets `{{secret:…}}`. Each server can be switched off, edited or removed,
and single tools can be allowed to run without asking. Changing a key restarts the servers that use it.

The file has the **same format as Claude Desktop**, so the setup snippets most servers publish can also be pasted
there directly. Changes apply as soon as the file is saved (only servers whose start settings changed restart).
`/tools` shows every server with its status and tools.

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

## Many servers

Every tool costs context, and small local models choose worse the more tools they see. With more than
**8 MCP tools** (local models) or **64** (cloud models), each request only gets the tools whose name, server
or description share words with the question (the last two messages), plus those already used in the chat.
Naming the server helps: "create a GitHub issue …", "search Notion for …".

## Browser login (OAuth)

Online servers that want an account instead of an API key (Notion, Linear, hosted GitHub servers, …) are added
with their URL and no header. In ⋯ → *Settings* → *MCP*, open the server and click **Log in with the browser**:
Cephi registers itself with the server, opens its login page and waits on a loopback port for the redirect.
The tokens land in the Windows Credential Manager (`Cephi/mcp_oauth_<server>`), never in `mcp.json`, and are
refreshed automatically. *Forget login* removes them; deleting the server does too.

## Not supported yet

- MCP resources and prompts (only tools are used).

## Troubleshooting

- A server that fails to start shows up as a configuration problem in the panel and under `/tools`, with the reason.
- Server output (stderr) is written to the Cephi log: `%LOCALAPPDATA%\io.github.eyvind-desch.cephi\logs\`.
