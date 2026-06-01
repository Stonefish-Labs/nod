import type { ClientState, NodEvent } from "./models";

export type RuntimeEvent =
  | { event: "ready"; payload: { state_path: string } }
  | { event: "state"; payload: ClientState }
  | { event: "notification_candidate"; payload: { event: NodEvent } }
  | { event: "notification_removed"; payload: { event_id: string } }
  | { event: "sync_status"; payload: { connected: boolean } }
  | { event: "auth_revoked"; payload: Record<string, never> }
  | { event: "resync_required"; payload: Record<string, never> }
  | { event: "transient_error"; payload: { message: string } };
