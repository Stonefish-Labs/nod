import { useEffect, useMemo, useState } from "react";
import {
  clearChannel,
  enroll,
  forgetServer,
  getState,
  listDevices,
  openExternalUrl,
  refresh,
  renameDevice,
  revokeDevice,
  selectChannel,
  selectEvent,
  selectServer,
  setNotificationPreference,
  setSubscription,
  submitAction,
} from "../commands";
import { listenForRuntimeEvents } from "../events";
import { selectedChannel, selectedEvent } from "../domain";
import type {
  Channel,
  ClientState,
  EnrollParams,
  EventAction,
  NodEvent,
  RuntimeEvent,
  ServerProfile,
  UserDevice,
} from "../types";
import { EMPTY_CLIENT_STATE } from "./state";

export interface DesktopClientActions {
  clearError: () => void;
  closeSettings: () => void;
  clearSelectedChannel: () => Promise<void>;
  enrollDevice: (params: EnrollParams) => Promise<void>;
  forgetSelectedServer: () => Promise<void>;
  openSettings: () => void;
  openUrl: (url: string) => Promise<void>;
  refreshState: () => Promise<void>;
  renameUserDevice: (deviceId: string, name: string) => Promise<boolean>;
  revokeUserDevice: (deviceId: string) => Promise<void>;
  selectChannel: (channel: Channel) => Promise<void>;
  selectEvent: (event: NodEvent) => Promise<void>;
  selectServer: (server: ServerProfile) => Promise<void>;
  submitEventAction: (
    event: NodEvent,
    action: EventAction,
    text?: string,
  ) => Promise<void>;
  toggleChannelSubscription: (channel: Channel) => Promise<void>;
  updateNotificationSound: (notificationSound: string) => Promise<void>;
}

export interface DesktopClient {
  activeChannel?: Channel;
  activeEvent?: NodEvent;
  actions: DesktopClientActions;
  devices: UserDevice[];
  error: string | null;
  isLoading: boolean;
  settingsOpen: boolean;
  state: ClientState;
}

export function useDesktopClient(): DesktopClient {
  const [state, setState] = useState<ClientState>(EMPTY_CLIENT_STATE);
  const [isLoading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [devices, setDevices] = useState<UserDevice[]>([]);

  useEffect(() => {
    let cancelled = false;
    let stopListening: (() => void) | undefined;

    getState()
      .then((loaded) => {
        if (!cancelled) {
          setState(loaded);
        }
      })
      .catch((reason: unknown) => setError(String(reason)))
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    listenForRuntimeEvents((runtimeEvent) => {
      if (!cancelled) {
        applyRuntimeEvent(runtimeEvent, setState, setError);
      }
    })
      .then((unlisten) => {
        // Tauri resolves the listener cleanup asynchronously, so unlisten if React unmounted first.
        if (cancelled) {
          unlisten();
          return;
        }
        stopListening = unlisten;
      })
      .catch((reason: unknown) => setError(String(reason)));

    return () => {
      cancelled = true;
      stopListening?.();
    };
  }, []);

  useEffect(() => {
    if (!settingsOpen) {
      return;
    }
    listDevices()
      .then(setDevices)
      .catch((reason: unknown) => setError(String(reason)));
  }, [settingsOpen]);

  const activeChannel = useMemo(() => selectedChannel(state), [state]);
  const activeEvent = useMemo(() => selectedEvent(state), [state]);

  async function runStateCommand(work: () => Promise<ClientState>): Promise<boolean> {
    try {
      setError(null);
      setState(await work());
      return true;
    } catch (reason) {
      setError(String(reason));
      return false;
    }
  }

  async function enrollDevice(params: EnrollParams): Promise<void> {
    await runStateCommand(() => enroll(params));
  }

  async function refreshState(): Promise<void> {
    await runStateCommand(refresh);
  }

  async function selectServerProfile(server: ServerProfile): Promise<void> {
    await runStateCommand(() => selectServer({ server_id: server.id }));
  }

  async function selectNotificationChannel(channel: Channel): Promise<void> {
    await runStateCommand(() => selectChannel({ channel_id: channel.id }));
  }

  async function selectNotificationEvent(event: NodEvent): Promise<void> {
    await runStateCommand(() => selectEvent({ event_id: event.id }));
  }

  async function submitEventAction(
    event: NodEvent,
    action: EventAction,
    text?: string,
  ): Promise<void> {
    try {
      setError(null);
      const updated = await submitAction({
        event_id: event.id,
        action_id: action.id,
        text,
      });
      setState((current) => ({
        ...current,
        events: current.events.map((candidate) =>
          candidate.id === updated.id ? updated : candidate,
        ),
      }));
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function openUrl(url: string): Promise<void> {
    try {
      setError(null);
      await openExternalUrl(url);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function updateNotificationSound(notificationSound: string): Promise<void> {
    await runStateCommand(() =>
      setNotificationPreference({ notification_sound: notificationSound }),
    );
  }

  async function toggleChannelSubscription(channel: Channel): Promise<void> {
    await runStateCommand(() =>
      setSubscription({
        channel_id: channel.id,
        subscribed: !channel.subscribed,
      }),
    );
  }

  async function renameUserDevice(
    deviceId: string,
    name: string,
  ): Promise<boolean> {
    const trimmedName = name.trim();
    if (trimmedName.length === 0) {
      return false;
    }
    try {
      setError(null);
      const device = await renameDevice({ device_id: deviceId, name: trimmedName });
      setDevices((current) =>
        current.map((candidate) => (candidate.id === device.id ? device : candidate)),
      );
      return true;
    } catch (reason) {
      setError(String(reason));
      return false;
    }
  }

  async function revokeUserDevice(deviceId: string): Promise<void> {
    try {
      setError(null);
      setState(await revokeDevice({ device_id: deviceId }));
      setDevices(await listDevices());
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function forgetSelectedServer(): Promise<void> {
    const serverId = state.selected_server_id;
    if (!serverId) {
      return;
    }
    if (await runStateCommand(() => forgetServer({ server_id: serverId }))) {
      setSettingsOpen(false);
    }
  }

  async function clearSelectedChannel(): Promise<void> {
    const channelId = state.selected_channel_id;
    if (!channelId) {
      return;
    }
    await runStateCommand(() => clearChannel({ channel_id: channelId }));
  }

  return {
    activeChannel,
    activeEvent,
    actions: {
      clearError: () => setError(null),
      clearSelectedChannel,
      closeSettings: () => setSettingsOpen(false),
      enrollDevice,
      forgetSelectedServer,
      openSettings: () => setSettingsOpen(true),
      openUrl,
      refreshState,
      renameUserDevice,
      revokeUserDevice,
      selectChannel: selectNotificationChannel,
      selectEvent: selectNotificationEvent,
      selectServer: selectServerProfile,
      submitEventAction,
      toggleChannelSubscription,
      updateNotificationSound,
    },
    devices,
    error,
    isLoading,
    settingsOpen,
    state,
  };
}

function applyRuntimeEvent(
  runtimeEvent: RuntimeEvent,
  setState: (state: ClientState) => void,
  setError: (message: string | null) => void,
): void {
  switch (runtimeEvent.event) {
    case "state":
      setState(runtimeEvent.payload);
      setError(runtimeEvent.payload.last_error ?? null);
      break;
    case "transient_error":
      setError(runtimeEvent.payload.message);
      break;
    case "auth_revoked":
      setError("This device registration was revoked.");
      break;
    default:
      break;
  }
}
