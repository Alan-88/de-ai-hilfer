import { apiUrl } from "$lib/api";
import type {
  AnalyzeResponse,
  AnalyzeStreamRequest,
  FollowUpCreateResponse,
  QualityMode,
} from "$lib/types";

interface StreamMetaPayload {
  kind: string;
  model: string;
  quality_mode: QualityMode;
  source: string;
  fallback: boolean;
}

interface StreamDeltaPayload {
  delta: string;
}

interface StreamErrorPayload {
  message: string;
}

interface StreamHandlers<TComplete> {
  signal?: AbortSignal;
  onMeta?: (payload: StreamMetaPayload) => void;
  onDelta?: (payload: StreamDeltaPayload) => void;
  onComplete?: (payload: TComplete) => void;
  onError?: (payload: StreamErrorPayload) => void;
}

interface FollowUpStreamRequest {
  entry_id: number;
  question: string;
  quality_mode?: QualityMode;
}

export function streamAnalyze(
  payload: AnalyzeStreamRequest,
  handlers: StreamHandlers<AnalyzeResponse>
) {
  return streamJsonEvents<AnalyzeResponse>("/analyze/stream", payload, handlers);
}

export function streamFollowUp(
  payload: FollowUpStreamRequest,
  handlers: StreamHandlers<FollowUpCreateResponse>
) {
  return streamJsonEvents<FollowUpCreateResponse>("/follow-ups/stream", payload, handlers);
}

async function streamJsonEvents<TComplete>(
  path: string,
  payload: object,
  handlers: StreamHandlers<TComplete>
) {
  const response = await fetch(apiUrl(path), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
    signal: handlers.signal,
  });

  if (!response.ok) {
    throw new Error(await extractErrorMessage(response));
  }

  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error("stream body unavailable");
  }

  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    let separatorIndex = buffer.indexOf("\n\n");

    while (separatorIndex !== -1) {
      const rawEvent = buffer.slice(0, separatorIndex);
      buffer = buffer.slice(separatorIndex + 2);
      handleEvent(rawEvent, handlers);
      separatorIndex = buffer.indexOf("\n\n");
    }
  }

  const trailing = buffer.trim();
  if (trailing) {
    handleEvent(trailing, handlers);
  }
}

function handleEvent<TComplete>(rawEvent: string, handlers: StreamHandlers<TComplete>) {
  const eventName =
    rawEvent
      .split("\n")
      .find((line) => line.startsWith("event:"))
      ?.slice("event:".length)
      .trim() ?? "message";
  const data = rawEvent
    .split("\n")
    .filter((line) => line.startsWith("data:"))
    .map((line) => line.slice("data:".length).trim())
    .join("\n");

  if (!data) return;

  const parsed = JSON.parse(data);
  if (eventName === "meta") {
    handlers.onMeta?.(parsed as StreamMetaPayload);
  } else if (eventName === "delta") {
    handlers.onDelta?.(parsed as StreamDeltaPayload);
  } else if (eventName === "complete") {
    handlers.onComplete?.(parsed as TComplete);
  } else if (eventName === "error") {
    handlers.onError?.(parsed as StreamErrorPayload);
  }
}

async function extractErrorMessage(response: Response) {
  try {
    const error = await response.json();
    return error.detail || error.message || `Request failed: ${response.status}`;
  } catch {
    return `Request failed: ${response.status}`;
  }
}
