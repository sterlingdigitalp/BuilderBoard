use crate::filesystem_tools::ignore::{is_default_ignored_dir, prompt_explicitly_allows_ignored};

pub const MAX_FILES_SCANNED: usize = 5_000;

#[derive(Debug, Clone)]
pub struct ScanContext {
    pub respect_ignore_list: bool,
    pub files_scanned: usize,
    pub scan_limit_reached: bool,
}

impl ScanContext {
    pub fn for_tool(prompt: &str, scan_path: &str) -> Self {
        Self {
            respect_ignore_list: !prompt_explicitly_allows_ignored(prompt, scan_path),
            files_scanned: 0,
            scan_limit_reached: false,
        }
    }

    pub fn direct_command(scan_path: &str) -> Self {
        Self {
            respect_ignore_list: !prompt_explicitly_allows_ignored("", scan_path),
            files_scanned: 0,
            scan_limit_reached: false,
        }
    }

    pub fn should_skip_directory(&self, name: &str) -> bool {
        self.respect_ignore_list && is_default_ignored_dir(name)
    }

    pub fn record_scan(&mut self) -> bool {
        self.files_scanned += 1;
        if self.files_scanned >= MAX_FILES_SCANNED {
            self.scan_limit_reached = true;
            return false;
        }
        true
    }
}
