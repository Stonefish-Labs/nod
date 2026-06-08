import { useEffect, useMemo, useState } from "react";
import {
  clearSource,
  enroll,
  forgetServer,
  getState,
  listDevices,
  openExternalUrl,
  refresh,
  renameDevice,
  revokeDevice,
  selectSource,
  selectRequest,
  selectServer,
  setNotificationPreference,
  setSubscription,
  submitOption,
} from "../commands";
import { listenForRuntimeMessages } from "../events";
import { selectedSource, selectedRequest } from "../domain";
import type {
  Source,
  ClientState,
  EnrollParams,
  RequestOption,
  NodRequest,
  RuntimeMessage,
  ServerProfile,
  UserDevice,
} from "../types";
import { EMPTY_CLIENT_STATE } from "./state";

export interface DesktopClientCommands {
  clearError: () => void;
  closeSettings: () => void;
  clearSelectedSource: () => Promise<void>;
  enrollDevice: (params: EnrollParams) => Promise<void>;
  forgetSelectedServer: () => Promise<void>;
  openSettings: () => void;
  openUrl: (url: string) => Promise<void>;
  refreshState: () => Promise<void>;
  renameUserDevice: (deviceId: string, name: string) => Promise<boolean>;
  revokeUserDevice: (deviceId: string) => Promise<void>;
  selectSource: (source: Source) => Promise<void>;
  selectRequest: (request: NodRequest) => Promise<void>;
  selectServer: (server: ServerProfile) => Promise<void>;
  submitRequestOption: (
    request: NodRequest,
    option: RequestOption,
    text?: string,
  ) => Promise<void>;
  toggleSourceSubscription: (source: Source) => Promise<void>;
  updateNotificationSound: (notificationSound: string) => Promise<void>;
}

export interface DesktopClient {
  activeSource?: Source;
  activeRequest?: NodRequest;
  commands: DesktopClientCommands;
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

    listenForRuntimeMessages((runtimeMessage) => {
      if (!cancelled) {
        applyRuntimeMessage(runtimeMessage, setState, setError);
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

  const activeSource = useMemo(() => selectedSource(state), [state]);
  const activeRequest = useMemo(() => selectedRequest(state), [state]);

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

  async function selectNodSource(source: Source): Promise<void> {
    await runStateCommand(() => selectSource({ source_id: source.id }));
  }

  async function selectNodRequest(request: NodRequest): Promise<void> {
    await runStateCommand(() => selectRequest({ request_id: request.id }));
  }

  async function submitRequestOption(
    request: NodRequest,
    option: RequestOption,
    text?: string,
  ): Promise<void> {
    try {
      setError(null);
      const updated = await submitOption({
        request_id: request.id,
        option_id: option.id,
        text,
      });
      setState((current) => ({
        ...current,
        requests: current.requests.map((candidate) =>
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

  async function toggleSourceSubscription(source: Source): Promise<void> {
    await runStateCommand(() =>
      setSubscription({
        source_id: source.id,
        subscribed: !source.subscribed,
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

  async function clearSelectedSource(): Promise<void> {
    const sourceId = state.selected_source_id;
    if (!sourceId) {
      return;
    }
    await runStateCommand(() => clearSource({ source_id: sourceId }));
  }

  return {
    activeSource,
    activeRequest,
    commands: {
      clearError: () => setError(null),
      clearSelectedSource,
      closeSettings: () => setSettingsOpen(false),
      enrollDevice,
      forgetSelectedServer,
      openSettings: () => setSettingsOpen(true),
      openUrl,
      refreshState,
      renameUserDevice,
      revokeUserDevice,
      selectSource: selectNodSource,
      selectRequest: selectNodRequest,
      selectServer: selectServerProfile,
      submitRequestOption,
      toggleSourceSubscription,
      updateNotificationSound,
    },
    devices,
    error,
    isLoading,
    settingsOpen,
    state,
  };
}

function applyRuntimeMessage(
  runtimeMessage: RuntimeMessage,
  setState: (state: ClientState) => void,
  setError: (message: string | null) => void,
): void {
  switch (runtimeMessage.kind) {
    case "state":
      setState(runtimeMessage.payload);
      setError(runtimeMessage.payload.last_error ?? null);
      break;
    case "transient_error":
      setError(runtimeMessage.payload.message);
      break;
    case "auth_revoked":
      setError("This device registration was revoked.");
      break;
    default:
      break;
  }
}
