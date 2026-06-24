import { invoke } from "@tauri-apps/api/core";
import { paneList } from "./paneCommands";

export function runtimeTraceEnabled(): boolean {
  try {
    return window.localStorage.getItem("BUILDERBOARD_TRACE_RUNTIME") === "1";
  } catch {
    return false;
  }
}

export function traceRuntimeMetric(label: string, value: number): void {
  if (!runtimeTraceEnabled()) {
    return;
  }

  console.log(`${label}=${Math.round(value)}`);
}

export async function probeRuntimePing(): Promise<number> {
  const started = performance.now();
  await invoke<number>("runtime_probe_ping");
  return performance.now() - started;
}

export async function probePaneListLatency(): Promise<number> {
  const started = performance.now();
  await paneList();
  return performance.now() - started;
}

export async function probeCrossPaneInteraction(paneId: string): Promise<{
  pingMs: number;
  paneListMs: number;
  messageListMs: number;
}> {
  const pingMs = await probeRuntimePing();
  const paneListMs = await probePaneListLatency();
  const messageStarted = performance.now();
  await invoke<unknown[]>("message_list", { paneId });
  const messageListMs = performance.now() - messageStarted;

  if (runtimeTraceEnabled()) {
    traceRuntimeMetric("EVENT_LOOP_BLOCK_MS", Math.max(pingMs, paneListMs, messageListMs));
    traceRuntimeMetric("RUNTIME_PROBE_ROUNDTRIP_MS", pingMs);
    traceRuntimeMetric("PANE_LIST_INVOKE_MS", paneListMs);
    traceRuntimeMetric("MESSAGE_LIST_INVOKE_MS", messageListMs);
  }

  return { pingMs, paneListMs, messageListMs };
}