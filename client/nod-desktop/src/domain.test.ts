import { describe, expect, it } from "vitest";
import {
  submittableOptions,
  optionRequiresText,
  canSubmitEnrollment,
  requestPreview,
  orderedRequests,
  selectedSource,
  selectedRequest,
  sourceColor,
  totalPendingCount,
} from "./domain";
import type { Source, ClientState, RequestOption, NodRequest } from "./types";

const baseRequest: NodRequest = {
  id: "base",
  request_id: "base",
  source_id: "default",
  recipients: [],
  decision_resolution: "shared",
  title: "Base",
  summary: "",
  body_markdown: "",
  fields: [],
  links: [],
  image_url: null,
  notification: {
    redact: false,
    title: null,
    body: null,
  },
  dedupe_key: null,
  expires_at: null,
  status: "pending",
  created_at: "2026-05-31T12:00:00.000Z",
  updated_at: "2026-05-31T12:00:00.000Z",
  resolved_at: null,
  decision: null,
  decisions: [],
  callback_url: null,
  options: [],
  request_digest: "digest",
};

const baseState: ClientState = {
  servers: [],
  selected_server_id: null,
  current_user: null,
  devices: [],
  sources: [],
  pending_counts_by_source: {},
  requests: [],
  selected_source_id: null,
  selected_request_id: null,
  notification_sound: "default",
  notification_delivery_mode: "websocket",
  is_registered: false,
  is_sync_connected: false,
  last_error: null,
};

const baseSource: Source = {
  id: "default",
  name: "Default",
  emoji: "🔔",
  subscribed: true,
  created_at: "2026-05-31T12:00:00.000Z",
};

const baseOption: RequestOption = {
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
  it("totals pending counts across sources", () => {
    expect(
      totalPendingCount({
        ...baseState,
        pending_counts_by_source: { a: 2, b: 3 },
      }),
    ).toBe(5);
  });

  it("generates stable display colors from source identity", () => {
    expect(sourceColor(baseSource)).toBe(sourceColor({ ...baseSource }));
  });

  it("orders pending requests before handled requests", () => {
    const requests = orderedRequests([
      { ...baseRequest, id: "resolved", request_id: "resolved", status: "resolved" },
      { ...baseRequest, id: "pending", request_id: "pending", status: "pending" },
    ]);

    expect(requests.map((request) => request.id)).toEqual(["pending", "resolved"]);
  });

  it("orders newer requests before older requests with the same status", () => {
    const requests = orderedRequests([
      { ...baseRequest, id: "older", request_id: "older", created_at: "2026-05-31T11:00:00.000Z" },
      { ...baseRequest, id: "newer", request_id: "newer", created_at: "2026-05-31T13:00:00.000Z" },
    ]);

    expect(requests.map((request) => request.id)).toEqual(["newer", "older"]);
  });

  it("selects the explicit request when it exists", () => {
    const selected = selectedRequest({
      ...baseState,
      selected_request_id: "target",
      requests: [
        { ...baseRequest, id: "fallback", request_id: "fallback" },
        { ...baseRequest, id: "target", request_id: "target" },
      ],
    });

    expect(selected?.id).toBe("target");
  });

  it("falls back to the first ordered request when the selected request is absent", () => {
    const selected = selectedRequest({
      ...baseState,
      selected_request_id: "missing",
      requests: [
        { ...baseRequest, id: "resolved", request_id: "resolved", status: "resolved" },
        { ...baseRequest, id: "pending", request_id: "pending", status: "pending" },
      ],
    });

    expect(selected?.id).toBe("pending");
  });

  it("selects the explicit source when it exists", () => {
    const selected = selectedSource({
      ...baseState,
      selected_source_id: "target",
      sources: [
        { ...baseSource, id: "fallback" },
        { ...baseSource, id: "target" },
      ],
    });

    expect(selected?.id).toBe("target");
  });

  it("falls back to the first source when the selected source is absent", () => {
    const selected = selectedSource({
      ...baseState,
      selected_source_id: "missing",
      sources: [
        { ...baseSource, id: "fallback" },
        { ...baseSource, id: "target" },
      ],
    });

    expect(selected?.id).toBe("fallback");
  });

  it("creates a default dismiss option for requests without options", () => {
    const options = submittableOptions({ ...baseRequest, options: [] });

    expect(options[0]).toMatchObject({
      id: "dismiss",
      label: "Dismiss",
      kind: "dismiss",
    });
  });

  it("uses request options when the request defines them", () => {
    const options = submittableOptions({ ...baseRequest, options: [baseOption] });

    expect(options).toEqual([baseOption]);
  });

  it("treats explicit text options as requiring text", () => {
    expect(optionRequiresText({ ...baseOption, requires_text: true })).toBe(true);
  });

  it("treats text option kinds as requiring text", () => {
    expect(optionRequiresText({ ...baseOption, kind: "approve_with_text" })).toBe(
      true,
    );
  });

  it("uses the request summary as the preview text", () => {
    expect(
      requestPreview({
        ...baseRequest,
        summary: "Summary",
        body_markdown: "Body",
      }),
    ).toBe("Summary");
  });

  it("falls back to the body as preview text when the summary is empty", () => {
    expect(
      requestPreview({
        ...baseRequest,
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
