import { AnimatePresence, motion } from "motion/react";
import { CompactBar } from "./components/shell/CompactBar";
import { Panel } from "./components/shell/Panel";
import {
  useChatRestore,
  useConfigSync,
  useKeyboardShortcuts,
  useNativeContextMenu,
  useScreenshotSync,
  useShortcutActions,
  useSelectionSync,
  useWebSync,
} from "./hooks/useAppLifecycle";
import { useUi } from "./store/ui";

export function App() {
  const mode = useUi((s) => s.mode);

  useConfigSync();
  useChatRestore();
  useWebSync();
  useSelectionSync();
  useScreenshotSync();
  useShortcutActions();
  useKeyboardShortcuts();
  useNativeContextMenu();

  return (
    <main className="h-full w-full overflow-hidden bg-surface text-fg">
      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          // The web tab is the same panel, only wider: no fade between them.
          key={mode === "compact" ? "compact" : "panel"}
          className="h-full"
          initial={{ opacity: 0, scale: 0.985 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.14, ease: "easeOut" }}
        >
          {mode === "compact" ? <CompactBar /> : <Panel />}
        </motion.div>
      </AnimatePresence>
    </main>
  );
}
