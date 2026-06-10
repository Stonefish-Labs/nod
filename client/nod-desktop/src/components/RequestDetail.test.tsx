// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

afterEach(cleanup);
import { RequestDetail } from "./RequestDetail";
import type { NodRequest, RequestOption } from "../types";

const approve: RequestOption = {
  id: "approve",
  label: "Approve",
  kind: "approve",
  style: "primary",
  requires_text: false,
  text_placeholder: null,
  destructive: false,
  foreground: false,
};

const approveWithNotes: RequestOption = {
  ...approve,
  id: "approve_notes",
  label: "Approve with notes",
  kind: "approve_with_text",
};

const reject: RequestOption = {
  ...approve,
  id: "reject",
  label: "Reject",
  kind: "reject",
  destructive: true,
};

function pendingRequest(options: RequestOption[]): NodRequest {
  return {
    id: "req-1",
    request_id: "req-1",
    channel_id: "default",
    recipients: ["owner"],
    decision_resolution: "shared",
    title: "Deploy?",
    summary: "v1 is ready",
    body_markdown: "",
    fields: [],
    links: [],
    image_url: null,
    notification: { redact: false, title: null, body: null },
    dedupe_key: null,
    expires_at: null,
    status: "pending",
    created_at: "2026-06-10T12:00:00.000Z",
    updated_at: "2026-06-10T12:00:00.000Z",
    resolved_at: null,
    decision: null,
    decisions: [],
    callback_url: null,
    options,
    request_digest: "digest",
  };
}

describe("RequestDetail notes", () => {
  it("sends the shared notes with whichever option is clicked", async () => {
    const onOption = vi.fn().mockResolvedValue(undefined);
    render(
      <RequestDetail
        request={pendingRequest([approve, approveWithNotes, reject])}
        onOption={onOption}
        onOpenUrl={vi.fn()}
      />,
    );

    // Typing must not crash (regression: reading event.currentTarget inside
    // the state updater blanked the screen on the first keystroke).
    const notes = screen.getByPlaceholderText("Sent with whichever decision you pick");
    fireEvent.change(notes, { target: { value: "ship it carefully" } });

    fireEvent.click(screen.getByRole("button", { name: "Reject" }));
    expect(onOption).toHaveBeenCalledWith(
      expect.objectContaining({ id: "req-1" }),
      expect.objectContaining({ id: "reject" }),
      "ship it carefully",
    );
  });

  it("requires notes before a with-text option can be submitted", () => {
    const onOption = vi.fn().mockResolvedValue(undefined);
    render(
      <RequestDetail
        request={pendingRequest([approveWithNotes, reject])}
        onOption={onOption}
        onOpenUrl={vi.fn()}
      />,
    );

    const withNotes = screen.getByRole("button", { name: "Approve with notes" });
    expect(withNotes).toBeDisabled();
    // Plain options stay usable without notes, and send none.
    fireEvent.click(screen.getByRole("button", { name: "Reject" }));
    expect(onOption).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({ id: "reject" }),
      undefined,
    );

    fireEvent.change(
      screen.getByPlaceholderText("Sent with whichever decision you pick"),
      { target: { value: "lgtm" } },
    );
    expect(withNotes).toBeEnabled();
  });

  it("hides the notes field when no option accepts text", () => {
    render(
      <RequestDetail
        request={pendingRequest([approve, reject])}
        onOption={vi.fn()}
        onOpenUrl={vi.fn()}
      />,
    );

    expect(
      screen.queryByPlaceholderText("Sent with whichever decision you pick"),
    ).toBeNull();
  });
});
