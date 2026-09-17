import { Composer } from "../chat/Composer";
import { MessageList } from "../chat/MessageList";
import { DiagnosticsBanner } from "./DiagnosticsBanner";
import { Header } from "./Header";

/** Expanded mode: full chat panel. */
export function Panel() {
  return (
    <div className="flex h-full flex-col">
      <Header />
      <DiagnosticsBanner />
      <MessageList />
      <div className="p-3 pt-1">
        <Composer />
      </div>
    </div>
  );
}
