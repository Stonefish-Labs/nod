import { Check, ChevronDown, Trash2 } from "lucide-react";
import { useState } from "react";
import type { UserDevice } from "../../types";

interface DeviceListProps {
  devices: UserDevice[];
  onRenameDevice: (deviceId: string, name: string) => Promise<boolean>;
  onRevokeDevice: (deviceId: string) => Promise<void>;
}

export function DeviceList({
  devices,
  onRenameDevice,
  onRevokeDevice,
}: DeviceListProps): JSX.Element {
  const [renamingDeviceId, setRenamingDeviceId] = useState<string | null>(null);
  const [renameText, setRenameText] = useState("");

  async function renameSelectedDevice(): Promise<void> {
    if (renamingDeviceId === null) {
      return;
    }
    if (await onRenameDevice(renamingDeviceId, renameText)) {
      setRenamingDeviceId(null);
      setRenameText("");
    }
  }

  return (
    <section className="settingsSection">
      <h3>Devices</h3>
      {devices.map((device) => (
        <div className="deviceRow" key={device.id}>
          {renamingDeviceId === device.id ? (
            <input
              value={renameText}
              onChange={(event) => setRenameText(event.currentTarget.value)}
            />
          ) : (
            <span>{device.name}</span>
          )}
          <small>
            {device.platform}
            {device.is_current ? " current" : ""}
          </small>
          {renamingDeviceId === device.id ? (
            <button
              type="button"
              onClick={() => void renameSelectedDevice()}
              disabled={renameText.trim().length === 0}
            >
              <Check size={14} />
            </button>
          ) : (
            <button
              type="button"
              onClick={() => {
                setRenamingDeviceId(device.id);
                setRenameText(device.name);
              }}
            >
              <ChevronDown size={14} />
            </button>
          )}
          <button
            type="button"
            className="dangerIcon"
            onClick={() => void onRevokeDevice(device.id)}
          >
            <Trash2 size={14} />
          </button>
        </div>
      ))}
    </section>
  );
}
