//! pk3 (zip) resource-archive reading (ADR-0031).
//!
//! A pk3 is a zip whose directory layout carries meaning for ZDoom-family
//! engines: the first path component selects a lump [`Namespace`], the
//! basename becomes an 8-character short name, `maps/<NAME>.wad` members hold
//! one map each, and a `.wad` at the archive root is an *embedded WAD* the
//! engine loads recursively. [`Archive`] models exactly that much — container
//! plus maps — and hands out [`Wad`] values for the WADs it contains; it does
//! not resolve names across members. The rules are transcribed from `GZDoom`'s
//! `filesystem.cpp` (`LumpRecord::SetFromLump`), `resourcefile.cpp`
//! (`FResourceFile::CheckEmbedded`), and `p_openmap.cpp`.
//!
//! Nothing is decoded when an archive is opened: the central directory alone
//! decides what is listed, and every allocation is bounded by
//! [`Limits`](crate::Limits) — `max_archive_members` for the member table and
//! `max_decoded_member_bytes` for a single [`Archive::read`] (ADR-0016).
//! Only the stored and deflate methods are decoded; every other method, and
//! any encrypted member, is rejected by name. The container seam is private,
//! so a future pk7 (7z) backend can slot in without a public change; today
//! the 7z signature is recognized only to produce
//! [`ArchiveError::UnsupportedContainer`].

mod error;
mod semantics;
mod zip;

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

pub use error::{ArchiveError, ArchiveWarning};

use crate::{ParseOptions, Strictness, Wad};

/// Container formats an [`Archive`] can be backed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ArchiveKind {
    /// A zip file (`.pk3`, `.zip`, `.pke`, `.ipk3`).
    Zip,
}

/// Container formats recognized by their signature but not decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContainerKind {
    /// A 7-Zip container (`.pk7`), signature `7z\xbc\xaf\x27\x1c`.
    Pk7,
}

impl fmt::Display for ContainerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pk7 => f.write_str("pk7 (7z)"),
        }
    }
}

/// A zip compression method, as recorded in the central directory.
///
/// Only [`Stored`](Self::Stored) and [`Deflate`](Self::Deflate) can be read;
/// the other named variants exist so an error can say *which* unsupported
/// method a member uses. Codes follow APPNOTE §4.4.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Method {
    /// Method 0: no compression.
    Stored,
    /// Method 8: raw deflate.
    Deflate,
    /// Method 1 (PKZIP 1.x Shrink).
    Shrink,
    /// Method 6 (PKZIP Implode).
    Implode,
    /// Method 12.
    Bzip2,
    /// Method 14.
    Lzma,
    /// Method 95.
    Xz,
    /// Method 98.
    Ppmd,
    /// Any other method code.
    Other(u16),
}

impl Method {
    /// Maps a central-directory method code to a variant.
    #[must_use]
    pub fn from_code(code: u16) -> Self {
        match code {
            0 => Self::Stored,
            8 => Self::Deflate,
            1 => Self::Shrink,
            6 => Self::Implode,
            12 => Self::Bzip2,
            14 => Self::Lzma,
            95 => Self::Xz,
            98 => Self::Ppmd,
            other => Self::Other(other),
        }
    }

    /// Whether this crate can decode the method.
    #[must_use]
    pub fn is_supported(self) -> bool {
        matches!(self, Self::Stored | Self::Deflate)
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stored => f.write_str("stored"),
            Self::Deflate => f.write_str("deflate"),
            Self::Shrink => f.write_str("shrink"),
            Self::Implode => f.write_str("implode"),
            Self::Bzip2 => f.write_str("bzip2"),
            Self::Lzma => f.write_str("lzma"),
            Self::Xz => f.write_str("xz"),
            Self::Ppmd => f.write_str("ppmd"),
            Self::Other(code) => write!(f, "#{code}"),
        }
    }
}

/// The lump namespace a member's directory places it in, per `GZDoom`'s
/// directory table (`filesystem.cpp`, `LumpRecord::SetFromLump`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Namespace {
    /// A file at the archive root (no `/` in its path).
    Global,
    /// `flats/`.
    Flats,
    /// `textures/`.
    Textures,
    /// `hires/`.
    Hires,
    /// `sprites/`.
    Sprites,
    /// `voxels/`.
    Voxels,
    /// `colormaps/`.
    Colormaps,
    /// `acs/`.
    Acs,
    /// `voices/`.
    Voices,
    /// `patches/`.
    Patches,
    /// `graphics/`.
    Graphics,
    /// `sounds/`.
    Sounds,
    /// `music/`.
    Music,
    /// Any other directory (`maps/`, `zscript/`, `decorate/`, wrapper folders…);
    /// such members have no short name.
    Hidden,
}

