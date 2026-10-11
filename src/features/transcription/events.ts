import { listen as tauriListen } from "@tauri-apps/api/event";
import type {
  ModelDownloadProgressEvent,
  TranscriptionProgressEvent,
  TranscriptionRemovedEvent,
} from "./contracts";

type Unsubscribe = () => void;
type EventEnvelope<T> = { payload: T };
type Listen = <T>(event: string, handler: (event: EventEnvelope<T>) => void) => Promise<Unsubscribe>;

export interface TranscriptionEventSource {
  subscribe(
    onProgress: (event: TranscriptionProgressEvent) => void,
    onRemoved: (event: TranscriptionRemovedEvent) => void,
    onModelProgress: (event: ModelDownloadProgressEvent) => void,
  ): Promise<Unsubscribe>;
}

export function createTranscriptionEventSource(
  listen: Listen = tauriListen as Listen,
): TranscriptionEventSource {
  return {
    async subscribe(onProgress, onRemoved, onModelProgress) {
      const unsubscribers: Unsubscribe[] = [];
      try {
        unsubscribers.push(await listen("transcribe://progress", ({ payload }) => onProgress(payload as TranscriptionProgressEvent)));
        unsubscribers.push(await listen("transcribe://removed", ({ payload }) => onRemoved(payload as TranscriptionRemovedEvent)));
        unsubscribers.push(await listen("transcribe://model-progress", ({ payload }) => onModelProgress(payload as ModelDownloadProgressEvent)));
      } catch (error) {
        unsubscribers.forEach((unsubscribe) => unsubscribe());
        throw error;
      }
      return () => unsubscribers.forEach((unsubscribe) => unsubscribe());
    },
  };
}
