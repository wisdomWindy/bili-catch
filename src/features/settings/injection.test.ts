import { mount } from "@vue/test-utils";
import { defineComponent } from "vue";
import { describe, expect, it } from "vitest";
import {
  useDirectoryPicker,
  useReleaseActions,
  useSettingsService,
} from "./injection";
import type { DirectoryPickerPort } from "./dialog";
import type { ReleaseActionsPort } from "./release";
import type { SettingsService } from "./service";

describe("settings injection fallbacks", () => {
  it("returns async unavailable adapters instead of throwing during setup", async () => {
    let service: SettingsService | undefined;
    let picker: DirectoryPickerPort | undefined;
    let release: ReleaseActionsPort | undefined;
    mount(defineComponent({
      setup() {
        service = useSettingsService();
        picker = useDirectoryPicker();
        release = useReleaseActions();
        return () => null;
      },
    }));
    if (!service || !picker || !release) throw new Error("fallback adapters were not captured");

    await expect(service.getSnapshot()).rejects.toMatchObject({
      details: "SETTINGS_SERVICE_UNAVAILABLE",
    });
    await expect(picker.selectDirectory("D:/Current")).rejects.toMatchObject({
      details: "DIRECTORY_PICKER_FAILED",
    });
    await expect(release.checkForUpdates()).rejects.toMatchObject({
      details: "UPDATE_NOT_CONFIGURED",
    });
  });
});
