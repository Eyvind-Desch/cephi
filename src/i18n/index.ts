import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { de } from "./locales/de";
import { en } from "./locales/en";

export type Language = "en" | "de";

void i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    de: { translation: de },
  },
  lng: "en",
  fallbackLng: "en",
  // React already escapes rendered values.
  interpolation: { escapeValue: false },
});

/** Switches the UI language (the backend resolves "system" to a concrete language). */
export function setLanguage(language: Language) {
  document.documentElement.lang = language;
  if (i18n.language !== language) void i18n.changeLanguage(language);
}

/** Formats a duration in the current language, e.g. "850 ms", "1.5 s" / "1,5 s". */
export function formatDuration(ms: number) {
  if (ms < 1000) return i18n.t("toolCard.durationMs", { value: ms });
  const seconds = new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 1 }).format(ms / 1000);
  return i18n.t("toolCard.durationS", { value: seconds });
}

export default i18n;
