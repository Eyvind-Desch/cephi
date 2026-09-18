// Minimal MCP server over stdio for Cephi's tests: one tool, `echo`, that returns its text.
// Speaks JSON-RPC 2.0 with newline-delimited messages, like every stdio MCP server.
import readline from "node:readline";

const send = (message) => process.stdout.write(JSON.stringify({ jsonrpc: "2.0", ...message }) + "\n");

readline.createInterface({ input: process.stdin }).on("line", (line) => {
  const message = JSON.parse(line);
  if (message.id === undefined) return; // notifications need no answer

  switch (message.method) {
    case "initialize":
      send({
        id: message.id,
        result: {
          protocolVersion: message.params.protocolVersion,
          capabilities: { tools: {} },
          serverInfo: { name: "echo", version: "1.0.0" },
        },
      });
      break;
    case "tools/list":
      send({
        id: message.id,
        result: {
          tools: [
            {
              name: "echo",
              description: "Returns the given text.",
              inputSchema: { type: "object", properties: { text: { type: "string" } }, required: ["text"] },
            },
          ],
        },
      });
      break;
    case "tools/call":
      send({ id: message.id, result: { content: [{ type: "text", text: `echo: ${message.params.arguments.text}` }] } });
      break;
    default:
      send({ id: message.id, error: { code: -32601, message: `unknown method ${message.method}` } });
  }
});
