import { useUi } from "../../store/ui";
import { Composer } from "../chat/Composer";
import { MessageList } from "../chat/MessageList";
import { KeysView } from "../keys/KeysView";
import { DiagnosticsBanner } from "./DiagnosticsBanner";
import { Header } from "./Header";

/** Expanded mode: full chat panel. */
export function Panel() {
  const view = useUi((s) => s.view);

  return (
    <div className="flex h-full flex-col">
      <Header />
      <DiagnosticsBanner />
      {view === "keys" ? (
        <KeysView />
      ) : (
        <>
          <MessageList />
          <div className="p-3 pt-1">
            <Composer />
          </div>
        </>
      )}
    </div>
  );
}
