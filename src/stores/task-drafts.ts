import { defineStore } from "pinia";
import type { DownloadTaskDraft } from "../features/download-center/contracts";

interface TaskDraftsState {
  items: DownloadTaskDraft[];
  handoffId: string | null;
}

let handoffSequence = 0;

export const useTaskDraftsStore = defineStore("task-drafts", {
  state: (): TaskDraftsState => ({ items: [], handoffId: null }),
  actions: {
    append(drafts: DownloadTaskDraft[]) {
      if (this.items.length === 0) {
        handoffSequence += 1;
        this.handoffId = `handoff_${Date.now().toString(36)}_${handoffSequence}`;
      }
      this.items.push(...drafts);
    },
    clear() {
      this.items = [];
      this.handoffId = null;
    },
    clearIfMatches(handoffId: string) {
      if (this.handoffId === handoffId) this.clear();
    },
  },
});
