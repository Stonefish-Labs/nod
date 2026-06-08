import type { Source } from "../../types";

interface SourceSubscriptionsProps {
  sources: Source[];
  onToggleSource: (source: Source) => Promise<void>;
}

export function SourceSubscriptions({
  sources,
  onToggleSource,
}: SourceSubscriptionsProps): JSX.Element {
  return (
    <section className="settingsSection">
      <h3>Sources</h3>
      {sources.map((source) => (
        <label className="checkRow" key={source.id}>
          <input
            type="checkbox"
            checked={source.subscribed}
            onChange={() => void onToggleSource(source)}
          />
          <span>{source.name}</span>
        </label>
      ))}
    </section>
  );
}
