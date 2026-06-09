import { X } from "lucide-react";
import { NOTIFICATION_SOUND_OPTIONS } from "../app/state";
import type { Channel, ClientState, UserDevice } from "../types";
import { ChannelSubscriptions } from "./settings/ChannelSubscriptions";
import { DestructiveSettingsControls } from "./settings/DestructiveSettingsControls";
import { DeviceList } from "./settings/DeviceList";

export interface SettingsDialogCommands {
  clearSelectedChannel: () => Promise<void>;
  closeSettings: () => void;
  forgetSelectedServer: () => Promise<void>;
  renameUserDevice: (deviceId: string, name: string) => Promise<boolean>;
  revokeUserDevice: (deviceId: string) => Promise<void>;
  toggleChannelSubscription: (channel: Channel) => Promise<void>;
  updateNotificationSound: (notificationSound: string) => Promise<void>;
}

interface SettingsDialogProps {
  commands: SettingsDialogCommands;
  devices: UserDevice[];
  state: ClientState;
}

export function SettingsDialog({
  commands,
  devices,
  state,
}: SettingsDialogProps): JSX.Element {
  return (
    <div className="scrim">
      <section className="dialog">
        <header>
          <h2>Settings</h2>
          <button type="button" onClick={commands.closeSettings}>
            <X size={16} />
          </button>
        </header>
        <label>
          Notification Sound
          <select
            value={state.notification_sound}
            onChange={(event) =>
              void commands.updateNotificationSound(event.currentTarget.value)
            }
          >
            {NOTIFICATION_SOUND_OPTIONS.map((option) => (
              <option key={option.id} value={option.id}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <ChannelSubscriptions
          channels={state.channels}
          onToggleChannel={commands.toggleChannelSubscription}
        />
        <DeviceList
          devices={devices}
          onRenameDevice={commands.renameUserDevice}
          onRevokeDevice={commands.revokeUserDevice}
        />
        <DestructiveSettingsControls
          canClearChannel={Boolean(state.selected_channel_id)}
          onClearSelectedChannel={commands.clearSelectedChannel}
          onForgetSelectedServer={commands.forgetSelectedServer}
        />
      </section>
    </div>
  );
}
