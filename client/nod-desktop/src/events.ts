import { listen } from "@tauri-apps/api/event";
import type { RuntimeEvent } from "./types";

const RUNTIME_EVENT_NAME = "nod://event";

export function listenForRuntimeEvents(
  handler: (event: RuntimeEvent) => void,
): Promise<() => void> {
  return listen<RuntimeEvent>(RUNTIME_EVENT_NAME, (event) => handler(event.payload));
}
