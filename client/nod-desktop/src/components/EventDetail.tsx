import { Check, ExternalLink, X } from "lucide-react";
import { useState } from "react";
import { actionableActions, actionRequiresText } from "../domain";
import type { EventAction, NodEvent } from "../types";

interface EventDetailProps {
  event?: NodEvent;
  onAction: (event: NodEvent, action: EventAction, text?: string) => Promise<void>;
  onOpenUrl: (url: string) => Promise<void>;
}

export function EventDetail({
  event,
  onAction,
  onOpenUrl,
}: EventDetailProps): JSX.Element {
  const [textByAction, setTextByAction] = useState<Record<string, string>>({});

  if (!event) {
    return <section className="detail empty">Select a Notification</section>;
  }

  return (
    <section className="detail">
      <header>
        <span className={`pill ${event.status}`}>{event.status}</span>
        <h2>{event.title}</h2>
        <p>{event.summary}</p>
      </header>
      {event.image_url ? <img src={event.image_url} alt="" className="eventImage" /> : null}
      {event.body_markdown ? <pre>{event.body_markdown}</pre> : null}
      <dl>
        {event.fields.map((field) => (
          <div key={`${field.label}:${field.value}`}>
            <dt>{field.label}</dt>
            <dd>{field.value}</dd>
          </div>
        ))}
      </dl>
      <div className="links">
        {event.links.map((link) => (
          <button type="button" key={link.url} onClick={() => void onOpenUrl(link.url)}>
            <ExternalLink size={14} />
            {link.label}
          </button>
        ))}
      </div>
      {event.status === "pending" ? (
        <div className="actions">
          {actionableActions(event).map((action) => (
            <div className="actionRow" key={action.id}>
              {actionRequiresText(action) ? (
                <input
                  value={textByAction[action.id] ?? ""}
                  onChange={(change) =>
                    setTextByAction((current) => ({
                      ...current,
                      [action.id]: change.currentTarget.value,
                    }))
                  }
                  placeholder={action.text_placeholder ?? action.label}
                />
              ) : null}
              <button
                type="button"
                className={action.destructive ? "danger" : ""}
                onClick={() => void onAction(event, action, textByAction[action.id])}
              >
                {action.destructive ? <X size={16} /> : <Check size={16} />}
                {action.label}
              </button>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}
