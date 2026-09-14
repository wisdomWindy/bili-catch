import type { InjectionKey } from "vue";
import { inject } from "vue";
import type { ParseVideoService } from "./service";
import type { ClipboardReader } from "./clipboard";

export const parseVideoServiceKey: InjectionKey<ParseVideoService> = Symbol("parse-video-service");
export const clipboardReaderKey: InjectionKey<ClipboardReader> = Symbol("clipboard-reader");
export interface DownloadNotifier {
  success(message: string): void;
  info(message: string): void;
}
export const downloadNotifierKey: InjectionKey<DownloadNotifier> = Symbol("download-notifier");

const unavailableService: ParseVideoService = {
  async parseVideo() {
    throw { code: "E007", message: "The local parsing service is unavailable" };
  },
};

export function useParseVideoService(): ParseVideoService {
  return inject(parseVideoServiceKey, unavailableService);
}

export function useClipboardReader(): ClipboardReader {
  return inject(clipboardReaderKey, {
    readText: async () => "",
  });
}

export function useDownloadNotifier(): DownloadNotifier {
  return inject(downloadNotifierKey, {
    success: () => undefined,
    info: () => undefined,
  });
}
