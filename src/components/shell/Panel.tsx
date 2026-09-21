import { lazy, Suspense, type ComponentType } from "react";
import { useUi, type PanelView } from "../../store/ui";
import { Composer } from "../chat/Composer";
import { MessageList } from "../chat/MessageList";
import { PageChip } from "../chat/PageChip";
import { ScreenshotChip } from "../chat/ScreenshotChip";
import { VoiceError } from "../chat/VoiceError";
import { SelectionChip } from "../chat/SelectionChip";
import { DiagnosticsBanner } from "./DiagnosticsBanner";
import { Header } from "./Header";

/**
 * Views that are not the chat load when they are opened for the first time. They are a good third
 * of the code (MCP editor, translator, web tab), and Cephi has to appear the moment the hotkey is
 * pressed — the chat must not wait for code nobody looked at yet.
 */
const LAZY_VIEWS: Partial<Record<PanelView, ComponentType>> = {
  history: lazy(() => import("../history/HistoryView").then((m) => ({ default: m.HistoryView }))),
  settings: lazy(() => import("../settings/SettingsView").then((m) => ({ default: m.SettingsView }))),
  tools: lazy(() => import("../tools/ToolsView").then((m) => ({ default: m.ToolsView }))),
  translate: lazy(() => import("../translate/TranslateView").then((m) => ({ default: m.TranslateView }))),
  web: lazy(() => import("../web/WebView").then((m) => ({ default: m.WebView }))),
};

/** Expanded mode: header plus the current view (chat, web tab, translator, tools or settings). */
export function Panel() {
  const view = useUi((s) => s.view);
  const View = LAZY_VIEWS[view];

  return (
    <div className="flex h-full flex-col">
      <Header />
      <DiagnosticsBanner />
      {View && (
        // Loading takes a few milliseconds from disk; an empty area is calmer than a spinner.
        <Suspense fallback={<div className="flex-1" />}>
          <View />
        </Suspense>
      )}
      {view === "chat" && (
        <>
          <MessageList />
          <div className="p-3 pt-1">
            <VoiceError />
            <SelectionChip />
            <ScreenshotChip />
            <PageChip />
            <Composer />
          </div>
        </>
      )}
    </div>
  );
}
