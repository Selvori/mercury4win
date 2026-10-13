import { create } from "zustand";

// Pause Reader interactions only while the list collects export IDs.
export const useDigestSelectionStore = create<{ selecting: boolean; set_selecting: (value: boolean) => void }>((set) => ({
  selecting: false,
  set_selecting: (selecting) => set({ selecting }),
}));
