import { useEffect, useRef } from "react";
import { useChat } from "../../store/chat";
import { EmptyState } from "./EmptyState";
import { MessageView } from "./MessageView";

/** Distance from the bottom (px) within which new content keeps the view pinned to the end. */
const STICK_THRESHOLD = 80;

export function MessageList() {
  const messages = useChat((s) => s.messages);
  const scrollRef = useRef<HTMLDivElement>(null);
  const stickToBottom = useRef(true);

  useEffect(() => {
    const el = scrollRef.current;
    if (el && stickToBottom.current) el.scrollTop = el.scrollHeight;
  }, [messages]);

  if (messages.length === 0) return <EmptyState />;

  return (
    <div
      ref={scrollRef}
      onScroll={(e) => {
        const el = e.currentTarget;
        stickToBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < STICK_THRESHOLD;
      }}
      className="flex-1 overflow-y-auto px-4 py-4"
    >
      <div className="space-y-5">
        {messages.map((message) => (
          <MessageView key={message.id} message={message} />
        ))}
      </div>
    </div>
  );
}
