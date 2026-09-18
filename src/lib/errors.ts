/** Backend errors arrive as plain strings; browsers wrap some failures in `Error`. */
export function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}