impl Namespace {
    /// The lowercase directory name this namespace maps from, or `None` for
    /// [`Global`](Self::Global) and [`Hidden`](Self::Hidden).
    #[must_use]
    pub fn directory(self) -> Option<&'static str> {
        semantics::directory_of(self)
    }
}

/// How a `maps/` member stores its map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MapKind {
    /// `maps/<NAME>.wad` — a WAD holding the map's lumps; parse it with
    /// [`Archive::wad`].
    Wad,
    /// `maps/<NAME>.map` — a bare UDMF `TEXTMAP`. Listed, not parsed.
    Textmap,
}

/// One map found under `maps/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveMap {
    name: String,
    kind: MapKind,
    member_index: usize,
}

impl ArchiveMap {
    /// The map name, uppercased (`MAP01`, `E1M1`, `MYMAP`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the member is a WAD or a bare `TEXTMAP`.
    #[must_use]
    pub fn kind(&self) -> MapKind {
        self.kind
    }

    /// Index of the member in [`Archive::members`].
    #[must_use]
    pub fn member_index(&self) -> usize {
        self.member_index
    }
}

/// Distinguishes members of one archive from another's (see
/// [`ArchiveError::ForeignMember`]).
static NEXT_ARCHIVE_ID: AtomicU64 = AtomicU64::new(1);

/// One file inside an [`Archive`].
///
/// A `Member` is only valid against the [`Archive`] that produced it:
/// [`Archive::read`] and [`Archive::wad`] answer
/// [`ArchiveError::ForeignMember`] for a member from any other archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    path: String,
    short_name: Option<String>,
    namespace: Namespace,
    size: u64,
    compressed_size: u64,
    method: Method,
    encrypted: bool,
    embedded_wad: bool,
    index: usize,
    entry: usize,
    archive_id: u64,
}

impl Member {
    /// The member's path inside the archive, normalized (`\` → `/`, no
    /// leading `/`), original case preserved.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The 8-character uppercase short name an engine would give this file
    /// (basename, extension stripped), or `None` for a
    /// [`Namespace::Hidden`] member or a non-ASCII path.
    #[must_use]
    pub fn short_name(&self) -> Option<&str> {
        self.short_name.as_deref()
    }

    /// The namespace its directory selects.
    #[must_use]
    pub fn namespace(&self) -> Namespace {
        self.namespace
    }

    /// The declared uncompressed size in bytes.
    #[must_use]
    pub fn size(&self) -> u64 {
        self.size
    }

    /// The stored (compressed) size in bytes.
    #[must_use]
    pub fn compressed_size(&self) -> u64 {
        self.compressed_size
    }

    /// The compression method.
    #[must_use]
    pub fn method(&self) -> Method {
        self.method
    }

    /// Whether the member is encrypted (and therefore unreadable).
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    /// Whether `GZDoom` would load this member as an embedded WAD: a `.wad` at
    /// the archive root, or `<archive-stem>/<file>.wad` when the archive's
    /// name is known (see [`Archive::with_name`]).
    #[must_use]
    pub fn is_embedded_wad(&self) -> bool {
        self.embedded_wad
    }

    /// Position in [`Archive::members`].
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }
}

/// A raw container entry, before pk3 semantics are applied.
#[derive(Debug, Clone)]
pub(crate) struct RawEntry {
    /// Normalized path (`\` → `/`, no leading `/`).
    pub(crate) path: String,
    /// Whether the raw name bytes were valid UTF-8 (`false` → decoded lossily).
    pub(crate) utf8: bool,
    pub(crate) method: u16,
    pub(crate) flags: u16,
    pub(crate) crc32: u32,
    pub(crate) compressed_size: u64,
    pub(crate) size: u64,
    pub(crate) local_header_offset: u64,
}

/// The container seam: a format that yields raw entries and decodes one on
/// demand. Private so a future 7z backend changes nothing public.
///
/// `Send + Sync` are supertraits so `Box<dyn Container>` — and therefore
/// [`Archive`] — is `Send + Sync` too, the way [`Wad`] is: an archive is an
/// owned, immutable buffer plus a member table, so it can be handed to another
/// thread or shared behind an `Arc`. Any future backend must keep that
/// property (`ZipContainer` holds only a `Vec<u8>` and a `Vec<RawEntry>`).
pub(crate) trait Container: fmt::Debug + Send + Sync {
    /// Every non-directory entry, in central-directory order.
    fn entries(&self) -> &[RawEntry];
    /// Decodes entry `index`, refusing to produce more than `cap` bytes.
    fn read_entry(&self, index: usize, cap: usize) -> Result<Vec<u8>, ArchiveError>;
}

