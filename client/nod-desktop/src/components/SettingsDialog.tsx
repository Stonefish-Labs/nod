import { X } from "lucide-react";
import { NOTIFICATION_SOUND_OPTIONS } from "../app/state";
import type { Source, ClientState, UserDevice } from "../types";
import { SourceSubscriptions } from "./settings/SourceSubscriptions";
import { DestructiveSettingsControls } from "./settings/DestructiveSettingsControls";
import { DeviceList } from "./settings/DeviceList";

export interface SettingsDialogCommands {
  clearSelectedSource: () => Promise<void>;
  closeSettings: () => void;
  forgetSelectedServer: () => Promise<void>;
  renameUserDevice: (deviceId: string, name: string) => Promise<boolean>;
  revokeUserDevice: (deviceId: string) => Promise<void>;
  toggleSourceSubscription: (source: Source) => Promise<void>;
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
        <SourceSubscriptions
          sources={state.sources}
          onToggleSource={commands.toggleSourceSubscription}
        />
        <DeviceList
          devices={devices}
          onRenameDevice={commands.renameUserDevice}
          onRevokeDevice={commands.revokeUserDevice}
        />
        <DestructiveSettingsControls
          canClearSource={Boolean(state.selected_source_id)}
          onClearSelectedSource={commands.clearSelectedSource}
          onForgetSelectedServer={commands.forgetSelectedServer}
        />
      </section>
    </div>
  );
}
