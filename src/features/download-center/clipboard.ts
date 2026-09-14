import { readText } from "@tauri-apps/plugin-clipboard-manager";

export interface ClipboardReader {
  readText(): Promise<string>;
}

export function createTauriClipboardReader(): ClipboardReader {
  return { readText };
}
