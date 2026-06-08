import type { ClientState } from "../types";

export const EMPTY_CLIENT_STATE: ClientState = {
  servers: [],
  selected_server_id: null,
  current_user: null,
  devices: [],
  sources: [],
  pending_counts_by_source: {},
  requests: [],
  selected_source_id: null,
  selected_request_id: null,
  notification_sound: "default",
  notification_delivery_mode: "websocket",
  is_registered: false,
  is_sync_connected: false,
  last_error: null,
};

export const NOTIFICATION_SOUND_OPTIONS = [
  { id: "default", label: "Default" },
  { id: "nod_ping.wav", label: "Ping" },
  { id: "nod_chime.wav", label: "Chime" },
  { id: "nod_low.wav", label: "Low" },
  { id: "silent", label: "Silent" },
] as const;
