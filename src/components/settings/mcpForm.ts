/**
 * Form model for one MCP server: what the settings page edits, and how it maps to `mcp.json`.
 * Secret values never land in the file: they go to the vault and the file gets `{{secret:name}}`.
 */
import type { McpServerConfig, Permission } from "../../lib/ipc";

export interface PairRow {
  key: string;
  /** Plain value, or the new secret value (empty keeps the stored one). */
  value: string;
  secret: boolean;
  /** Existing value with `{{secret:…}}` references, kept while no new value is typed. */
  stored: string | null;
}

export interface ServerForm {
  name: string;
  kind: "local" | "remote";
  command: string;
  /** One argument per line. */
  args: string;
  cwd: string;
  url: string;
  env: PairRow[];
  headers: PairRow[];
  confirm: boolean;
  disabled: boolean;
  autoApprove: string[];
}

const SECRET_REF = /\{\{secret:[a-z0-9_]+\}\}/;
/** Names that usually hold credentials: pasted values of these go to the vault. */
export const SENSITIVE_KEY = /token|key|secret|password|passwd|auth|credential|cookie/i;

export const emptyRow = (): PairRow => ({ key: "", value: "", secret: false, stored: null });

export function emptyForm(): ServerForm {
  return {
    name: "",
    kind: "local",
    command: "npx",
    args: "",
    cwd: "",
    url: "",
    env: [],
    headers: [],
    confirm: true,
    disabled: false,
    autoApprove: [],
  };
}

function rowsFrom(values: Record<string, string> | undefined): PairRow[] {
  return Object.entries(values ?? {}).map(([key, value]) =>
    SECRET_REF.test(value)
      ? { key, value: "", secret: true, stored: value }
      : SENSITIVE_KEY.test(key)
        ? // READMEs show `<YOUR_TOKEN>`: leave the field empty so the real key is asked for.
          { key, value: PLACEHOLDER.test(value) ? "" : value, secret: true, stored: null }
        : { key, value, secret: false, stored: null },
  );
}

const PLACEHOLDER = /^<.*>$|your|x{3,}|\.\.\.|…/i;

export function formFrom(name: string, server: Partial<McpServerConfig>): ServerForm {
  return {
    name,
    kind: server.url ? "remote" : "local",
    command: server.command ?? "",
    args: (server.args ?? []).join("\n"),
    cwd: server.cwd ?? "",
    url: server.url ?? "",
    env: rowsFrom(server.env),
    headers: rowsFrom(server.headers),
    confirm: (server.permission ?? "confirm") === "confirm",
    disabled: server.disabled ?? false,
    autoApprove: server.autoApprove ?? [],
  };
}

/** Vault name for a secret of this server: `mcp_github_authorization`. */
export function secretName(server: string, key: string): string {
  const clean = (s: string) =>
    s
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "");
  return `mcp_${clean(server)}_${clean(key) || "value"}`.slice(0, 64).replace(/_+$/, "");
}

export interface SaveRequest {
  server: Partial<McpServerConfig>;
  /** Vault entries to write before the server is saved. */
  secrets: { name: string; value: string }[];
}

function resolveRows(serverName: string, rows: PairRow[], secrets: SaveRequest["secrets"]) {
  const result: Record<string, string> = {};
  for (const row of rows) {
    const key = row.key.trim();
    if (!key) continue;
    if (!row.secret) {
      result[key] = row.value;
    } else if (row.value.trim()) {
      const name = secretName(serverName, key);
      // `Authorization: Bearer abc` keeps the scheme in the file and only the token in the vault.
      // A bare token in `Authorization` means a bearer token, as with every MCP server so far.
      const bearer = /^bearer\s+/i.exec(row.value.trim());
      const value = bearer ? row.value.trim().slice(bearer[0].length) : row.value.trim();
      const scheme = bearer || (key.toLowerCase() === "authorization" && !/\s/.test(value)) ? "Bearer " : "";
      secrets.push({ name, value });
      result[key] = `${scheme}{{secret:${name}}}`;
    } else if (row.stored) {
      result[key] = row.stored;
    }
  }
  return result;
}

export function toSaveRequest(form: ServerForm): SaveRequest {
  const secrets: SaveRequest["secrets"] = [];
  const name = form.name.trim();
  const permission: Permission = form.confirm ? "confirm" : "auto";
  const common = { disabled: form.disabled, permission, autoApprove: form.autoApprove };
  const server: Partial<McpServerConfig> =
    form.kind === "local"
      ? {
          command: form.command.trim(),
          args: form.args
            .split("\n")
            .map((a) => a.trim())
            .filter(Boolean),
          cwd: form.cwd.trim() || null,
          env: resolveRows(name, form.env, secrets),
          ...common,
        }
      : { url: form.url.trim(), headers: resolveRows(name, form.headers, secrets), ...common };
  return { server, secrets };
}

/** Secret rows that have neither a stored nor a new value. */
export function missingSecrets(form: ServerForm): string[] {
  const rows = form.kind === "local" ? form.env : form.headers;
  return rows.filter((r) => r.key.trim() && r.secret && !r.value.trim() && !r.stored).map((r) => r.key.trim());
}

/**
 * Reads a snippet from a server's README, in any of the common shapes:
 * `{"mcpServers": {"name": {...}}}`, `{"name": {...}}` or just `{"command": ...}`.
 * Also accepts VS Code's `"servers"` key and a missing outer pair of braces.
 */
export function parseSnippet(text: string): { name: string; server: Partial<McpServerConfig> } | null {
  const trimmed = text.trim();
  let value: unknown;
  for (const candidate of [trimmed, `{${trimmed}}`]) {
    try {
      value = JSON.parse(candidate);
      break;
    } catch {
      // try the next shape
    }
  }
  if (!isObject(value)) return null;

  const looksLikeServer = (v: unknown): v is Partial<McpServerConfig> =>
    isObject(v) && (typeof v.command === "string" || typeof v.url === "string");
  const container = isObject(value.mcpServers) ? value.mcpServers : isObject(value.servers) ? value.servers : value;
  if (looksLikeServer(container)) return { name: "", server: normalize(container) };
  const entry = Object.entries(container).find(([, v]) => looksLikeServer(v));
  return entry ? { name: entry[0], server: normalize(entry[1] as Partial<McpServerConfig>) } : null;
}

/** VS Code writes `serverUrl`/`type`; only keep what Cephi reads. */
function normalize(raw: Partial<McpServerConfig> & { serverUrl?: string }): Partial<McpServerConfig> {
  const strings = (v: unknown): Record<string, string> =>
    isObject(v) ? Object.fromEntries(Object.entries(v).map(([k, x]) => [k, String(x)])) : {};
  return {
    command: typeof raw.command === "string" ? raw.command : null,
    args: Array.isArray(raw.args) ? raw.args.map(String) : [],
    cwd: typeof raw.cwd === "string" ? raw.cwd : null,
    url: typeof raw.url === "string" ? raw.url : typeof raw.serverUrl === "string" ? raw.serverUrl : null,
    env: strings(raw.env),
    headers: strings(raw.headers),
  };
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Vault names this server's config refers to (to clean up when it is removed). */
export function referencedSecrets(server: McpServerConfig): string[] {
  const values = [...Object.values(server.env), ...Object.values(server.headers)];
  return [...new Set(values.flatMap((v) => [...v.matchAll(/\{\{secret:([a-z0-9_]+)\}\}/g)].map((m) => m[1])))];
}
