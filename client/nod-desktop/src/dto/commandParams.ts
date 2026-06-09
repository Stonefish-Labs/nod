export interface EnrollParams {
  base_url: string;
  device_name: string;
  code: string;
  notification_sound?: string | null;
}

export interface SubmitOptionParams {
  request_id: string;
  option_id: string;
  text?: string | null;
}

export interface ChannelParams {
  channel_id: string;
}

export interface SelectServerParams {
  server_id: string;
}

export interface SelectRequestParams {
  request_id: string;
}

export interface SetSubscriptionParams {
  channel_id: string;
  subscribed: boolean;
}

export interface NotificationPreferenceParams {
  notification_sound: string;
}

export interface RenameDeviceParams {
  device_id: string;
  name: string;
}

export interface RevokeDeviceParams {
  device_id: string;
}
