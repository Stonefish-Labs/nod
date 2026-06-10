import { Check, ExternalLink, X } from "lucide-react";
import { useEffect, useState } from "react";
import { decisionActions, optionRequiresText, type DecisionAction } from "../domain";
import type { NodRequest, RequestOption } from "../types";

interface RequestDetailProps {
  request?: NodRequest;
  onOption: (request: NodRequest, option: RequestOption, text?: string) => Promise<void>;
  onOpenUrl: (url: string) => Promise<void>;
}

export function RequestDetail({
  request,
  onOption,
  onOpenUrl,
}: RequestDetailProps): JSX.Element {
  // One notes field for the whole decision: whichever option is clicked
  // (approve or reject) carries the notes with it.
  const [notes, setNotes] = useState("");

  useEffect(() => {
    setNotes("");
  }, [request?.id]);

  if (!request) {
    return <section className="detail empty">Select a Request</section>;
  }

  const actions = decisionActions(request);
  const someOptionTakesNotes = actions.some(
    (action) => action.withTextOption !== undefined || optionRequiresText(action.option),
  );
  const trimmedNotes = notes.trim();

  function submit(action: DecisionAction): void {
    if (!request) {
      return;
    }
    const option =
      trimmedNotes !== "" && action.withTextOption ? action.withTextOption : action.option;
    void onOption(request, option, trimmedNotes === "" ? undefined : trimmedNotes);
  }

  return (
    <section className="detail">
      <header>
        <span className={`pill ${request.status}`}>{request.status}</span>
        <h2>{request.title}</h2>
        <p>{request.summary}</p>
      </header>
      {request.image_url ? <img src={request.image_url} alt="" className="requestImage" /> : null}
      {request.body_markdown ? <pre>{request.body_markdown}</pre> : null}
      <dl>
        {request.fields.map((field) => (
          <div key={`${field.label}:${field.value}`}>
            <dt>{field.label}</dt>
            <dd>{field.value}</dd>
          </div>
        ))}
      </dl>
      <div className="links">
        {request.links.map((link) => (
          <button type="button" key={link.url} onClick={() => void onOpenUrl(link.url)}>
            <ExternalLink size={14} />
            {link.label}
          </button>
        ))}
      </div>
      {request.status === "pending" ? (
        <div className="options">
          <div className="optionButtons">
            {actions.map((action) => {
              const destructive =
                action.option.destructive || action.withTextOption?.destructive === true;
              return (
                <button
                  type="button"
                  key={action.option.id}
                  className={destructive ? "danger" : ""}
                  disabled={optionRequiresText(action.option) && trimmedNotes === ""}
                  onClick={() => submit(action)}
                >
                  {destructive ? <X size={16} /> : <Check size={16} />}
                  {action.option.label}
                </button>
              );
            })}
          </div>
          {someOptionTakesNotes ? (
            <label className="optionNotes">
              Notes
              <textarea
                value={notes}
                onChange={(change) => setNotes(change.currentTarget.value)}
                placeholder="Sent with whichever decision you pick"
                rows={3}
              />
            </label>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
