// These DTOs intentionally mirror Rust serde payloads, so field names stay snake_case here.
export type DevicePlatform =
  | "ios"
  | "macos"
  | "watchos"
  | "windows"
  | "linux"
  | "unknown";

export type NotificationDeliveryMode = "push" | "websocket";
export type RequestStatus = "pending" | "resolved" | "expired" | "cancelled";
export type OptionKind =
  | "approve"
  | "approve_with_text"
  | "reject"
  | "reject_with_text"
  | "dismiss"
  | "open"
  | "custom";

export interface ServerProfile {
  id: string;
  name: string;
  base_url_string: string;
  device_name: string;
  device_id?: string | null;
  user_id?: string | null;
  user_name?: string | null;
}

export interface User {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface UserDevice {
  id: string;
  user_id: string;
  name: string;
  platform: DevicePlatform;
  native_app_id?: string | null;
  push_provider?: string | null;
  has_push_token: boolean;
  notification_sound: string;
  last_seen_at: string;
  created_at: string;
  is_current: boolean;
}

export interface Source {
  id: string;
  name: string;
  emoji: string;
  subscribed: boolean;
  created_at: string;
}

export interface RequestField {
  label: string;
  value: string;
  style?: string | null;
}

export interface RequestLink {
  label: string;
  url: string;
}

export interface RequestOption {
  id: string;
  label: string;
  kind: OptionKind;
  style: string;
  requires_text: boolean;
  text_placeholder?: string | null;
  destructive: boolean;
  foreground: boolean;
}

export interface Decision {
  request_id: string;
  option_id: string;
  option_kind: OptionKind;
  option_label: string;
  text?: string | null;
  actor_user_id?: string | null;
  actor_device_id?: string | null;
  resolved_at: string;
}

export interface UserDecision {
  user_id: string;
  decision: Decision;
}

export interface NodRequest {
  id: string;
  request_id: string;
  source_id: string;
  recipients: string[];
  decision_resolution: "shared" | "per_user";
  title: string;
  summary: string;
  body_markdown: string;
  fields: RequestField[];
  links: RequestLink[];
  image_url?: string | null;
  notification: RequestNotification;
  dedupe_key?: string | null;
  expires_at?: string | null;
  status: RequestStatus;
  created_at: string;
  updated_at: string;
  resolved_at?: string | null;
  decision?: Decision | null;
  decisions: UserDecision[];
  callback_url?: string | null;
  options: RequestOption[];
  request_digest?: string | null;
}

export interface RequestNotification {
  redact: boolean;
  title?: string | null;
  body?: string | null;
}

export interface ClientState {
  servers: ServerProfile[];
  selected_server_id?: string | null;
  current_user?: User | null;
  devices: UserDevice[];
  sources: Source[];
  pending_counts_by_source: Record<string, number>;
  requests: NodRequest[];
  selected_source_id?: string | null;
  selected_request_id?: string | null;
  notification_sound: string;
  notification_delivery_mode: NotificationDeliveryMode;
  is_registered: boolean;
  is_sync_connected: boolean;
  last_error?: string | null;
}
