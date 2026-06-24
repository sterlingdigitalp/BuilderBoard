use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntryDto {
    pub name: String,
    pub path: String,
    pub entry_type: String,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDirectoryResult {
    pub path: String,
    pub entries: Vec<DirectoryEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReadFileResult {
    pub path: String,
    pub content: String,
    pub size_bytes: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatchLineDto {
    pub line_number: u32,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatchFileDto {
    pub path: String,
    pub matches: Vec<SearchMatchLineDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilesResult {
    pub path: String,
    pub query: String,
    pub matches: Vec<SearchMatchFileDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindFilesResult {
    pub path: String,
    pub pattern: String,
    pub matches: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovedRootResult {
    pub workspace_id: String,
    pub approved_root: Option<String>,
}
