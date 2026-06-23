import { invoke } from "@tauri-apps/api/core";
import type { PaneDto, PaneStatus } from "../types/layout";

const paneStatuses: PaneStatus[] = ["idle", "streaming", "error"];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function nullableString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function nullableNumber(value: unknown): number | null {
  return typeof value === "number" ? value : null;
}

function paneStatus(value: unknown): PaneStatus {
  return typeof value === "string" && paneStatuses.includes(value as PaneStatus)
    ? (value as PaneStatus)
    : "idle";
}

export function toPaneDto(value: unknown): PaneDto {
  if (!isRecord(value) || typeof value.id !== "string") {
    throw new Error("Invalid pane response from persistence layer.");
  }

  return {
    id: value.id,
    workspaceId: typeof value.workspaceId === "string" ? value.workspaceId : "default",
    title: nullableString(value.title),
    roleLabel: nullableString(value.roleLabel),
    sortOrder: typeof value.sortOrder === "number" ? value.sortOrder : 0,
    widthRatio: nullableNumber(value.widthRatio),
    heightRatio: nullableNumber(value.heightRatio),
    providerId: nullableString(value.providerId),
    accountId: nullableString(value.accountId),
    modelId: nullableString(value.modelId),
    status: paneStatus(value.status),
    layoutJson: nullableString(value.layoutJson),
    metadataJson: nullableString(value.metadataJson)
  };
}

// Phase 2B consumes Builder C's persistence commands; database access stays in Tauri.
export async function paneList(): Promise<PaneDto[]> {
  const response = await invoke<unknown[]>("pane_list");
  return response.map(toPaneDto).sort((a, b) => a.sortOrder - b.sortOrder);
}

export async function paneCreate(): Promise<PaneDto | null> {
  const response = await invoke<unknown>("pane_create");
  return response === null || response === undefined ? null : toPaneDto(response);
}

export async function paneClose(paneId: string): Promise<void> {
  await invoke("pane_close", { paneId });
}
