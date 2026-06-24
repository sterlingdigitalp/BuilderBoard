import { invoke } from "@tauri-apps/api/core";
import type { PaneDto, PaneStatus } from "../types/layout";
import { isTauriRuntime } from "./tauriRuntime";
import { DEFAULT_WORKSPACE_ID } from "./workspaceCommands";

const paneStatuses: PaneStatus[] = ["idle", "streaming", "error"];
const localPaneKey = "builderboard.localPanes.v1";

type LocalPaneState = Record<string, PaneDto[]>;

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

function readLocalPanes(): LocalPaneState {
  try {
    const rawState = window.localStorage.getItem(localPaneKey);
    if (!rawState) {
      return {};
    }

    const parsed = JSON.parse(rawState);
    return isRecord(parsed) ? (parsed as LocalPaneState) : {};
  } catch {
    return {};
  }
}

function writeLocalPanes(state: LocalPaneState): void {
  window.localStorage.setItem(localPaneKey, JSON.stringify(state));
}

function localPaneList(workspaceId: string): PaneDto[] {
  return (readLocalPanes()[workspaceId] ?? []).sort((a, b) => a.sortOrder - b.sortOrder);
}

function createLocalPane(workspaceId: string): PaneDto {
  const state = readLocalPanes();
  const panes = state[workspaceId] ?? [];
  const pane: PaneDto = {
    id: `local-pane-${crypto.randomUUID()}`,
    workspaceId,
    title: "New Pane",
    roleLabel: null,
    sortOrder: panes.length,
    widthRatio: null,
    heightRatio: null,
    providerId: null,
    accountId: null,
    modelId: null,
    status: "idle",
    projectId: null,
    layoutJson: null,
    metadataJson: JSON.stringify({ localOnly: true })
  };

  writeLocalPanes({
    ...state,
    [workspaceId]: [...panes, pane]
  });
  return pane;
}

function closeLocalPane(paneId: string): boolean {
  const state = readLocalPanes();
  let didClose = false;
  const nextState = Object.fromEntries(
    Object.entries(state).map(([workspaceId, panes]) => {
      const nextPanes = panes.filter((pane) => pane.id !== paneId);
      didClose = didClose || nextPanes.length !== panes.length;
      return [workspaceId, nextPanes];
    })
  );

  if (didClose) {
    writeLocalPanes(nextState);
  }

  return didClose;
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
    projectId: nullableString(value.projectId),
    layoutJson: nullableString(value.layoutJson),
    metadataJson: nullableString(value.metadataJson)
  };
}

// Phase 8A lists all shell panes; new panes bind to the focused project.
export async function paneList(workspaceId?: string): Promise<PaneDto[]> {
  const resolvedWorkspaceId = workspaceId ?? DEFAULT_WORKSPACE_ID;

  try {
    const response = await invoke<unknown[]>("pane_list", {
      workspaceId: DEFAULT_WORKSPACE_ID
    });
    return response.map(toPaneDto).sort((a, b) => a.sortOrder - b.sortOrder);
  } catch (error) {
    if (!isTauriRuntime()) {
      return localPaneList(resolvedWorkspaceId);
    }
    throw error;
  }
}

export async function paneCreate(
  workspaceId?: string,
  projectId?: string
): Promise<PaneDto | null> {
  const resolvedWorkspaceId = workspaceId ?? DEFAULT_WORKSPACE_ID;

  try {
    const response = await invoke<unknown>("pane_create", {
      workspaceId: DEFAULT_WORKSPACE_ID,
      projectId
    });
    return response === null || response === undefined ? null : toPaneDto(response);
  } catch (error) {
    if (!isTauriRuntime()) {
      return createLocalPane(resolvedWorkspaceId);
    }
    throw error;
  }
}

export async function paneSetProject(paneId: string, projectId: string): Promise<PaneDto> {
  const response = await invoke<unknown>("pane_set_project", { paneId, projectId });
  return toPaneDto(response);
}

export async function paneClose(paneId: string): Promise<void> {
  if (paneId.startsWith("local-pane-")) {
    closeLocalPane(paneId);
    return;
  }

  try {
    await invoke("pane_close", { paneId });
  } catch (error) {
    if (!closeLocalPane(paneId)) {
      throw error;
    }
  }
}
