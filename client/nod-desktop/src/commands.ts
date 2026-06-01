import { invoke } from "@tauri-apps/api/core";
import type {
  ChannelParams,
  ClientState,
  EnrollParams,
  NodEvent,
  NotificationPreferenceParams,
  RenameDeviceParams,
  RevokeDeviceParams,
  SelectEventParams,
  SelectServerParams,
  SetSubscriptionParams,
  SubmitActionParams,
  UserDevice,
} from "./types";

export function getState(): Promise<ClientState> {
  return invoke<ClientState>("state");
}

export function enroll(params: EnrollParams): Promise<ClientState> {
  return invoke<ClientState>("enroll", { params });
}

export function refresh(): Promise<ClientState> {
  return invoke<ClientState>("refresh");
}

export function selectServer(params: SelectServerParams): Promise<ClientState> {
  return invoke<ClientState>("select_server", { params });
}

export function forgetServer(params: SelectServerParams): Promise<ClientState> {
  return invoke<ClientState>("forget_server", { params });
}

export function selectChannel(params: ChannelParams): Promise<ClientState> {
  return invoke<ClientState>("select_channel", { params });
}

export function selectEvent(params: SelectEventParams): Promise<ClientState> {
  return invoke<ClientState>("select_event", { params });
}

export function submitAction(params: SubmitActionParams): Promise<NodEvent> {
  return invoke<NodEvent>("submit_action", { params });
}

export function clearChannel(params: ChannelParams): Promise<ClientState> {
  return invoke<ClientState>("clear_channel", { params });
}

export function setSubscription(params: SetSubscriptionParams): Promise<ClientState> {
  return invoke<ClientState>("set_subscription", { params });
}

export function setNotificationPreference(
  params: NotificationPreferenceParams,
): Promise<ClientState> {
  return invoke<ClientState>("set_notification_preference", { params });
}

export function listDevices(): Promise<UserDevice[]> {
  return invoke<UserDevice[]>("list_devices");
}

export function renameDevice(params: RenameDeviceParams): Promise<UserDevice> {
  return invoke<UserDevice>("rename_device", { params });
}

export function revokeDevice(params: RevokeDeviceParams): Promise<ClientState> {
  return invoke<ClientState>("revoke_device", { params });
}

export function openExternalUrl(url: string): Promise<void> {
  return invoke<void>("open_external_url", { url });
}
