import { CircleAlert, X } from "lucide-react";
import type { Channel } from "../types";

interface TopbarProps {
  activeChannel?: Channel;
  error: string | null;
  isConnected: boolean;
  onDismissError: () => void;
}

export function Topbar({
  activeChannel,
  error,
  isConnected,
  onDismissError,
}: TopbarProps): JSX.Element {
  return (
    <header className="topbar">
      <div>
        <p>{activeChannel?.name ?? "Requests"}</p>
        <span>{isConnected ? "Connected" : "Offline"}</span>
      </div>
      {error ? (
        <button className="alert" type="button" onClick={onDismissError}>
          <CircleAlert size={16} />
          {error}
          <X size={14} />
        </button>
      ) : null}
    </header>
  );
}
