import { Check, ExternalLink, X } from "lucide-react";
import { useEffect, useState } from "react";
import { submittableOptions, optionRequiresText } from "../domain";
import type { RequestOption, NodRequest } from "../types";

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

  const options = submittableOptions(request);
  const someOptionTakesNotes = options.some(optionRequiresText);
  const trimmedNotes = notes.trim();

  function submit(option: RequestOption): void {
    if (!request) {
      return;
    }
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
            {options.map((option) => (
              <button
                type="button"
                key={option.id}
                className={option.destructive ? "danger" : ""}
                disabled={optionRequiresText(option) && trimmedNotes === ""}
                onClick={() => submit(option)}
              >
                {option.destructive ? <X size={16} /> : <Check size={16} />}
                {option.label}
              </button>
            ))}
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
