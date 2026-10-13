import { create } from "zustand";

interface NoteSaveSnapshot {
  entry_id: number;
  markdown: string;
  revision: number;
}

interface NoteDraftState {
  drafts: Record<number, string>;
  revisions: Record<number, number>;
  pending_saves: Record<number, number>;
  save_errors: Record<number, string>;
  set_draft: (entry_id: number, markdown: string) => void;
  discard_draft: (entry_id: number) => void;
  begin_save: (entry_id: number) => NoteSaveSnapshot | undefined;
  finish_save: (snapshot: NoteSaveSnapshot) => void;
  fail_save: (snapshot: NoteSaveSnapshot, error: string) => void;
}

// Unsaved notes belong to an article for this app session. Closing a panel or
// selecting another article keeps them; saving and cancelling remove them.
export const useNoteDraftStore = create<NoteDraftState>((set, get) => ({
  drafts: {},
  revisions: {},
  pending_saves: {},
  save_errors: {},
  set_draft: (entry_id, markdown) => set((state) => ({
    drafts: { ...state.drafts, [entry_id]: markdown },
    revisions: { ...state.revisions, [entry_id]: (state.revisions[entry_id] ?? 0) + 1 },
  })),
  discard_draft: (entry_id) => set((state) => {
    const drafts = { ...state.drafts };
    const save_errors = { ...state.save_errors };
    delete drafts[entry_id];
    delete save_errors[entry_id];
    return { drafts, save_errors };
  }),
  begin_save: (entry_id) => {
    const state = get();
    if (state.drafts[entry_id] === undefined || state.pending_saves[entry_id] !== undefined) return;
    const snapshot = { entry_id, markdown: state.drafts[entry_id], revision: state.revisions[entry_id] };
    set((current) => {
      const save_errors = { ...current.save_errors };
      delete save_errors[entry_id];
      return { pending_saves: { ...current.pending_saves, [entry_id]: snapshot.revision }, save_errors };
    });
    return snapshot;
  },
  finish_save: ({ entry_id, markdown, revision }) => set((state) => {
    if (state.pending_saves[entry_id] !== revision) return state;
    const pending_saves = { ...state.pending_saves };
    delete pending_saves[entry_id];
    // An earlier save must not discard edits made while that save was pending.
    if (state.revisions[entry_id] !== revision || state.drafts[entry_id] !== markdown) return { pending_saves };
    const drafts = { ...state.drafts };
    delete drafts[entry_id];
    return { drafts, pending_saves };
  }),
  fail_save: ({ entry_id, revision }, error) => set((state) => {
    if (state.pending_saves[entry_id] !== revision) return state;
    const pending_saves = { ...state.pending_saves };
    delete pending_saves[entry_id];
    return { pending_saves, save_errors: { ...state.save_errors, [entry_id]: error } };
  }),
}));
