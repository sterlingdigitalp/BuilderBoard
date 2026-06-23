export type ThemeMode = "light" | "dark";

export type PaneStatus = "idle" | "streaming" | "error";

// Frontend mirror of the persistence-backed PaneDto returned by Tauri pane commands.
export interface PaneDto {
  id: string;
  workspaceId: string;
  title: string | null;
  roleLabel: string | null;
  sortOrder: number;
  widthRatio: number | null;
  heightRatio: number | null;
  providerId: string | null;
  accountId: string | null;
  modelId: string | null;
  status: PaneStatus;
  layoutJson: string | null;
  metadataJson: string | null;
}

export type PaneDefinition = PaneDto;
