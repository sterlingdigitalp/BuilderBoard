import { invoke } from "@tauri-apps/api/core";
import type { WorkspaceCreateInput, WorkspaceDto } from "../types/workspaces";

export const DEFAULT_WORKSPACE_ID = "00000000-0000-4000-8000-000000000001";
export const WORKSPACE_CHANGED_EVENT = "builderboard:workspace-changed";

const localStateKey = "builderboard.workspaceState.v1";

interface LocalWorkspaceState {
  workspaces: WorkspaceDto[];
  activeWorkspaceId: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function nullableString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function slugify(value: string): string {
  const slug = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");

  return slug || "workspace";
}

function nowIso(): string {
  return new Date().toISOString();
}

function defaultWorkspace(): WorkspaceDto {
  const now = nowIso();

  return {
    id: DEFAULT_WORKSPACE_ID,
    name: "Default",
    slug: "default",
    isDefault: true,
    layoutJson: null,
    metadataJson: null,
    createdAt: now,
    updatedAt: now
  };
}

function readLocalState(): LocalWorkspaceState {
  const fallback = {
    workspaces: [defaultWorkspace()],
    activeWorkspaceId: DEFAULT_WORKSPACE_ID
  };

  try {
    const rawState = window.localStorage.getItem(localStateKey);
    if (!rawState) {
      return fallback;
    }

    const parsed = JSON.parse(rawState);
    if (!isRecord(parsed) || !Array.isArray(parsed.workspaces)) {
      return fallback;
    }

    const workspaces = parsed.workspaces.map(toWorkspaceDto);
    const activeWorkspaceId =
      typeof parsed.activeWorkspaceId === "string" &&
      workspaces.some((workspace) => workspace.id === parsed.activeWorkspaceId)
        ? parsed.activeWorkspaceId
        : workspaces[0]?.id ?? DEFAULT_WORKSPACE_ID;

    return {
      workspaces: workspaces.length > 0 ? workspaces : fallback.workspaces,
      activeWorkspaceId
    };
  } catch {
    return fallback;
  }
}

function writeLocalState(state: LocalWorkspaceState, options?: { notify?: boolean }): void {
  window.localStorage.setItem(localStateKey, JSON.stringify(state));
  if (options?.notify === true) {
    window.dispatchEvent(new CustomEvent(WORKSPACE_CHANGED_EVENT));
  }
}

function mergeWithLocal(workspaces: WorkspaceDto[]): WorkspaceDto[] {
  const localState = readLocalState();
  const workspaceMap = new Map<string, WorkspaceDto>();

  workspaces.forEach((workspace) => workspaceMap.set(workspace.id, workspace));
  localState.workspaces
    .filter((workspace) => workspace.isLocalOnly === true)
    .forEach((workspace) => workspaceMap.set(workspace.id, workspace));

  return Array.from(workspaceMap.values()).sort((left, right) => {
    if (left.isDefault !== right.isDefault) {
      return left.isDefault ? -1 : 1;
    }

    return left.createdAt.localeCompare(right.createdAt);
  });
}

export function toWorkspaceDto(value: unknown): WorkspaceDto {
  if (!isRecord(value) || typeof value.id !== "string") {
    throw new Error("Invalid workspace response from persistence layer.");
  }

  return {
    id: value.id,
    name: typeof value.name === "string" ? value.name : "Untitled workspace",
    slug: typeof value.slug === "string" ? value.slug : slugify(String(value.name ?? value.id)),
    isDefault: value.isDefault === true,
    layoutJson: nullableString(value.layoutJson),
    metadataJson: nullableString(value.metadataJson),
    createdAt: typeof value.createdAt === "string" ? value.createdAt : "",
    updatedAt: typeof value.updatedAt === "string" ? value.updatedAt : "",
    isLocalOnly: value.isLocalOnly === true
  };
}

export function readActiveWorkspaceId(): string {
  return readLocalState().activeWorkspaceId;
}

export async function workspaceList(): Promise<WorkspaceDto[]> {
  try {
    const response = await invoke<unknown[]>("workspace_list");
    const workspaces = mergeWithLocal(response.map(toWorkspaceDto));
    const state = readLocalState();
    writeLocalState({
      workspaces,
      activeWorkspaceId: workspaces.some((workspace) => workspace.id === state.activeWorkspaceId)
        ? state.activeWorkspaceId
        : workspaces[0]?.id ?? DEFAULT_WORKSPACE_ID
    });
    return workspaces;
  } catch {
    return readLocalState().workspaces;
  }
}

export async function workspaceCreate(input: WorkspaceCreateInput): Promise<WorkspaceDto> {
  const name = input.name.trim() || "New Workspace";

  try {
    const response = await invoke<unknown>("workspace_create", { name });
    const workspace = toWorkspaceDto(response);
    const state = readLocalState();
    const existing = state.workspaces.filter((entry) => entry.id !== workspace.id);

    writeLocalState({
      workspaces: mergeWithLocal([...existing, workspace]),
      activeWorkspaceId: workspace.id
    }, { notify: true });
    return workspace;
  } catch {
    const state = readLocalState();
    const now = nowIso();
    const workspace = {
      id: `local-workspace-${crypto.randomUUID()}`,
      name,
      slug: slugify(name),
      isDefault: false,
      layoutJson: null,
      metadataJson: null,
      createdAt: now,
      updatedAt: now,
      isLocalOnly: true
    };

    writeLocalState({
      workspaces: [...state.workspaces, workspace],
      activeWorkspaceId: workspace.id
    }, { notify: true });
    return workspace;
  }
}

export async function workspaceSwitch(workspaceId: string): Promise<void> {
  try {
    await invoke("workspace_switch", { workspaceId });
  } catch {
    // Frontend fallback keeps active workspace restoration working until backend commands land.
  }

  const state = readLocalState();
  const activeWorkspaceId = state.workspaces.some((workspace) => workspace.id === workspaceId)
    ? workspaceId
    : state.activeWorkspaceId;

  writeLocalState({ ...state, activeWorkspaceId }, { notify: true });
}
