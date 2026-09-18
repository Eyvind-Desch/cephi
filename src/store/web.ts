import { create } from "zustand";
import type { WebState } from "../lib/ipc";

interface WebStore extends WebState {
  /** Bumped to put the cursor into the address bar (Ctrl+L inside the page). */
  addressFocus: number;
}

/** Mirror of the web tab's page state, kept in sync by `cephi://web-state` events. */
export const useWeb = create<WebStore>()(() => ({ url: null, title: "", loading: false, addressFocus: 0 }));
