import { defineStore } from "pinia";
import type { AppError } from "../../contracts/app-error";
import { normalizeIpcError } from "../../contracts/app-error";
import type { AuthSnapshot, AuthStateEvent } from "./contracts";
import type { AuthEventSource } from "./events";
import type { AuthService } from "./service";

export type AuthLoadStatus = "idle" | "subscribing" | "loading" | "ready" | "failed";
type AuthAction = "start" | "cancel" | "logout";

interface AuthState {
  snapshot: AuthSnapshot;
  status: AuthLoadStatus;
  error: AppError | null;
  eventError: AppError | null;
  pending: Record<AuthAction, boolean>;
  service: AuthService | null;
  unsubscribe: (() => void) | null;
  lifecycle: number;
  destroyed: boolean;
}

const initialSnapshot = (): AuthSnapshot => ({
  revision: 0,
  status: "restoring",
  qrContent: null,
  expiresAt: null,
  account: null,
  error: null,
});

const initialPending = (): Record<AuthAction, boolean> => ({
  start: false,
  cancel: false,
  logout: false,
});

export const useAuthStore = defineStore("authentication", {
  state: (): AuthState => ({
    snapshot: initialSnapshot(),
    status: "idle",
    error: null,
    eventError: null,
    pending: initialPending(),
    service: null,
    unsubscribe: null,
    lifecycle: 0,
    destroyed: false,
  }),
  actions: {
    async initialize(service: AuthService, eventSource: AuthEventSource) {
      this.unsubscribe?.();
      this.unsubscribe = null;
      this.lifecycle += 1;
      const lifecycle = this.lifecycle;
      this.destroyed = false;
      this.service = service;
      this.status = "subscribing";
      this.error = null;
      this.eventError = null;
      this.pending = initialPending();
      const buffered: AuthStateEvent[] = [];
      let hydrating = true;
      const onState = (event: AuthStateEvent) => {
        if (this.lifecycle !== lifecycle || this.destroyed) return;
        if (hydrating) buffered.push(event);
        else this.applyEvent(event);
      };

      try {
        const unsubscribe = await eventSource.subscribe(onState);
        if (this.lifecycle !== lifecycle || this.destroyed) {
          unsubscribe();
          return;
        }
        this.unsubscribe = unsubscribe;
      } catch (error: unknown) {
        if (this.lifecycle === lifecycle && !this.destroyed) {
          this.eventError = normalizeIpcError(error);
        }
      }

      if (this.lifecycle !== lifecycle || this.destroyed) return;
      this.status = "loading";
      try {
        const next = await service.getSnapshot();
        if (this.lifecycle !== lifecycle || this.destroyed) return;
        this.mergeCommandSnapshot(next);
        hydrating = false;
        buffered.forEach((event) => this.applyEvent(event));
        this.status = "ready";
      } catch (error: unknown) {
        hydrating = false;
        if (this.lifecycle !== lifecycle || this.destroyed) return;
        buffered.forEach((event) => this.applyEvent(event));
        this.error = normalizeIpcError(error);
        this.status = "failed";
      }
    },
    applyEvent(event: AuthStateEvent) {
      if (this.destroyed || event.snapshot.revision <= this.snapshot.revision) return;
      this.snapshot = event.snapshot;
    },
    mergeCommandSnapshot(next: AuthSnapshot) {
      if (this.destroyed || next.revision < this.snapshot.revision) return;
      this.snapshot = next;
    },
    async start() {
      await this.runAction("start");
    },
    async cancel() {
      await this.runAction("cancel");
    },
    async logout() {
      await this.runAction("logout");
    },
    async runAction(action: AuthAction) {
      if (!this.service || this.pending[action]) return;
      const lifecycle = this.lifecycle;
      this.pending[action] = true;
      this.error = null;
      try {
        const next = await this.service[action]();
        if (this.lifecycle === lifecycle && !this.destroyed) {
          this.mergeCommandSnapshot(next);
        }
      } catch (error: unknown) {
        if (this.lifecycle === lifecycle && !this.destroyed) {
          this.error = normalizeIpcError(error);
        }
      } finally {
        if (this.lifecycle === lifecycle) this.pending[action] = false;
      }
    },
    dispose() {
      this.lifecycle += 1;
      this.destroyed = true;
      this.pending = initialPending();
      this.unsubscribe?.();
      this.unsubscribe = null;
    },
  },
});
