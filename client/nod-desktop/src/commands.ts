import { invoke } from "@tauri-apps/api/core";
import type {
  ClientState,
  EnrollParams,
  NodRequest,
  NotificationPreferenceParams,
  RenameDeviceParams,
  RevokeDeviceParams,
  SelectRequestParams,
  SelectServerParams,
  SetSubscriptionParams,
  SourceParams,
  SubmitOptionParams,
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

export function selectSource(params: SourceParams): Promise<ClientState> {
  return invoke<ClientState>("select_source", { params });
}

export function selectRequest(params: SelectRequestParams): Promise<ClientState> {
  return invoke<ClientState>("select_request", { params });
}

export function submitOption(params: SubmitOptionParams): Promise<NodRequest> {
  return invoke<NodRequest>("submit_option", { params });
}

export function clearSource(params: SourceParams): Promise<ClientState> {
  return invoke<ClientState>("clear_source", { params });
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
