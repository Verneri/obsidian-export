/// Returns the relative path of `file` from `root`.
///
/// # Panics
///
/// Panics if `file` is not nested under `root`.
#[must_use]
pub fn relative_path(root: &std::path::Path, file: &std::path::Path) -> std::path::PathBuf {
    file.strip_prefix(root)
        .expect("file should always be nested under root")
        .to_path_buf()
}
