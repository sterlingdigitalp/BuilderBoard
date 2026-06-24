import { invoke } from "@tauri-apps/api/core";
import type {
  ApprovedRootResult,
  FindFilesResult,
  ListDirectoryResult,
  ReadFileResult,
  SearchFilesResult
} from "../types/filesystem";

export async function filesystemSetApprovedRoot(
  path: string,
  workspaceId?: string
): Promise<string> {
  return invoke<string>("filesystem_set_approved_root", { workspaceId, path });
}

export async function filesystemGetApprovedRoot(
  workspaceId?: string
): Promise<ApprovedRootResult> {
  return invoke<ApprovedRootResult>("filesystem_get_approved_root", { workspaceId });
}

export async function filesystemListDirectory(
  path: string,
  workspaceId?: string
): Promise<ListDirectoryResult> {
  return invoke<ListDirectoryResult>("filesystem_list_directory", { workspaceId, path });
}

export async function filesystemReadFile(
  path: string,
  workspaceId?: string
): Promise<ReadFileResult> {
  return invoke<ReadFileResult>("filesystem_read_file", { workspaceId, path });
}

export async function filesystemSearchFiles(
  path: string,
  query: string,
  workspaceId?: string
): Promise<SearchFilesResult> {
  return invoke<SearchFilesResult>("filesystem_search_files", { workspaceId, path, query });
}

export async function filesystemFindFiles(
  path: string,
  pattern: string,
  workspaceId?: string
): Promise<FindFilesResult> {
  return invoke<FindFilesResult>("filesystem_find_files", { workspaceId, path, pattern });
}