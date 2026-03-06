use std::path::PathBuf;

pub trait PlatformFileDialog {
    fn open_file(&self, title: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf>;
    fn save_file(
        &self,
        title: &str,
        default_name: &str,
        filters: &[(&str, &[&str])],
    ) -> Option<PathBuf>;
    fn open_directory(&self, title: &str) -> Option<PathBuf>;
}

pub struct NativeFileDialog;

impl NativeFileDialog {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NativeFileDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformFileDialog for NativeFileDialog {
    fn open_file(&self, title: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new().set_title(title);
        for (name, extensions) in filters {
            dialog = dialog.add_filter(*name, extensions);
        }
        dialog.pick_file()
    }

    fn save_file(
        &self,
        title: &str,
        default_name: &str,
        filters: &[(&str, &[&str])],
    ) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new()
            .set_title(title)
            .set_file_name(default_name);
        for (name, extensions) in filters {
            dialog = dialog.add_filter(*name, extensions);
        }
        dialog.save_file()
    }

    fn open_directory(&self, title: &str) -> Option<PathBuf> {
        rfd::FileDialog::new().set_title(title).pick_folder()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubDialog;
    impl PlatformFileDialog for StubDialog {
        fn open_file(&self, _title: &str, _filters: &[(&str, &[&str])]) -> Option<PathBuf> {
            None
        }
        fn save_file(
            &self,
            _title: &str,
            _default_name: &str,
            _filters: &[(&str, &[&str])],
        ) -> Option<PathBuf> {
            None
        }
        fn open_directory(&self, _title: &str) -> Option<PathBuf> {
            None
        }
    }

    #[test]
    fn stub_returns_none() {
        let dialog = StubDialog;
        assert!(dialog.open_file("Open", &[]).is_none());
        assert!(dialog.save_file("Save", "file.txt", &[]).is_none());
        assert!(dialog.open_directory("Dir").is_none());
    }
}
