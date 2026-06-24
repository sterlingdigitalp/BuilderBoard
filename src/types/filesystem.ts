export interface DirectoryEntryDto {
  name: string;
  path: string;
  entryType: string;
  sizeBytes: number | null;
  modifiedAt: string | null;
}

export interface ListDirectoryResult {
  path: string;
  entries: DirectoryEntryDto[];
}

export interface ReadFileResult {
  path: string;
  content: string;
  sizeBytes: number;
  truncated: boolean;
}

export interface SearchMatchLineDto {
  lineNumber: number;
  line: string;
}

export interface SearchMatchFileDto {
  path: string;
  matches: SearchMatchLineDto[];
}

export interface SearchFilesResult {
  path: string;
  query: string;
  matches: SearchMatchFileDto[];
}

export interface FindFilesResult {
  path: string;
  pattern: string;
  matches: string[];
}

export interface ApprovedRootResult {
  workspaceId: string;
  approvedRoot: string | null;
}