import { CircleAlert, X } from "lucide-react";
import type { Source } from "../types";

interface TopbarProps {
  activeSource?: Source;
  error: string | null;
  isConnected: boolean;
  onDismissError: () => void;
}

export function Topbar({
  activeSource,
  error,
  isConnected,
  onDismissError,
}: TopbarProps): JSX.Element {
  return (
    <header className="topbar">
      <div>
        <p>{activeSource?.name ?? "Requests"}</p>
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
