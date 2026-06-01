import { Trash2 } from "lucide-react";

interface DestructiveSettingsActionsProps {
  canClearChannel: boolean;
  onClearSelectedChannel: () => Promise<void>;
  onForgetSelectedServer: () => Promise<void>;
}

export function DestructiveSettingsActions({
  canClearChannel,
  onClearSelectedChannel,
  onForgetSelectedServer,
}: DestructiveSettingsActionsProps): JSX.Element {
  return (
    <footer>
      <button type="button" className="danger" onClick={() => void onForgetSelectedServer()}>
        <Trash2 size={16} />
        Forget Server
      </button>
      <button
        type="button"
        onClick={() => void onClearSelectedChannel()}
        disabled={!canClearChannel}
      >
        <Trash2 size={16} />
        Clear Channel
      </button>
    </footer>
  );
}
