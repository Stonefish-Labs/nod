import { eventPreview, orderedEvents } from "../domain";
import type { NodEvent } from "../types";

interface EventListProps {
  events: NodEvent[];
  onSelect: (event: NodEvent) => Promise<void>;
  selectedEventId: string | null;
}

export function EventList({
  events,
  onSelect,
  selectedEventId,
}: EventListProps): JSX.Element {
  const ordered = orderedEvents(events);

  return (
    <section className="eventList">
      {ordered.length === 0 ? <p className="empty">No Notifications</p> : null}
      {ordered.map((event) => (
        <button
          type="button"
          key={event.id}
          className={event.id === selectedEventId ? "eventCard active" : "eventCard"}
          onClick={() => void onSelect(event)}
        >
          <span className={`status ${event.status}`} />
          <strong>{event.title}</strong>
          <span>{eventPreview(event)}</span>
          <time>{new Date(event.created_at).toLocaleString()}</time>
        </button>
      ))}
    </section>
  );
}
