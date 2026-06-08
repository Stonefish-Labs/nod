import { Check, ExternalLink, X } from "lucide-react";
import { useState } from "react";
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
  const [textByOption, setTextByOption] = useState<Record<string, string>>({});

  if (!request) {
    return <section className="detail empty">Select a Request</section>;
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
          {submittableOptions(request).map((option) => (
            <div className="optionRow" key={option.id}>
              {optionRequiresText(option) ? (
                <input
                  value={textByOption[option.id] ?? ""}
                  onChange={(change) =>
                    setTextByOption((current) => ({
                      ...current,
                      [option.id]: change.currentTarget.value,
                    }))
                  }
                  placeholder={option.text_placeholder ?? option.label}
                />
              ) : null}
              <button
                type="button"
                className={option.destructive ? "danger" : ""}
                onClick={() => void onOption(request, option, textByOption[option.id])}
              >
                {option.destructive ? <X size={16} /> : <Check size={16} />}
                {option.label}
              </button>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}
