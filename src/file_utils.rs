use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ms(pub u128);

pub fn file_modified_time(path: impl AsRef<Path>) -> Option<Ms> {
    Some(Ms(std::fs::metadata(path.as_ref())
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis()))
}

pub trait FileStemString {
    fn file_stem_string(&self) -> Option<String>;
}

impl<T: AsRef<Path>> FileStemString for T {
    fn file_stem_string(&self) -> Option<String> {
        self.as_ref()
            .file_stem()
            .and_then(|p| p.to_str())
            .map(|p| p.to_string())
    }
}
