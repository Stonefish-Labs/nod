import type { ClientState, NodRequest } from "./models";

export type RuntimeMessage =
  | { kind: "ready"; payload: { state_path: string } }
  | { kind: "state"; payload: ClientState }
  | { kind: "notification_candidate"; payload: { request: NodRequest } }
  | { kind: "notification_removed"; payload: { request_id: string } }
  | { kind: "sync_status"; payload: { connected: boolean } }
  | { kind: "auth_revoked"; payload: Record<string, never> }
  | { kind: "resync_required"; payload: Record<string, never> }
  | { kind: "transient_error"; payload: { message: string } };
