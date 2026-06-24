export interface WorkspaceDto {
  id: string;
  name: string;
  slug: string;
  isDefault: boolean;
  layoutJson: string | null;
  metadataJson: string | null;
  createdAt: string;
  updatedAt: string;
  isLocalOnly?: boolean;
}

export interface WorkspaceCreateInput {
  name: string;
}
