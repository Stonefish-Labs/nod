import { describe, expect, it } from "vitest";
import {
  actionableActions,
  actionRequiresText,
  canSubmitEnrollment,
  eventPreview,
  orderedEvents,
  selectedChannel,
  selectedEvent,
  totalPendingCount,
} from "./domain";
import type { Channel, ClientState, EventAction, NodEvent } from "./types";

const baseEvent: NodEvent = {
  id: "base",
  channel_id: "default",
  recipients: [],
  action_resolution: "shared",
  title: "Base",
  summary: "",
  body_markdown: "",
  fields: [],
  links: [],
  image_url: null,
  priority: 5,
  privacy: "private",
  dedupe_key: null,
  expires_at: null,
  status: "pending",
  created_at: "2026-05-31T12:00:00.000Z",
  updated_at: "2026-05-31T12:00:00.000Z",
  resolved_at: null,
  result: null,
  user_results: [],
  callback_url: null,
  actions: [],
  request_digest: "digest",
};

const baseState: ClientState = {
  servers: [],
  selected_server_id: null,
  current_user: null,
  devices: [],
  channels: [],
  pending_counts_by_channel: {},
  events: [],
  selected_channel_id: null,
  selected_event_id: null,
  notification_sound: "default",
  notification_delivery_mode: "websocket",
  is_registered: false,
  is_sync_connected: false,
  last_error: null,
};

const baseChannel: Channel = {
  id: "default",
  name: "Default",
  icon: "bell",
  color: "#d7f86f",
  default_priority: 5,
  privacy: "private",
  subscribed: true,
  created_at: "2026-05-31T12:00:00.000Z",
};

const baseAction: EventAction = {
  id: "approve",
  label: "Approve",
  kind: "approve",
  style: "default",
  requires_text: false,
  text_placeholder: null,
  destructive: false,
  foreground: false,
};

describe("domain helpers", () => {
  it("totals pending counts across channels", () => {
    expect(
      totalPendingCount({
        ...baseState,
        pending_counts_by_channel: { a: 2, b: 3 },
      }),
    ).toBe(5);
  });

  it("orders pending events before handled events", () => {
    const events = orderedEvents([
      { ...baseEvent, id: "resolved", status: "resolved" },
      { ...baseEvent, id: "pending", status: "pending" },
    ]);

    expect(events.map((event) => event.id)).toEqual(["pending", "resolved"]);
  });

  it("orders newer events before older events with the same status", () => {
    const events = orderedEvents([
      { ...baseEvent, id: "older", created_at: "2026-05-31T11:00:00.000Z" },
      { ...baseEvent, id: "newer", created_at: "2026-05-31T13:00:00.000Z" },
    ]);

    expect(events.map((event) => event.id)).toEqual(["newer", "older"]);
  });

  it("selects the explicit event when it exists", () => {
    const selected = selectedEvent({
      ...baseState,
      selected_event_id: "target",
      events: [
        { ...baseEvent, id: "fallback" },
        { ...baseEvent, id: "target" },
      ],
    });

    expect(selected?.id).toBe("target");
  });

  it("falls back to the first ordered event when the selected event is absent", () => {
    const selected = selectedEvent({
      ...baseState,
      selected_event_id: "missing",
      events: [
        { ...baseEvent, id: "resolved", status: "resolved" },
        { ...baseEvent, id: "pending", status: "pending" },
      ],
    });

    expect(selected?.id).toBe("pending");
  });

  it("selects the explicit channel when it exists", () => {
    const selected = selectedChannel({
      ...baseState,
      selected_channel_id: "target",
      channels: [
        { ...baseChannel, id: "fallback" },
        { ...baseChannel, id: "target" },
      ],
    });

    expect(selected?.id).toBe("target");
  });

  it("falls back to the first channel when the selected channel is absent", () => {
    const selected = selectedChannel({
      ...baseState,
      selected_channel_id: "missing",
      channels: [
        { ...baseChannel, id: "fallback" },
        { ...baseChannel, id: "target" },
      ],
    });

    expect(selected?.id).toBe("fallback");
  });

  it("creates a default dismiss action for events without actions", () => {
    const actions = actionableActions({ ...baseEvent, actions: [] });

    expect(actions[0]).toMatchObject({
      id: "dismiss",
      label: "Dismiss",
      kind: "dismiss",
    });
  });

  it("uses event actions when the event defines them", () => {
    const actions = actionableActions({ ...baseEvent, actions: [baseAction] });

    expect(actions).toEqual([baseAction]);
  });

  it("treats explicit text actions as requiring text", () => {
    expect(actionRequiresText({ ...baseAction, requires_text: true })).toBe(true);
  });

  it("treats text action kinds as requiring text", () => {
    expect(actionRequiresText({ ...baseAction, kind: "approve_with_text" })).toBe(
      true,
    );
  });

  it("uses the event summary as the preview text", () => {
    expect(
      eventPreview({
        ...baseEvent,
        summary: "Summary",
        body_markdown: "Body",
      }),
    ).toBe("Summary");
  });

  it("falls back to the body as preview text when the summary is empty", () => {
    expect(
      eventPreview({
        ...baseEvent,
        summary: "",
        body_markdown: "Body",
      }),
    ).toBe("Body");
  });

  it("allows enrollment when required fields are present", () => {
    expect(
      canSubmitEnrollment({
        base_url: "https://nod.example.com",
        device_name: "Desktop",
        code: "ABCDEFGH",
      }),
    ).toBe(true);
  });

  it("blocks enrollment when the code is too short", () => {
    expect(
      canSubmitEnrollment({
        base_url: "https://nod.example.com",
        device_name: "Desktop",
        code: "ABC",
      }),
    ).toBe(false);
  });
});
