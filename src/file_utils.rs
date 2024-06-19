use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
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
