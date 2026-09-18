import { useUi } from "../../store/ui";
import { Composer } from "../chat/Composer";
import { MessageList } from "../chat/MessageList";
import { SelectionChip } from "../chat/SelectionChip";
import { KeysView } from "../keys/KeysView";
import { TranslateView } from "../translate/TranslateView";
import { WebView } from "../web/WebView";
import { DiagnosticsBanner } from "./DiagnosticsBanner";
import { Header } from "./Header";

/** Expanded mode: header plus the current view (chat, web tab, translator or keys). */
export function Panel() {
  const view = useUi((s) => s.view);

  return (
    <div className="flex h-full flex-col">
      <Header />
      <DiagnosticsBanner />
      {view === "keys" && <KeysView />}
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