/// A pk3 (zip) archive: its member table plus the pk3 semantics `GZDoom`
/// applies to it. See the [module docs](self).
#[derive(Debug)]
pub struct Archive {
    container: Box<dyn Container>,
    id: u64,
    kind: ArchiveKind,
    name: Option<String>,
    members: Vec<Member>,
    options: ParseOptions,
    warnings: Vec<ArchiveWarning>,
}

const ZIP_LOCAL_SIG: &[u8] = b"PK\x03\x04";
const ZIP_EMPTY_SIG: &[u8] = b"PK\x05\x06";
const ZIP_SPANNED_SIG: &[u8] = b"PK\x07\x08";
const SEVEN_ZIP_SIG: &[u8] = b"7z\xbc\xaf\x27\x1c";

impl Archive {
    /// Opens an archive from bytes with the default (strict) options.
    ///
    /// # Errors
    ///
    /// See [`from_bytes_with_options`](Self::from_bytes_with_options).
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, ArchiveError> {
        Self::from_bytes_with_options(bytes, ParseOptions::default())
    }

    /// Opens an archive from bytes.
    ///
    /// The container is identified by its leading signature, never by a file
    /// extension; the central directory is parsed and every member is
    /// listed, but nothing is decoded until [`read`](Self::read). The
    /// archive's own name is unknown here, so the `<stem>/<file>.wad`
    /// embedded-WAD rule cannot fire until [`with_name`](Self::with_name).
    ///
    /// # Errors
    ///
    /// - [`ArchiveError::NotAnArchive`], [`EmptyArchive`](ArchiveError::EmptyArchive),
    ///   [`SpannedArchive`](ArchiveError::SpannedArchive), or
    ///   [`UnsupportedContainer`](ArchiveError::UnsupportedContainer) for the
    ///   leading bytes; `NotAnArchive` also when no end-of-central-directory
    ///   record is found.
    /// - [`CorruptDirectory`](ArchiveError::CorruptDirectory) and
    ///   [`TooManyMembers`](ArchiveError::TooManyMembers) in both modes.
    /// - In strict mode only: [`UnsupportedMethod`](ArchiveError::UnsupportedMethod),
    ///   [`Encrypted`](ArchiveError::Encrypted),
    ///   [`MemberTooLarge`](ArchiveError::MemberTooLarge), and
    ///   [`NonAsciiName`](ArchiveError::NonAsciiName); lenient mode lists
    ///   the member and records the matching [`ArchiveWarning`].
    /// - A duplicate member path is never an error in either mode: both
    ///   members are kept and the later one wins [`member`](Self::member)
    ///   lookups; lenient mode additionally records
    ///   [`ArchiveWarning::DuplicatePath`].
    pub fn from_bytes_with_options(
        bytes: impl Into<Vec<u8>>,
        options: ParseOptions,
    ) -> Result<Self, ArchiveError> {
        let bytes: Vec<u8> = bytes.into();
        if bytes.starts_with(SEVEN_ZIP_SIG) {
            return Err(ArchiveError::UnsupportedContainer(ContainerKind::Pk7));
        }
        if bytes.starts_with(ZIP_EMPTY_SIG) {
            return Err(ArchiveError::EmptyArchive);
        }
        if bytes.starts_with(ZIP_SPANNED_SIG) {
            return Err(ArchiveError::SpannedArchive);
        }
        if !bytes.starts_with(ZIP_LOCAL_SIG) {
            return Err(ArchiveError::NotAnArchive);
        }
        let container = zip::ZipContainer::open(bytes, options.limits.max_archive_members)?;
        Self::assemble(Box::new(container), ArchiveKind::Zip, options)
    }

