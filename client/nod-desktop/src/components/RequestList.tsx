import { ChevronDown, ChevronRight } from "lucide-react";
import { useEffect, useState } from "react";
import { requestPreview, orderedRequests } from "../domain";
import type { NodRequest } from "../types";

interface RequestListProps {
  requests: NodRequest[];
  onSelect: (request: NodRequest) => Promise<void>;
  selectedRequestId: string | null;
}

// Mirrors the macOS inbox: Pending opens expanded, Handled stays tucked away
// until there is nothing left to act on. The parent keys this component by
// channel, so switching channels resets the expansion state.
export function RequestList({
  requests,
  onSelect,
  selectedRequestId,
}: RequestListProps): JSX.Element {
  const ordered = orderedRequests(requests);
  const pending = ordered.filter((request) => request.status === "pending");
  const handled = ordered.filter((request) => request.status !== "pending");
  const [pendingExpanded, setPendingExpanded] = useState(true);
  const [handledExpanded, setHandledExpanded] = useState(false);

  const hasPending = pending.length > 0;
  useEffect(() => {
    if (!hasPending) {
      setHandledExpanded(true);
    }
  }, [hasPending]);

  function card(request: NodRequest): JSX.Element {
    return (
      <button
        type="button"
        key={request.id}
        className={request.id === selectedRequestId ? "requestCard active" : "requestCard"}
        onClick={() => void onSelect(request)}
      >
        <span className={`status ${request.status}`} />
        <strong>{request.title}</strong>
        <span>{requestPreview(request)}</span>
        <time>{new Date(request.created_at).toLocaleString()}</time>
      </button>
    );
  }

  return (
    <section className="requestList">
      {ordered.length === 0 ? <p className="empty">No Requests</p> : null}
      {pending.length > 0 ? (
        <>
          <SectionHeader
            title="Pending"
            count={pending.length}
            expanded={pendingExpanded}
            onToggle={() => setPendingExpanded((expanded) => !expanded)}
          />
          {pendingExpanded ? pending.map(card) : null}
        </>
      ) : null}
      {handled.length > 0 ? (
        <>
          <SectionHeader
            title="Handled"
            count={handled.length}
            expanded={handledExpanded}
            onToggle={() => setHandledExpanded((expanded) => !expanded)}
          />
          {handledExpanded ? handled.map(card) : null}
        </>
      ) : null}
    </section>
  );
}

interface SectionHeaderProps {
  title: string;
  count: number;
  expanded: boolean;
  onToggle: () => void;
}

function SectionHeader({ title, count, expanded, onToggle }: SectionHeaderProps): JSX.Element {
  return (
    <button type="button" className="sectionHeader" aria-expanded={expanded} onClick={onToggle}>
      {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
      {title}
      <span className="sectionCount">{count}</span>
    </button>
  );
}
