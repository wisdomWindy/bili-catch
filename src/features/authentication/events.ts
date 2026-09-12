import { listen as tauriListen } from "@tauri-apps/api/event";
import type { AuthStateEvent } from "./contracts";

type Unsubscribe = () => void;
type EventEnvelope<T> = { payload: T };
type Listen = <T>(
  event: string,
  handler: (event: EventEnvelope<T>) => void,
) => Promise<Unsubscribe>;

export interface AuthEventSource {
  subscribe(onState: (event: AuthStateEvent) => void): Promise<Unsubscribe>;
}

export function createAuthEventSource(
  listen: Listen = tauriListen as Listen,
): AuthEventSource {
  return {
    subscribe: (onState) =>
      listen<AuthStateEvent>("auth://state", ({ payload }) => onState(payload)),
  };
}
