import type {
  Channel,
  ClientState,
  EnrollParams,
  EventAction,
  EventStatus,
  NodEvent,
} from "./types";

const statusRank: Record<EventStatus, number> = {
  pending: 0,
  resolved: 1,
  expired: 1,
  cancelled: 1,
};

const defaultDismissAction: EventAction = {
  id: "dismiss",
  label: "Dismiss",
  kind: "dismiss",
  style: "default",
  requires_text: false,
  destructive: false,
  foreground: false,
};

export function totalPendingCount(state: ClientState): number {
  return Object.values(state.pending_counts_by_channel).reduce(
    (total, count) => total + count,
    0,
  );
}

export function pendingCountFor(channel: Channel, state: ClientState): number {
  return state.pending_counts_by_channel[channel.id] ?? 0;
}

export function orderedEvents(events: readonly NodEvent[]): NodEvent[] {
  return [...events].sort((left, right) => {
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

export function selectedEvent(state: ClientState): NodEvent | undefined {
  return (
    state.events.find((event) => event.id === state.selected_event_id) ??
    orderedEvents(state.events)[0]
  );
}

export function selectedChannel(state: ClientState): Channel | undefined {
  return (
    state.channels.find((channel) => channel.id === state.selected_channel_id) ??
    state.channels[0]
  );
}

export function actionableActions(event: NodEvent): EventAction[] {
  return event.actions.length === 0 ? [defaultDismissAction] : event.actions;
}

export function actionRequiresText(action: EventAction): boolean {
  return action.requires_text || action.kind.endsWith("_with_text");
}

export function eventPreview(event: NodEvent): string {
  return event.summary || event.body_markdown;
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
