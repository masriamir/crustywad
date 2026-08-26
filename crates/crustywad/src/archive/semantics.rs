//! Pure pk3 path rules (namespace, short name, embedded WAD, map detection),
//! transcribed from `GZDoom`. Filled in by Task 2.

use super::{MapKind, Namespace};

pub(crate) fn directory_of(namespace: Namespace) -> Option<&'static str> {
    let _ = namespace;
    None
}
pub(crate) fn normalize_path(raw: &str) -> String {
    raw.to_string()
}
pub(crate) fn namespace_of(path: &str) -> Namespace {
    let _ = path;
    Namespace::Hidden
}
pub(crate) fn short_name_of(path: &str, namespace: Namespace) -> Option<String> {
    let _ = (path, namespace);
    None
}
pub(crate) fn is_embedded_wad(path: &str, archive_name: Option<&str>) -> bool {
    let _ = (path, archive_name);
    false
}
#[allow(dead_code)] // consumed by `Archive::maps` from Task 6 on
pub(crate) fn map_of(path: &str) -> Option<(String, MapKind)> {
    let _ = path;
    None
}
