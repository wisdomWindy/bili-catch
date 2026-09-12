import type { AppError } from "../../contracts/app-error";

export interface DirectoryPickerPort {
  selectDirectory(initialPath: string): Promise<string | null>;
}

function pickerError(): AppError {
  return {
    code: "E_INTERNAL",
    message: "Directory picker is unavailable",
    details: "DIRECTORY_PICKER_FAILED",
  };
}

export function createTauriDirectoryPicker(): DirectoryPickerPort {
  return {
    async selectDirectory(initialPath) {
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const selected = await open({
          directory: true,
          multiple: false,
          defaultPath: initialPath,
        });
        if (selected === null || typeof selected === "string") return selected;
        throw pickerError();
      } catch {
        throw pickerError();
      }
    },
  };
}
