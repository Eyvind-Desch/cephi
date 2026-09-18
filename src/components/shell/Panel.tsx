import { useUi } from "../../store/ui";
import { Composer } from "../chat/Composer";
import { MessageList } from "../chat/MessageList";
import { SelectionChip } from "../chat/SelectionChip";
import { HistoryView } from "../history/HistoryView";
import { SettingsView } from "../settings/SettingsView";
import { ToolsView } from "../tools/ToolsView";
import { TranslateView } from "../translate/TranslateView";
import { WebView } from "../web/WebView";
import { DiagnosticsBanner } from "./DiagnosticsBanner";
import { Header } from "./Header";

/** Expanded mode: header plus the current view (chat, web tab, translator, tools or settings). */
export function Panel() {
  const view = useUi((s) => s.view);

  return (
    <div className="flex h-full flex-col">
      <Header />
      <DiagnosticsBanner />
      {view === "history" && <HistoryView />}
      {view === "settings" && <SettingsView />}
      {view === "tools" && <ToolsView />}
      {view === "web" && <WebView />}
      {view === "translate" && <TranslateView />}
      {view === "chat" && (
        <>
          <MessageList />
          <div className="p-3 pt-1">
            <SelectionChip />
            <Composer />
          </div>
        </>
      )}
    </div>
  );
}
