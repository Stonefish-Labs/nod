import { requestPreview, orderedRequests } from "../domain";
import type { NodRequest } from "../types";

interface RequestListProps {
  requests: NodRequest[];
  onSelect: (request: NodRequest) => Promise<void>;
  selectedRequestId: string | null;
}

export function RequestList({
  requests,
  onSelect,
  selectedRequestId,
}: RequestListProps): JSX.Element {
  const ordered = orderedRequests(requests);

  return (
    <section className="requestList">
      {ordered.length === 0 ? <p className="empty">No Requests</p> : null}
      {ordered.map((request) => (
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
      ))}
    </section>
  );
}
