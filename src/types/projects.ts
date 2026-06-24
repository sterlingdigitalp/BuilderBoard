export interface ProjectDto {
  id: string;
  workspaceId: string;
  name: string;
  code: string;
  approvedRoot: string;
  isActive: boolean;
}