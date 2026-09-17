import i18n from "../../i18n";

/** Language name in the UI language, e.g. `en` → "Englisch". Falls back to the code. */
export function languageName(code: string) {
  try {
    return new Intl.DisplayNames([i18n.language], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
}
