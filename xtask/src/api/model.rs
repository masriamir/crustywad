//! API response envelopes and records (DESIGN.md §4.3–§4.4).
//!
//! PII invariant (ADR-0030 §3): [`FileRecord`] has **no `email` field** —
//! the API sends one in every listing record and it is dropped here, at
//! deserialization. Do not add one.

use serde::{Deserialize, Deserializer, Serialize};

/// A collection that a PHP backend may serialize as an array *or* a bare
/// object when it has exactly one element (DESIGN.md §4.4). `Many` must
/// stay first: untagged deserialization tries variants in order, and an
/// object never matches `Vec<T>`.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    /// The normal case: a JSON array.
    Many(Vec<T>),
    /// The PHP quirk: a single bare object.
    One(T),
}

impl<T> OneOrMany<T> {
    /// Flatten to a `Vec` regardless of the wire shape.
    pub fn into_vec(self) -> Vec<T> {
        match self {
            OneOrMany::Many(v) => v,
            OneOrMany::One(x) => vec![x],
        }
    }
}

/// One archive file entry, as it appears in a `getcontents` or
/// `latestfiles` listing (DESIGN.md §4.3). Serialized verbatim into
/// `idgames-files.jsonl` — field order here is the output schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    /// Archive file ID.
    #[serde(deserialize_with = "lenient_u64")]
    pub id: u64,
    /// Nullable on 1994-era records.
    #[serde(default)]
    pub title: Option<String>,
    /// Full directory path; the API sends it with a trailing slash.
    pub dir: String,
    /// Filename only, no path.
    pub filename: String,
    /// Size of the **zip** in bytes (not the WAD size).
    #[serde(deserialize_with = "lenient_u64")]
    pub size: u64,
    /// Unix epoch seconds of archive addition.
    #[serde(deserialize_with = "lenient_i64")]
    pub age: i64,
    /// `YYYY-MM-DD`.
    #[serde(default, deserialize_with = "null_string")]
    pub date: String,
    /// Upload author name, empty when absent.
    #[serde(default, deserialize_with = "null_string")]
    pub author: String,
    /// Author-supplied free text — treated as untrusted, never asserted
    /// PII-free (ADR-0030 §3).
    #[serde(default, deserialize_with = "null_string")]
    pub description: String,
    /// Mean user rating; `None` when unrated.
    #[serde(default, deserialize_with = "lenient_opt_f64")]
    pub rating: Option<f64>,
    /// Number of user votes behind `rating`.
    #[serde(default, deserialize_with = "lenient_u64")]
    pub votes: u64,
    /// doomworld.com frontend URL.
    #[serde(default, deserialize_with = "null_string")]
    pub url: String,
    /// `idgames://` protocol URL.
    #[serde(default, deserialize_with = "null_string")]
    pub idgamesurl: String,
}

/// One `latestfiles` listing record. Observed live (2026-08-15): these are
/// ABBREVIATED — only `id`, `title`, `author`, `description`, `rating`
/// arrive; `dir`/`filename`/`size`/`age`/`date`/URL fields do not. Only
/// `id` is load-bearing (the §4.5 freshness probe). No email field, as
/// everywhere (ADR-0030 §3).
#[derive(Debug, Clone, Deserialize)]
pub struct LatestFileRecord {
    /// Archive file ID — the §4.5 max-id probe value.
    #[serde(deserialize_with = "lenient_u64")]
    pub id: u64,
    /// Nullable title. Captured only to match the observed response shape —
    /// per this struct's doc comment, only `id` is load-bearing for the
    /// freshness probe; flagged as a vestigial-candidate in the #405
    /// final-fix report.
    #[serde(default)]
    #[allow(dead_code)]
    pub title: Option<String>,
    /// Upload author name, empty when absent. See `title`: shape-only, not
    /// load-bearing.
    #[serde(default, deserialize_with = "null_string")]
    #[allow(dead_code)]
    pub author: String,
    /// Author-supplied free text — treated as untrusted, never asserted
    /// PII-free (ADR-0030 §3). See `title`: shape-only, not load-bearing.
    #[serde(default, deserialize_with = "null_string")]
    #[allow(dead_code)]
    pub description: String,
    /// Mean user rating; `None` when unrated. See `title`: shape-only, not
    /// load-bearing (exercised directly by model.rs tests, unread by
    /// production code).
    #[serde(default, deserialize_with = "lenient_opt_f64")]
    #[allow(dead_code)]
    pub rating: Option<f64>,
}

/// One subdirectory entry in a `getcontents` listing.
#[derive(Debug, Clone, Deserialize)]
pub struct DirRecord {
    /// Archive directory ID. Captured for deserialization completeness;
    /// traversal keys subdirectories by path/name (`name`, below), not this
    /// ID, and phase 2 (#406, HTTP range reads) is also path-keyed —
    /// flagged as a vestigial-candidate in the #405 final-fix report.
    #[serde(default, deserialize_with = "lenient_u64")]
    #[allow(dead_code)]
    pub id: u64,
    /// Full path from the archive root.
    pub name: String,
}

