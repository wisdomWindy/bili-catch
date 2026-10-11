import type { InjectionKey } from "vue";
import type { TranscriptionEventSource } from "./events";
import type { TranscriptionService } from "./service";

export const transcriptionServiceKey: InjectionKey<TranscriptionService> = Symbol("transcription-service");
export const transcriptionEventSourceKey: InjectionKey<TranscriptionEventSource> = Symbol("transcription-event-source");
