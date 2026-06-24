import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectDto } from "../types/projects";
import { WORKSPACE_CHANGED_EVENT, workspaceSwitch } from "./workspaceCommands";

export const PROJECT_CHANGED_EVENT = "builderboard:project-changed";

const localActiveProjectKey = "builderboard.activeProjectId.v1";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function toProjectDto(value: unknown): ProjectDto {
  if (!isRecord(value) || typeof value.id !== "string") {
    throw new Error("Invalid project response from persistence layer.");
  }

  return {
    id: value.id,
    workspaceId: typeof value.workspaceId === "string" ? value.workspaceId : "",
    name: typeof value.name === "string" ? value.name : "Untitled project",
    code: typeof value.code === "string" ? value.code : "Pr",
    approvedRoot: typeof value.approvedRoot === "string" ? value.approvedRoot : "",
    isActive: value.isActive === true
  };
}

function notifyProjectChanged(): void {
  window.dispatchEvent(new CustomEvent(PROJECT_CHANGED_EVENT));
  window.dispatchEvent(new CustomEvent(WORKSPACE_CHANGED_EVENT));
}

export async function projectList(): Promise<ProjectDto[]> {
  const response = await invoke<unknown[]>("project_list");
  return response.map(toProjectDto);
}

export async function projectGetActive(): Promise<ProjectDto | null> {
  const response = await invoke<unknown | null>("project_get_active");
  return response === null ? null : toProjectDto(response);
}

export async function projectCreateFromFolder(
  folderPath: string,
  createInitialPane = true
): Promise<ProjectDto> {
  const response = await invoke<unknown>("project_create_from_folder", {
    folderPath,
    createInitialPane
  });
  const project = toProjectDto(response);
  writeActiveProjectId(project.id);
  notifyProjectChanged();
  return project;
}

export async function projectSwitch(projectId: string): Promise<ProjectDto> {
  const response = await invoke<unknown>("project_switch", { projectId });
  const project = toProjectDto(response);
  writeActiveProjectId(project.id);
  await workspaceSwitch(project.workspaceId);
  notifyProjectChanged();
  return project;
}

export async function pickProjectFolder(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select project folder"
  });

  if (selected === null) {
    return null;
  }

  return Array.isArray(selected) ? selected[0] ?? null : selected;
}

export function readActiveProjectId(): string {
  try {
    const stored = window.localStorage.getItem(localActiveProjectKey);
    if (typeof stored === "string" && stored.length > 0) {
      return stored;
    }
  } catch {
    // Fall through to backend-derived state on next reload.
  }

  return "";
}

export function writeActiveProjectId(projectId: string): void {
  try {
    window.localStorage.setItem(localActiveProjectKey, projectId);
  } catch {
    // Persistence is best-effort; backend project_switch remains authoritative.
  }
}