/// The `content` object of a listing response. Both collections are
/// tri-state: absent/`null` (`None`), bare object, or array (§4.4).
#[derive(Debug, Deserialize)]
pub struct ContentListing {
    /// File entries; `None` when the key is `null` or absent.
    #[serde(default)]
    pub file: Option<OneOrMany<FileRecord>>,
    /// Subdirectory entries; `None` when the key is `null` or absent.
    #[serde(default)]
    pub dir: Option<OneOrMany<DirRecord>>,
}

impl ContentListing {
    /// §4.1: both collections `null`/absent is indistinguishable from a
    /// nonexistent path — a suspect path, never an empty directory.
    /// Must be checked before [`Self::into_parts`] defaults erase it.
    pub fn is_suspect(&self) -> bool {
        self.file.is_none() && self.dir.is_none()
    }

    /// Flatten to `(files, dirs)`, defaulting absent collections to empty.
    pub fn into_parts(self) -> (Vec<FileRecord>, Vec<DirRecord>) {
        (
            self.file.map(OneOrMany::into_vec).unwrap_or_default(),
            self.dir.map(OneOrMany::into_vec).unwrap_or_default(),
        )
    }
}

/// The API's error envelope payload (spike-verified shape).
#[derive(Debug, Clone, Deserialize)]
pub struct ApiFault {
    /// The API's error class, e.g. `"Required Argument Missing"`.
    #[serde(rename = "type", default)]
    pub kind: String,
    /// The API's human-readable message.
    #[serde(default)]
    pub message: String,
}

/// Failure to interpret a response body as a listing envelope.
#[derive(Debug)]
pub enum EnvelopeError {
    /// The API answered with its error envelope.
    Api(ApiFault),
    /// The body matched neither the success nor the error envelope.
    Shape(String),
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvelopeError::Api(fault) => {
                write!(f, "API error ({}): {}", fault.kind, fault.message)
            }
            EnvelopeError::Shape(msg) => write!(f, "unrecognized envelope: {msg}"),
        }
    }
}

impl std::error::Error for EnvelopeError {}

/// Interpret a response body: `{"error":…}` → [`EnvelopeError::Api`];
/// `{"content":…, "meta":{"version":N}}` → `(N, listing)`; anything else
/// → [`EnvelopeError::Shape`].
pub fn parse_envelope(body: &serde_json::Value) -> Result<(u64, ContentListing), EnvelopeError> {
    if let Some(err) = body.get("error") {
        let fault: ApiFault = serde_json::from_value(err.clone())
            .map_err(|e| EnvelopeError::Shape(format!("error envelope: {e}")))?;
        return Err(EnvelopeError::Api(fault));
    }
    let content = body
        .get("content")
        .ok_or_else(|| EnvelopeError::Shape("no `content` and no `error` key".into()))?;
    let listing: ContentListing = serde_json::from_value(content.clone())
        .map_err(|e| EnvelopeError::Shape(format!("content: {e}")))?;
    let version = body
        .get("meta")
        .and_then(|m| m.get("version"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    Ok((version, listing))
}

/// Content wrapper for a `latestfiles` envelope — mirrors [`ContentListing`]
/// but for the abbreviated [`LatestFileRecord`] shape observed live.
#[derive(Debug, Deserialize)]
struct LatestListing {
    /// File entries; `None` when the key is `null` or absent.
    #[serde(default)]
    file: Option<OneOrMany<LatestFileRecord>>,
}

/// Interpret a `latestfiles` response body: the same envelope handling as
/// [`parse_envelope`] (`{"error":…}` → [`EnvelopeError::Api`];
/// `{"content":…, "meta":{"version":N}}` → `(N, records)`; anything else →
/// [`EnvelopeError::Shape`]), but against the abbreviated
/// [`LatestFileRecord`] shape the live API actually returns for this
/// action (§4.5 — only `id` is load-bearing there).
pub fn parse_latest_envelope(
    body: &serde_json::Value,
) -> Result<(u64, Vec<LatestFileRecord>), EnvelopeError> {
    if let Some(err) = body.get("error") {
        let fault: ApiFault = serde_json::from_value(err.clone())
            .map_err(|e| EnvelopeError::Shape(format!("error envelope: {e}")))?;
        return Err(EnvelopeError::Api(fault));
    }
    let content = body
        .get("content")
        .ok_or_else(|| EnvelopeError::Shape("no `content` and no `error` key".into()))?;
    let listing: LatestListing = serde_json::from_value(content.clone())
        .map_err(|e| EnvelopeError::Shape(format!("content: {e}")))?;
    let version = body
        .get("meta")
        .and_then(|m| m.get("version"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let records = listing.file.map(OneOrMany::into_vec).unwrap_or_default();
    Ok((version, records))
}

/// Guarantee exactly one trailing `/` (mandatory on `getcontents` names,
/// §4.1). Empty input stays empty (the archive root is not addressable).
pub fn normalize_dir(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    format!("{}/", path.trim_end_matches('/'))
}

/// §4.4: PHP's JSON encoder is inconsistent — numbers may arrive as
/// strings. Accept both; also treat `null` as the type's zero (missing
/// data must not fail the record).
fn lenient_u64<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    match serde_json::Value::deserialize(d)? {
        serde_json::Value::Number(n) => n
            .as_u64()
            .ok_or_else(|| serde::de::Error::custom(format!("not a u64: {n}"))),
        serde_json::Value::String(s) => s
            .trim()
            .parse()
            .map_err(|e| serde::de::Error::custom(format!("not a u64: {s:?}: {e}"))),
        serde_json::Value::Null => Ok(0),
        other => Err(serde::de::Error::custom(format!("not a u64: {other}"))),
    }
}

fn lenient_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    match serde_json::Value::deserialize(d)? {
        serde_json::Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| serde::de::Error::custom(format!("not an i64: {n}"))),
        serde_json::Value::String(s) => s
            .trim()
            .parse()
            .map_err(|e| serde::de::Error::custom(format!("not an i64: {s:?}: {e}"))),
        serde_json::Value::Null => Ok(0),
        other => Err(serde::de::Error::custom(format!("not an i64: {other}"))),
    }
}

fn lenient_opt_f64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    match serde_json::Value::deserialize(d)? {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::Number(n) => Ok(n.as_f64()),
        serde_json::Value::String(s) => s
            .trim()
            .parse()
            .map(Some)
            .map_err(|e| serde::de::Error::custom(format!("not an f64: {s:?}: {e}"))),
        other => Err(serde::de::Error::custom(format!("not an f64: {other}"))),
    }
}

