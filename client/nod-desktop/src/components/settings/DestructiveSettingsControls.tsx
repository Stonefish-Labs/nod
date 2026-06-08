import { Trash2 } from "lucide-react";

interface DestructiveSettingsControlsProps {
  canClearSource: boolean;
  onClearSelectedSource: () => Promise<void>;
  onForgetSelectedServer: () => Promise<void>;
}

export function DestructiveSettingsControls({
  canClearSource,
  onClearSelectedSource,
  onForgetSelectedServer,
}: DestructiveSettingsControlsProps): JSX.Element {
  return (
    <footer>
      <button type="button" className="danger" onClick={() => void onForgetSelectedServer()}>
        <Trash2 size={16} />
        Forget Server
      </button>
      <button
        type="button"
        onClick={() => void onClearSelectedSource()}
        disabled={!canClearSource}
      >
        <Trash2 size={16} />
        Clear Source
      </button>
    </footer>
  );
}
