import { create } from "zustand";
import type { WebState } from "../lib/ipc";

/** Mirror of the web tab's page state, kept in sync by `orbit://web-state` events. */
export const useWeb = create<WebState>()(() => ({ url: null, title: "", loading: false }));