/// `null` → `""` (§4.4: missing/null strings default to empty).
fn null_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    fn full_record() -> serde_json::Value {
        json!({
            "id": 12815, "title": "Example", "dir": "levels/doom/0-9/",
            "filename": "example.zip", "size": 552_251, "age": 1_054_512_000,
            "date": "2003-06-02", "author": "Someone", "email": "someone@example.com",
            "description": "A map.", "rating": 4.5, "votes": 12,
            "url": "https://www.doomworld.com/idgames/levels/doom/0-9/example",
            "idgamesurl": "idgames://levels/doom/0-9/example.zip"
        })
    }

    #[test]
    fn one_or_many_many() {
        let v: OneOrMany<u32> = serde_json::from_value(json!([1, 2, 3])).unwrap();
        assert_eq!(v.into_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn one_or_many_single_object() {
        // §4.4: PHP may serialize a one-element list as a bare object.
        let v: OneOrMany<DirRecord> =
            serde_json::from_value(json!({"id": 1, "name": "levels/doom"})).unwrap();
        assert_eq!(v.into_vec().len(), 1);
    }

    #[test]
    fn one_or_many_empty_array() {
        let v: OneOrMany<u32> = serde_json::from_value(json!([])).unwrap();
        assert!(v.into_vec().is_empty());
    }

    #[test]
    fn file_record_drops_email_on_input_and_output() {
        let rec: FileRecord = serde_json::from_value(full_record()).unwrap();
        let out = serde_json::to_value(&rec).unwrap();
        assert_no_email_keys(&out);
        assert_eq!(rec.id, 12815);
        assert_eq!(rec.dir, "levels/doom/0-9/");
    }

    #[test]
    fn lenient_numerics_accept_strings() {
        let mut v = full_record();
        v["size"] = json!("552251");
        v["age"] = json!("1054512000");
        v["votes"] = json!("12");
        v["rating"] = json!("4.5");
        v["id"] = json!("12815");
        let rec: FileRecord = serde_json::from_value(v).unwrap();
        assert_eq!(rec.size, 552_251);
        assert_eq!(rec.age, 1_054_512_000);
        assert_eq!(rec.votes, 12);
        assert_eq!(rec.rating, Some(4.5));
        assert_eq!(rec.id, 12815);
    }

    #[test]
    fn nullable_and_missing_fields_default() {
        // §4.4: missing/null strings default to empty rather than failing the record.
        let rec: FileRecord = serde_json::from_value(json!({
            "id": 3, "dir": "levels/doom/0-9/", "filename": "old.zip",
            "size": 100, "age": 0,
            "title": null, "author": null, "rating": null
        }))
        .unwrap();
        assert_eq!(rec.title, None);
        assert_eq!(rec.author, "");
        assert_eq!(rec.date, "");
        assert_eq!(rec.rating, None);
        assert_eq!(rec.votes, 0);
    }

    #[test]
    fn listing_null_collections_are_suspect() {
        // §4.1: both fields null == suspect path, detected on the raw Options.
        let l: ContentListing = serde_json::from_value(json!({"file": null, "dir": null})).unwrap();
        assert!(l.is_suspect());
        let l: ContentListing =
            serde_json::from_value(json!({"dir": null, "file": [full_record()]})).unwrap();
        assert!(!l.is_suspect());
        let (files, dirs) = l.into_parts();
        assert_eq!(files.len(), 1);
        assert!(dirs.is_empty());
    }

    #[test]
    fn listing_absent_keys_are_suspect_too() {
        // Absent key and explicit null both deserialize to None.
        let l: ContentListing = serde_json::from_value(json!({})).unwrap();
        assert!(l.is_suspect());
    }

    #[test]
    fn parse_envelope_success() {
        let body = json!({
            "content": {"file": [full_record()], "dir": null},
            "meta": {"version": 3}
        });
        let (version, listing) = parse_envelope(&body).unwrap();
        assert_eq!(version, 3);
        assert_eq!(listing.into_parts().0.len(), 1);
    }

    #[test]
    fn parse_envelope_error() {
        let body = json!({
            "error": {"type": "Required Argument Missing", "message": "The name argument is missing."},
            "meta": {"version": 3}
        });
        match parse_envelope(&body) {
            Err(EnvelopeError::Api(f)) => assert!(f.message.contains("missing")),
            other => panic!("expected Api fault, got {other:?}"),
        }
    }

    #[test]
    fn parse_envelope_rejects_shapeless_body() {
        assert!(matches!(
            parse_envelope(&json!({"meta": {"version": 3}})),
            Err(EnvelopeError::Shape(_))
        ));
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let mut v = full_record();
        v["textfile"] = json!("tolerated");
        let rec: FileRecord = serde_json::from_value(v).unwrap();
        assert_eq!(rec.filename, "example.zip");
    }

    #[test]
    fn latest_records_parse_the_observed_abbreviated_payload() {
        // Observed live 2026-08-15: `latestfiles` records are abbreviated —
        // only id/title/author/description/rating arrive. This also locks
        // in the bare-object OneOrMany path (limit=1 returns a bare object,
        // not a one-element array).
        let body = json!({
            "content": {"file": {
                "id": 22083, "title": "Sacco", "author": "Willis Lambert",
                "description": "Probably way too many monster closets.", "rating": null
            }},
            "meta": {"version": 3}
        });
        let (version, records) = parse_latest_envelope(&body).unwrap();
        assert_eq!(version, 3);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, 22083);
        assert_eq!(records[0].rating, None);
    }

    #[test]
    fn latest_records_array_form_ignores_unknown_email_key() {
        let body = json!({
            "content": {"file": [
                {
                    "id": 22083, "title": "Sacco", "author": "Willis Lambert",
                    "description": "Probably way too many monster closets.", "rating": null,
                    "email": "willis@example.com"
                },
                {
                    "id": 22082, "title": "Testing Facility", "author": "Dashy",
                    "description": "This is my first map!", "rating": null
                }
            ]},
            "meta": {"version": 3}
        });
        let (_version, records) = parse_latest_envelope(&body).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, 22083);
        assert_eq!(records[1].id, 22082);
        // No `email` field exists on `LatestFileRecord` for the key to land
        // in (ADR-0030 §3) — serde's default unknown-field tolerance simply
        // drops it, and the record still parses.
    }

    #[test]
    fn parse_latest_envelope_error() {
        let body = json!({
            "error": {"type": "Required Argument Missing", "message": "The name argument is missing."},
            "meta": {"version": 3}
        });
        match parse_latest_envelope(&body) {
            Err(EnvelopeError::Api(f)) => assert!(f.message.contains("missing")),
            other => panic!("expected Api fault, got {other:?}"),
        }
    }

    #[test]
    fn normalize_dir_appends_exactly_one_slash() {
        assert_eq!(normalize_dir("levels/doom"), "levels/doom/");
        assert_eq!(normalize_dir("levels/doom/"), "levels/doom/");
        assert_eq!(normalize_dir(""), "");
    }

    /// Shared assertion: no object key anywhere in `v` contains "email"
    /// (case-insensitive). ADR-0030 §3.
    pub(crate) fn assert_no_email_keys(v: &serde_json::Value) {
        match v {
            serde_json::Value::Object(map) => {
                for (k, val) in map {
                    assert!(
                        !k.to_ascii_lowercase().contains("email"),
                        "email-shaped key {k:?} present"
                    );
                    assert_no_email_keys(val);
                }
            }
            serde_json::Value::Array(items) => items.iter().for_each(assert_no_email_keys),
            _ => {}
        }
    }
}
