import type {
  ClientState,
  EnrollParams,
  NodRequest,
  RequestOption,
  RequestStatus,
  Source,
} from "./types";

const statusRank: Record<RequestStatus, number> = {
  pending: 0,
  resolved: 1,
  expired: 1,
  cancelled: 1,
};

const defaultDismissOption: RequestOption = {
  id: "dismiss",
  label: "Dismiss",
  kind: "dismiss",
  style: "default",
  requires_text: false,
  destructive: false,
  foreground: false,
};

export function totalPendingCount(state: ClientState): number {
  return Object.values(state.pending_counts_by_source).reduce(
    (total, count) => total + count,
    0,
  );
}

export function pendingCountFor(source: Source, state: ClientState): number {
  return state.pending_counts_by_source[source.id] ?? 0;
}

export function sourceColor(source: Source): string {
  const palette = [
    "#2563EB",
    "#059669",
    "#D97706",
    "#DC2626",
    "#7C3AED",
    "#0891B2",
    "#4F46E5",
    "#DB2777",
  ];
  let hash = 0;
  for (const char of source.id) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  }
  return palette[hash % palette.length];
}

export function orderedRequests(requests: readonly NodRequest[]): NodRequest[] {
  return [...requests].sort((left, right) => {
    const rankDelta = statusRank[left.status] - statusRank[right.status];
    if (rankDelta !== 0) {
      return rankDelta;
    }
    const timeDelta =
      new Date(right.created_at).getTime() - new Date(left.created_at).getTime();
    if (timeDelta !== 0) {
      return timeDelta;
    }
    return right.id.localeCompare(left.id);
  });
}

export function selectedRequest(state: ClientState): NodRequest | undefined {
  return (
    state.requests.find((request) => request.id === state.selected_request_id) ??
    orderedRequests(state.requests)[0]
  );
}

export function selectedSource(state: ClientState): Source | undefined {
  return (
    state.sources.find((source) => source.id === state.selected_source_id) ??
    state.sources[0]
  );
}

export function submittableOptions(request: NodRequest): RequestOption[] {
  return request.options.length === 0 ? [defaultDismissOption] : request.options;
}

export function optionRequiresText(option: RequestOption): boolean {
  return option.requires_text || option.kind.endsWith("_with_text");
}

export function requestPreview(request: NodRequest): string {
  return request.summary || request.body_markdown;
}

export function canSubmitEnrollment(
  draft: Pick<EnrollParams, "base_url" | "device_name" | "code">,
): boolean {
  return (
    draft.base_url.trim().length > 0 &&
    draft.device_name.trim().length > 0 &&
    draft.code.trim().length >= 8
  );
}
