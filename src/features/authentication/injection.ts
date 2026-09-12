import { inject, type InjectionKey } from "vue";
import type { AuthEventSource } from "./events";
import type { AuthService } from "./service";

export const authServiceKey: InjectionKey<AuthService> = Symbol("auth-service");
export const authEventSourceKey: InjectionKey<AuthEventSource> = Symbol("auth-event-source");

const unavailableService: AuthService = {
  getSnapshot: async () => { throw { code: "E007", message: "Authentication is unavailable" }; },
  start: async () => { throw { code: "E007", message: "Authentication is unavailable" }; },
  cancel: async () => { throw { code: "E007", message: "Authentication is unavailable" }; },
  logout: async () => { throw { code: "E007", message: "Authentication is unavailable" }; },
};

const unavailableEvents: AuthEventSource = {
  subscribe: async () => () => undefined,
};

export function useAuthService(): AuthService {
  return inject(authServiceKey, unavailableService);
}

export function useAuthEventSource(): AuthEventSource {
  return inject(authEventSourceKey, unavailableEvents);
}
