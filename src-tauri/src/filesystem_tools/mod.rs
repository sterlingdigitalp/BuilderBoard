pub mod approved_root;
pub mod commands;
pub mod error;
pub mod ignore;
pub mod models;
pub mod perf;
pub mod scan_context;
pub mod scope;
pub mod service;

pub use commands::{
    filesystem_find_files_with_database, filesystem_get_approved_root_with_database,
    filesystem_list_directory_with_database, filesystem_read_file_with_database,
    filesystem_search_files_with_database, filesystem_set_approved_root_with_database,
};
pub use error::FilesystemError;
pub use ignore::{is_default_ignored_dir, DEFAULT_IGNORED_DIRS};
pub use models::{
    ApprovedRootResult, DirectoryEntryDto, FindFilesResult, ListDirectoryResult, ReadFileResult,
    SearchFilesResult,
};
pub use perf::{trace_perf_metric, PerfSpan};
pub use scan_context::ScanContext;
pub use scope::ApprovedScope;
pub use service::FilesystemService;
