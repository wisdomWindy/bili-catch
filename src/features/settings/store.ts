import { defineStore } from "pinia";
import { ref } from "vue";
import type { AppError } from "../../contracts/app-error";
import { normalizeIpcError } from "../../contracts/app-error";
import {
  SETTING_KEYS,
  type AppearanceSettings,
  type FieldSaveState,
  type SettingKey,
  type SettingsLoadStatus,
  type SettingsPatch,
  type SettingsSnapshot,
  type SettingsValues,
} from "./contracts";
import type { SettingsEffectSink } from "./effects";
import type { SettingsService } from "./service";

const SAVED_VISIBLE_MS = 1_500;
const SAFE_APPEARANCE: AppearanceSettings = { theme: "system", locale: "zh-CN" };

function initialFieldStates(): Record<SettingKey, FieldSaveState> {
  return Object.fromEntries(
    SETTING_KEYS.map((key) => [key, { status: "idle", error: null }]),
  ) as Record<SettingKey, FieldSaveState>;
}

function appearance(values: SettingsValues): AppearanceSettings {
  return { theme: values.theme, locale: values.locale };
}

export const useSettingsStore = defineStore("settings", () => {
  const status = ref<SettingsLoadStatus>("idle");
  const loadError = ref<AppError | null>(null);
  const snapshot = ref<SettingsSnapshot | null>(null);
  const values = ref<SettingsValues | null>(null);
  const fieldStates = ref(initialFieldStates());

  const desiredPatches = new Map<SettingKey, SettingsPatch>();
  const workers = new Map<SettingKey, Promise<void>>();
  const fieldGenerations = new Map<SettingKey, number>();
  const savedTimers = new Map<SettingKey, ReturnType<typeof setTimeout>>();
  let initializeGeneration = 0;
  let initializePromise: Promise<void> | undefined;

  function setFieldState(key: SettingKey, next: FieldSaveState) {
    fieldStates.value = { ...fieldStates.value, [key]: next };
  }

  function mergeVisible(incoming: SettingsSnapshot) {
    let merged: SettingsValues = { ...incoming.values };
    for (const patch of desiredPatches.values()) {
      merged = { ...merged, [patch.field]: patch.value };
    }
    values.value = merged;
  }

  function applySnapshot(incoming: SettingsSnapshot, effects: SettingsEffectSink): boolean {
    if (snapshot.value && incoming.revision < snapshot.value.revision) return false;
    mergeVisible(incoming);
    snapshot.value = incoming;
    effects.applyCommitted(incoming.values);
    if (values.value) effects.applyAppearance(appearance(values.value));
    return true;
  }

  function resetFields() {
    desiredPatches.clear();
    fieldGenerations.clear();
    for (const timer of savedTimers.values()) clearTimeout(timer);
    savedTimers.clear();
    fieldStates.value = initialFieldStates();
  }

  function beginInitialize(service: SettingsService, effects: SettingsEffectSink): Promise<void> {
    const generation = ++initializeGeneration;
    status.value = "loading";
    loadError.value = null;
    const current = (async () => {
      try {
        const incoming = await service.getSnapshot();
        if (generation !== initializeGeneration) return;
        resetFields();
        snapshot.value = null;
        values.value = null;
        applySnapshot(incoming, effects);
        status.value = "ready";
      } catch (error: unknown) {
        if (generation !== initializeGeneration) return;
        snapshot.value = null;
        values.value = null;
        loadError.value = normalizeIpcError(error);
        status.value = "load_error";
        effects.applyAppearance(SAFE_APPEARANCE);
      } finally {
        if (generation === initializeGeneration) initializePromise = undefined;
      }
    })();
    initializePromise = current;
    return current;
  }

  function initialize(service: SettingsService, effects: SettingsEffectSink): Promise<void> {
    if (status.value === "ready") return Promise.resolve();
    return initializePromise ?? beginInitialize(service, effects);
  }

  function retryInitialize(service: SettingsService, effects: SettingsEffectSink): Promise<void> {
    return beginInitialize(service, effects);
  }

  function dispose() {
    initializeGeneration += 1;
    initializePromise = undefined;
    for (const timer of savedTimers.values()) clearTimeout(timer);
    savedTimers.clear();
  }

  function updateVisible(patch: SettingsPatch) {
    if (!values.value) return;
    values.value = { ...values.value, [patch.field]: patch.value };
  }

  function preview(patch: SettingsPatch, effects: SettingsEffectSink) {
    if (status.value !== "ready" || !values.value) return;
    updateVisible(patch);
    setFieldState(patch.field, { status: "preview", error: null });
    if (patch.field === "theme" || patch.field === "locale") {
      effects.applyAppearance(appearance(values.value));
    }
  }

  function scheduleSavedReset(key: SettingKey, generation: number) {
    const previous = savedTimers.get(key);
    if (previous) clearTimeout(previous);
    savedTimers.set(key, setTimeout(() => {
      if (
        fieldGenerations.get(key) === generation
        && fieldStates.value[key].status === "saved"
      ) {
        setFieldState(key, { status: "idle", error: null });
        savedTimers.delete(key);
      }
    }, SAVED_VISIBLE_MS));
  }

  function rollbackField(key: SettingKey, effects: SettingsEffectSink) {
    if (!values.value || !snapshot.value) return;
    values.value = { ...values.value, [key]: snapshot.value.values[key] };
    if (key === "theme" || key === "locale") {
      effects.applyAppearance(appearance(values.value));
    }
  }

  async function drain(
    key: SettingKey,
    service: SettingsService,
    effects: SettingsEffectSink,
  ): Promise<void> {
    while (true) {
      const requested = desiredPatches.get(key);
      if (!requested) return;
      const generation = fieldGenerations.get(key) ?? 0;
      try {
        const incoming = await service.update(requested);
        applySnapshot(incoming, effects);
        if (desiredPatches.get(key) === requested) {
          desiredPatches.delete(key);
          setFieldState(key, { status: "saved", error: null });
          scheduleSavedReset(key, generation);
          return;
        }
      } catch (error: unknown) {
        if (
          desiredPatches.get(key) === requested
          && fieldGenerations.get(key) === generation
        ) {
          desiredPatches.delete(key);
          rollbackField(key, effects);
          setFieldState(key, { status: "error", error: normalizeIpcError(error) });
          return;
        }
      }
      setFieldState(key, { status: "saving", error: null });
    }
  }

  function commit(
    patch: SettingsPatch,
    service: SettingsService,
    effects: SettingsEffectSink,
  ): Promise<void> {
    if (status.value !== "ready" || !values.value || !snapshot.value) {
      return Promise.resolve();
    }

    const key = patch.field;
    const nextGeneration = (fieldGenerations.get(key) ?? 0) + 1;
    fieldGenerations.set(key, nextGeneration);
    const savedTimer = savedTimers.get(key);
    if (savedTimer) {
      clearTimeout(savedTimer);
      savedTimers.delete(key);
    }
    updateVisible(patch);
    if (key === "theme" || key === "locale") {
      effects.applyAppearance(appearance(values.value));
    }

    if (snapshot.value.values[key] === patch.value && !workers.has(key)) {
      desiredPatches.delete(key);
      setFieldState(key, { status: "idle", error: null });
      return Promise.resolve();
    }

    desiredPatches.set(key, patch);
    setFieldState(key, { status: "saving", error: null });
    const existing = workers.get(key);
    if (existing) return existing;

    const worker = drain(key, service, effects);
    workers.set(key, worker);
    void worker.finally(() => {
      if (workers.get(key) === worker) workers.delete(key);
    });
    return worker;
  }

  return {
    status,
    loadError,
    snapshot,
    values,
    fieldStates,
    initialize,
    retryInitialize,
    dispose,
    preview,
    commit,
    applySnapshot,
  };
});
