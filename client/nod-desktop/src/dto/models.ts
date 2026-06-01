// These DTOs intentionally mirror Rust serde payloads, so field names stay snake_case here.
export type DevicePlatform =
  | "ios"
  | "macos"
  | "watchos"
  | "windows"
  | "linux"
  | "unknown";

export type NotificationDeliveryMode = "push" | "websocket";
export type EventStatus = "pending" | "resolved" | "expired" | "cancelled";
export type ActionKind =
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

export interface Channel {
  id: string;
  name: string;
  icon: string;
  color: string;
  default_priority: number;
  privacy: string;
  subscribed: boolean;
  created_at: string;
}

export interface EventField {
  label: string;
  value: string;
  style?: string | null;
}

export interface EventLink {
  label: string;
  url: string;
}

export interface EventAction {
  id: string;
  label: string;
  kind: ActionKind;
  style: string;
  requires_text: boolean;
  text_placeholder?: string | null;
  destructive: boolean;
  foreground: boolean;
}

export interface EventResult {
  event_id: string;
  action_id: string;
  action_kind: ActionKind;
  action_label: string;
  text?: string | null;
  actor_user_id?: string | null;
  actor_device_id?: string | null;
  resolved_at: string;
}

export interface EventUserResult {
  user_id: string;
  result: EventResult;
}

export interface NodEvent {
  id: string;
  channel_id: string;
  recipients: string[];
  action_resolution: "shared" | "per_user";
  title: string;
  summary: string;
  body_markdown: string;
  fields: EventField[];
  links: EventLink[];
  image_url?: string | null;
  priority: number;
  privacy: string;
  dedupe_key?: string | null;
  expires_at?: string | null;
  status: EventStatus;
  created_at: string;
  updated_at: string;
  resolved_at?: string | null;
  result?: EventResult | null;
  user_results: EventUserResult[];
  callback_url?: string | null;
  actions: EventAction[];
  request_digest?: string | null;
}

export interface ClientState {
  servers: ServerProfile[];
  selected_server_id?: string | null;
  current_user?: User | null;
  devices: UserDevice[];
  channels: Channel[];
  pending_counts_by_channel: Record<string, number>;
  events: NodEvent[];
  selected_channel_id?: string | null;
  selected_event_id?: string | null;
  notification_sound: string;
  notification_delivery_mode: NotificationDeliveryMode;
  is_registered: boolean;
  is_sync_connected: boolean;
  last_error?: string | null;
}