    /// Reads `path` into memory and opens it; the file stem becomes the
    /// archive [`name`](Self::name).
    ///
    /// # Errors
    ///
    /// [`ArchiveError::Io`] if the file cannot be read, then everything
    /// [`from_bytes_with_options`](Self::from_bytes_with_options) reports.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ArchiveError> {
        Self::from_path_with_options(path, ParseOptions::default())
    }

    /// [`from_path`](Self::from_path) with explicit options.
    ///
    /// # Errors
    ///
    /// As [`from_path`](Self::from_path).
    pub fn from_path_with_options(
        path: impl AsRef<Path>,
        options: ParseOptions,
    ) -> Result<Self, ArchiveError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| ArchiveError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let archive = Self::from_bytes_with_options(bytes, options)?;
        Ok(match path.file_stem().and_then(|s| s.to_str()) {
            Some(stem) => archive.with_name(stem),
            None => archive,
        })
    }

    /// Sets the archive's name (its file stem, e.g. `myproject` for
    /// `myproject.pk3`) and re-derives every member's embedded-WAD flag, so
    /// `<stem>/<file>.wad` members count as embedded (`GZDoom`'s
    /// `IsFileInFolder` accommodation for mispackaged archives).
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        let name = self.name.as_deref();
        for member in &mut self.members {
            member.embedded_wad = semantics::is_embedded_wad(&member.path, name);
        }
        self
    }

    /// The container format.
    #[must_use]
    pub fn kind(&self) -> ArchiveKind {
        self.kind
    }

    /// The archive's name (file stem), if known.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Every member in central-directory order.
    #[must_use]
    pub fn members(&self) -> &[Member] {
        &self.members
    }

    /// Every `maps/<NAME>.wad` and `maps/<NAME>.map` member, in directory
    /// order, with `NAME` uppercased. Nothing is parsed; pass the member at
    /// [`ArchiveMap::member_index`] to [`wad`](Self::wad) for a
    /// [`MapKind::Wad`] entry.
    #[must_use]
    pub fn maps(&self) -> Vec<ArchiveMap> {
        self.members
            .iter()
            .filter_map(|m| {
                semantics::map_of(&m.path).map(|(name, kind)| ArchiveMap {
                    name,
                    kind,
                    member_index: m.index,
                })
            })
            .collect()
    }

    /// Members `GZDoom` would load as embedded WADs — see
    /// [`Member::is_embedded_wad`].
    #[must_use]
    pub fn embedded_wads(&self) -> Vec<&Member> {
        self.members.iter().filter(|m| m.embedded_wad).collect()
    }

    /// Finds a member by full path, ASCII-case-insensitively; `\` is accepted
    /// for `/` and a leading `/` is ignored. When duplicates exist the later
    /// entry wins, as in `GZDoom`.
    #[must_use]
    pub fn member(&self, path: &str) -> Option<&Member> {
        let wanted = semantics::normalize_path(path);
        self.members
            .iter()
            .rev()
            .find(|m| m.path.eq_ignore_ascii_case(&wanted))
    }

    /// Warnings recorded while opening in lenient mode (empty in strict mode).
    #[must_use]
    pub fn warnings(&self) -> &[ArchiveWarning] {
        &self.warnings
    }

    /// Decodes a member's bytes.
    ///
    /// # Errors
    ///
    /// [`ArchiveError::ForeignMember`] when `member` came from a different
    /// archive — members are only valid against the archive that produced
    /// them. [`ArchiveError::Encrypted`] /
    /// [`UnsupportedMethod`](ArchiveError::UnsupportedMethod)
    /// / [`MemberTooLarge`](ArchiveError::MemberTooLarge) for members that
    /// lenient mode listed but cannot read; and, in both modes,
    /// [`CorruptDirectory`](ArchiveError::CorruptDirectory) when the local
    /// header disagrees with the central directory,
    /// [`CorruptStream`](ArchiveError::CorruptStream),
    /// [`SizeMismatch`](ArchiveError::SizeMismatch), and
    /// [`ChecksumMismatch`](ArchiveError::ChecksumMismatch).
    pub fn read(&self, member: &Member) -> Result<Vec<u8>, ArchiveError> {
        // A `Member` carries the identity of the archive that produced it, so
        // a member from another archive is refused by name instead of silently
        // reading whatever sits at that entry index here.
        if member.archive_id != self.id {
            return Err(ArchiveError::ForeignMember {
                path: member.path.clone(),
            });
        }
        if member.encrypted {
            return Err(ArchiveError::Encrypted {
                path: member.path.clone(),
            });
        }
        if !member.method.is_supported() {
            return Err(ArchiveError::UnsupportedMethod {
                path: member.path.clone(),
                method: member.method,
            });
        }
        let cap = self.options.limits.max_decoded_member_bytes;
        if member.size > cap as u64 {
            return Err(ArchiveError::MemberTooLarge {
                path: member.path.clone(),
                declared: member.size,
                limit: cap,
            });
        }
        self.container.read_entry(member.entry, cap)
    }

    /// Decodes a `.wad` member and parses it with the archive's own
    /// [`ParseOptions`], so a pk3's maps parse exactly like a standalone WAD.
    ///
    /// # Errors
    ///
    /// [`ArchiveError::NotAWad`] when the member's path does not end in
    /// `.wad`; [`ArchiveError::ForeignMember`] when `member` came from a
    /// different archive; everything [`read`](Self::read) reports; and
    /// [`ArchiveError::Wad`] wrapping the [`ParseError`](crate::ParseError)
    /// when the bytes are not a valid WAD.
    pub fn wad(&self, member: &Member) -> Result<Wad, ArchiveError> {
        // Byte comparison via the shared helper — a `&str` byte-index slice
        // panics when a multi-byte character straddles the boundary.
        if !semantics::has_wad_extension(&member.path) {
            return Err(ArchiveError::NotAWad {
                path: member.path.clone(),
            });
        }
        let bytes = self.read(member)?;
        Wad::from_bytes_with_options(bytes, self.options).map_err(|source| ArchiveError::Wad {
            path: member.path.clone(),
            source,
        })
    }

    /// Builds the member table from raw entries, applying the open-time
    /// strictness policy (ADR-0031 §6).
    fn assemble(
        container: Box<dyn Container>,
        kind: ArchiveKind,
        options: ParseOptions,
    ) -> Result<Self, ArchiveError> {
        let id = NEXT_ARCHIVE_ID.fetch_add(1, Ordering::Relaxed);
        let strict = options.strictness == Strictness::Strict;
        let limit = options.limits.max_decoded_member_bytes;
        let mut warnings = Vec::new();
        let mut members = Vec::with_capacity(container.entries().len());
        for (entry_index, raw) in container.entries().iter().enumerate() {
            let method = Method::from_code(raw.method);
            let encrypted = raw.flags & 0x0001 != 0;
            let ascii = raw.utf8 && raw.path.is_ascii();
            if !ascii {
                if strict {
                    return Err(ArchiveError::NonAsciiName {
                        path: raw.path.clone(),
                    });
                }
                warnings.push(ArchiveWarning::NonAsciiName {
                    path: raw.path.clone(),
                });
            }
            if !method.is_supported() {
                if strict {
                    return Err(ArchiveError::UnsupportedMethod {
                        path: raw.path.clone(),
                        method,
                    });
                }
                warnings.push(ArchiveWarning::UnreadableMember {
                    path: raw.path.clone(),
                    reason: format!("unsupported compression method {method}"),
                });
            }
            if encrypted {
                if strict {
                    return Err(ArchiveError::Encrypted {
                        path: raw.path.clone(),
                    });
                }
                warnings.push(ArchiveWarning::UnreadableMember {
                    path: raw.path.clone(),
                    reason: "encrypted".to_string(),
                });
            }
            if raw.size > limit as u64 {
                if strict {
                    return Err(ArchiveError::MemberTooLarge {
                        path: raw.path.clone(),
                        declared: raw.size,
                        limit,
                    });
                }
                warnings.push(ArchiveWarning::MemberTooLarge {
                    path: raw.path.clone(),
                    declared: raw.size,
                    limit,
                });
            }
            let namespace = semantics::namespace_of(&raw.path);
            let short_name = if ascii {
                semantics::short_name_of(&raw.path, namespace)
            } else {
                None
            };
            members.push(Member {
                path: raw.path.clone(),
                short_name,
                namespace,
                size: raw.size,
                compressed_size: raw.compressed_size,
                method,
                encrypted,
                embedded_wad: semantics::is_embedded_wad(&raw.path, None),
                index: members.len(),
                entry: entry_index,
                archive_id: id,
            });
        }
        // Duplicate paths: zips permit them, and GZDoom keeps the later
        // entry, so this is never an error in either mode. Both members stay
        // in the table; `member` (which searches in reverse) resolves
        // lookups to the later one. Lenient mode additionally records a
        // warning. A `HashSet` keeps this O(n) over up to
        // `max_archive_members` entries (ADR-0016 §1); `insert` returning
        // `false` means the lowercased path was already present.
        let mut seen: HashSet<String> = HashSet::with_capacity(members.len());
        for member in &members {
            let lower = member.path.to_ascii_lowercase();
            if !seen.insert(lower) && !strict {
                warnings.push(ArchiveWarning::DuplicatePath {
                    path: member.path.clone(),
                });
            }
        }
        Ok(Self {
            container,
            id,
            kind,
            name: None,
            members,
            options,
            warnings,
        })
    }
}
