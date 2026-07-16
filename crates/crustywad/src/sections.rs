//! Marker-delimited directory sections (ADR-0022 §2, issue #280).
//!
//! Classic WADs bracket flats/sprites/patches between zero-size marker
//! lumps (`F_START..F_END`, ...); Doom 64 WADs bracket sprites, world
//! textures, and sounds (`S_`/`T_`/`DS_START..END`). Both reference
//! engines derive section extents by unguarded subtraction of two
//! independently looked-up marker names; this module replaces that
//! anti-pattern with a validated scan (strict errors / lenient warnings
//! per the ADR).
//!
//! Grammar (recognition is by NAME only — no engine checks marker sizes):
//! base pairs `S_`/`F_`/`P_`/`T_`/`G_`/`DS_` + `START`/`END`; Boom's
//! doubled-first-letter aliases for single-character prefixes only
//! (`FF_`/`PP_`/`SS_` — per `PrBoom+` `w_wad.c` `IsMarker`: "FF_* is valid
//! alias for F_*, but HI_* should not allow HHI_*"); numbered sub-pairs
//! `F1_`-`F9_`/`P1_`-`P9_`/`S1_`-`S9_` (retail IWADs nest `F1_`/`F2_`
//! (/`F3_`) and `P1_`-`P3_`; sprites never nest in retail). `CHECKSUM`
//! and `ENDOFWAD` (Doom 64 trailers) are recognized and ignored.
//!
//! Scope is a SINGLE WAD's directory; multi-WAD load-order overlay is
//! tracked on the editor epic's lump/resource manager (#65).

use crate::{Strictness, Wad};

/// The meaning of a marker-delimited directory section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SectionKind {
    /// `S_START..S_END` (classic sprites; Doom 64 uses the same markers).
    Sprites,
    /// `F_START..F_END` (classic flats).
    Flats,
    /// `P_START..P_END` (classic patches).
    Patches,
    /// `T_START..T_END` (Doom 64 world textures — walls and floors share it).
    Textures,
    /// `DS_START..DS_END` (Doom 64 digital sounds).
    Sounds,
    /// `G_START..G_END` (Doom 64 generic graphics; absent from the retail
    /// KEX IWAD and therefore optional).
    Graphics,
}

/// One marker-delimited section of a WAD directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The section's semantic kind (doubled aliases normalized).
    pub kind: SectionKind,
    /// Directory index of the opening marker lump.
    pub start_marker: usize,
    /// Directory index of the closing marker lump — or the directory
    /// LENGTH when lenient recovery closed the section at end-of-directory
    /// (no closing lump exists).
    pub end_marker: usize,
    /// The lumps between the markers: `start_marker + 1 .. end_marker`.
    /// For an outer classic section this is the engine-parity extent and
    /// includes any nested sub-pair marker lumps.
    pub lumps: std::ops::Range<usize>,
    /// Nested numbered sub-pairs (`F1_`/`F2_`/..), one level deep — the
    /// empirically observed maximum. Always empty on sub-sections.
    pub sub_sections: Vec<Section>,
}

/// The result of scanning a WAD directory for sections.
#[derive(Debug, Clone, Default)]
pub struct SectionTable {
    sections: Vec<Section>,
    warnings: Vec<SectionWarning>,
}

impl SectionTable {
    /// Top-level sections in directory order (duplicates of a kind are
    /// possible after lenient recovery of naive-merge WADs).
    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    /// Lenient-mode recoveries recorded during the scan (always empty when
    /// obtained through strict [`Wad::sections`]).
    #[must_use]
    pub fn warnings(&self) -> &[SectionWarning] {
        &self.warnings
    }

    /// All top-level sections of `kind`, in directory order.
    pub fn of_kind(&self, kind: SectionKind) -> impl Iterator<Item = &Section> {
        self.sections.iter().filter(move |s| s.kind == kind)
    }
}

/// A structurally malformed marker layout (strict mode; each variant's
/// lenient counterpart in [`SectionWarning`] states the recovery).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SectionError {
    /// A `START` marker was never closed (row 1: before end-of-directory;
    /// row 2: a numbered sub-pair open at its parent's `END`).
    #[error("{kind:?} section opened at lump {index} is never closed")]
    UnpairedStart {
        /// The unclosed section's kind.
        kind: SectionKind,
        /// Directory index of the unmatched `START` marker.
        index: usize,
    },
    /// An `END` marker had no matching open `START` (row 3).
    #[error("{kind:?} section end marker at lump {index} has no open section")]
    UnpairedEnd {
        /// The closing marker's kind.
        kind: SectionKind,
        /// Directory index of the unmatched `END` marker.
        index: usize,
    },
    /// Two complete sibling pairs of the same kind (row 4).
    #[error(
        "duplicate {kind:?} sections: first opened at lump {first_start}, second at {second_start}"
    )]
    DuplicatePair {
        /// The duplicated kind.
        kind: SectionKind,
        /// The first pair's `START` index.
        first_start: usize,
        /// The second pair's `START` index.
        second_start: usize,
    },
    /// A same-kind `START` while that kind is already open (row 5).
    #[error(
        "{kind:?} section opened at lump {inner_start} while the section opened at {outer_start} is still open"
    )]
    NestedDuplicate {
        /// The kind opened twice.
        kind: SectionKind,
        /// The already-open `START` index.
        outer_start: usize,
        /// The redundant `START` index.
        inner_start: usize,
    },
    /// An `END` closed a section that was not the most recently opened
    /// (row 6: cross-kind interleave).
    #[error(
        "{closing_kind:?} section closed at lump {index} while a {open_kind:?} section is still open"
    )]
    Interleaved {
        /// The kind left open when the LIFO order was violated.
        open_kind: SectionKind,
        /// The kind being closed out of order.
        closing_kind: SectionKind,
        /// Directory index of the out-of-order `END` marker.
        index: usize,
    },
    /// A numbered sub-pair outside its parent kind (rows 7-8).
    #[error("numbered {kind:?} sub-section at lump {index} has no open parent of its kind")]
    OrphanSubPair {
        /// The sub-pair's kind.
        kind: SectionKind,
        /// Directory index of the sub-pair's `START` marker.
        index: usize,
    },
}

/// A marker anomaly recovered during lenient scanning; each variant states
/// its recovery.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SectionWarning {
    /// An unclosed `START`; the section was closed at end-of-directory
    /// (top-level) or at its parent's `END` (numbered sub-pair).
    #[error(
        "{kind:?} section opened at lump {index} is never closed; closed at the enclosing boundary during lenient scanning"
    )]
    UnpairedStart {
        /// The unclosed section's kind.
        kind: SectionKind,
        /// Directory index of the unmatched `START` marker.
        index: usize,
    },
    /// An `END` with no open section; the marker was ignored.
    #[error(
        "{kind:?} section end marker at lump {index} has no open section; ignored during lenient scanning"
    )]
    UnpairedEnd {
        /// The closing marker's kind.
        kind: SectionKind,
        /// Directory index of the unmatched `END` marker.
        index: usize,
    },
    /// Two complete sibling pairs of the same kind; both were kept as
    /// separate sections.
    #[error(
        "duplicate {kind:?} sections (lumps {first_start} and {second_start}); both kept during lenient scanning"
    )]
    DuplicatePair {
        /// The duplicated kind.
        kind: SectionKind,
        /// The first pair's `START` index.
        first_start: usize,
        /// The second pair's `START` index.
        second_start: usize,
    },
    /// A redundant same-kind `START`; the marker was ignored (its eventual
    /// surplus `END` is then reported as [`UnpairedEnd`][Self::UnpairedEnd]).
    #[error(
        "{kind:?} section reopened at lump {inner_start} while the section opened at {outer_start} is still open; redundant marker ignored during lenient scanning"
    )]
    NestedDuplicate {
        /// The kind opened twice.
        kind: SectionKind,
        /// The already-open `START` index.
        outer_start: usize,
        /// The redundant `START` index.
        inner_start: usize,
    },
    /// A LIFO-violating `END`; the matching open section was closed where
    /// its `END` appeared, others left open (one warning per `END` marker,
    /// naming the topmost jumped kind).
    #[error(
        "{closing_kind:?} section closed at lump {index} while a {open_kind:?} section is still open; matching section closed during lenient scanning"
    )]
    Interleaved {
        /// The topmost kind jumped by the out-of-order close.
        open_kind: SectionKind,
        /// The kind being closed out of order.
        closing_kind: SectionKind,
        /// Directory index of the out-of-order `END` marker.
        index: usize,
    },
    /// A numbered sub-pair with no open parent of its kind; promoted to a
    /// top-level section.
    #[error(
        "numbered {kind:?} sub-section at lump {index} has no open parent of its kind; promoted to top level during lenient scanning"
    )]
    OrphanSubPair {
        /// The sub-pair's kind.
        kind: SectionKind,
        /// Directory index of the sub-pair's `START` marker.
        index: usize,
    },
}

/// A classified marker name.
enum Marker {
    Start {
        kind: SectionKind,
        sub: bool,
    },
    End {
        kind: SectionKind,
        sub: bool,
    },
    /// `CHECKSUM`/`ENDOFWAD`: recognized so they are documented as
    /// non-content, but they neither open nor close anything.
    Trailer,
}

/// Classifies a lump name against the marker grammar; `None` = content.
fn classify(name: &str) -> Option<Marker> {
    if name == "CHECKSUM" || name == "ENDOFWAD" {
        return Some(Marker::Trailer);
    }
    let (prefix, is_start) = if let Some(p) = name.strip_suffix("_START") {
        (p, true)
    } else {
        let p = name.strip_suffix("_END")?;
        (p, false)
    };
    let single = |b: u8| match b {
        b'S' => Some(SectionKind::Sprites),
        b'F' => Some(SectionKind::Flats),
        b'P' => Some(SectionKind::Patches),
        b'T' => Some(SectionKind::Textures),
        b'G' => Some(SectionKind::Graphics),
        _ => None,
    };
    let (kind, sub) = match prefix.as_bytes() {
        [b] => (single(*b)?, false),
        b"DS" => (SectionKind::Sounds, false),
        // Boom doubled aliases: F/P/S only (never T/G/DS).
        [a, b] if a == b && matches!(a, b'F' | b'P' | b'S') => (single(*a)?, false),
        // Numbered sub-pairs: F/P/S + one digit 1-9.
        [a, d] if d.is_ascii_digit() && *d != b'0' && matches!(a, b'F' | b'P' | b'S') => {
            (single(*a)?, true)
        }
        _ => return None,
    };
    Some(if is_start {
        Marker::Start { kind, sub }
    } else {
        Marker::End { kind, sub }
    })
}

/// One open (not yet closed) section during the scan.
struct Open {
    kind: SectionKind,
    sub: bool,
    start: usize,
    children: Vec<Section>,
}

impl Open {
    fn close(self, end_marker: usize) -> Section {
        Section {
            kind: self.kind,
            start_marker: self.start,
            end_marker,
            lumps: self.start + 1..end_marker,
            sub_sections: self.children,
        }
    }
}

/// Tracks the first completed top-level pair per kind, for row-4 duplicate
/// detection.
type FirstCompleted = std::collections::HashMap<SectionKind, usize>;

/// Handles a top-level (non-numbered) `START` marker: rows 4-5.
fn handle_top_start(
    open: &mut Vec<Open>,
    table: &mut SectionTable,
    first_completed: &FirstCompleted,
    strictness: Strictness,
    kind: SectionKind,
    i: usize,
) -> Result<(), SectionError> {
    if let Some(outer) = open.iter().find(|o| o.kind == kind && !o.sub) {
        let (outer_start, inner_start) = (outer.start, i);
        match strictness {
            Strictness::Strict => {
                return Err(SectionError::NestedDuplicate {
                    kind,
                    outer_start,
                    inner_start,
                });
            }
            Strictness::Lenient => table.warnings.push(SectionWarning::NestedDuplicate {
                kind,
                outer_start,
                inner_start,
            }),
        }
        return Ok(()); // row 5: redundant marker ignored
    }
    if let Some(&first_start) = first_completed.get(&kind) {
        match strictness {
            Strictness::Strict => {
                return Err(SectionError::DuplicatePair {
                    kind,
                    first_start,
                    second_start: i,
                });
            }
            Strictness::Lenient => table.warnings.push(SectionWarning::DuplicatePair {
                kind,
                first_start,
                second_start: i,
            }),
        }
        // row 4: fall through — the second pair still opens.
    }
    open.push(Open {
        kind,
        sub: false,
        start: i,
        children: Vec::new(),
    });
    Ok(())
}

/// Handles a numbered sub-pair `START` marker: rows 5, 7-8.
fn handle_sub_start(
    open: &mut Vec<Open>,
    table: &mut SectionTable,
    strictness: Strictness,
    kind: SectionKind,
    i: usize,
) -> Result<(), SectionError> {
    let has_parent = open.iter().any(|o| o.kind == kind && !o.sub);
    let sub_already_open = open.iter().any(|o| o.kind == kind && o.sub);
    if sub_already_open {
        let outer_start = open
            .iter()
            .find(|o| o.kind == kind && o.sub)
            .map(|o| o.start)
            .expect("just checked");
        match strictness {
            Strictness::Strict => {
                return Err(SectionError::NestedDuplicate {
                    kind,
                    outer_start,
                    inner_start: i,
                });
            }
            Strictness::Lenient => table.warnings.push(SectionWarning::NestedDuplicate {
                kind,
                outer_start,
                inner_start: i,
            }),
        }
        return Ok(());
    }
    if !has_parent {
        match strictness {
            Strictness::Strict => {
                return Err(SectionError::OrphanSubPair { kind, index: i });
            }
            Strictness::Lenient => {
                table
                    .warnings
                    .push(SectionWarning::OrphanSubPair { kind, index: i });
            }
        }
        // rows 7-8: promote — open as top-level below.
    }
    open.push(Open {
        kind,
        sub: true,
        start: i,
        children: Vec::new(),
    });
    Ok(())
}

/// Handles an `END` marker (top-level or numbered sub-pair): rows 2-3, 6.
fn handle_end(
    open: &mut Vec<Open>,
    table: &mut SectionTable,
    first_completed: &mut FirstCompleted,
    strictness: Strictness,
    kind: SectionKind,
    sub: bool,
    i: usize,
) -> Result<(), SectionError> {
    let Some(pos) = open.iter().rposition(|o| o.kind == kind && o.sub == sub) else {
        match strictness {
            Strictness::Strict => {
                return Err(SectionError::UnpairedEnd { kind, index: i });
            }
            Strictness::Lenient => {
                table
                    .warnings
                    .push(SectionWarning::UnpairedEnd { kind, index: i });
                return Ok(()); // row 3: ignored
            }
        }
    };
    if pos != open.len() - 1 {
        // Entries above `pos` are jumped. Same-kind numbered children of
        // this section close at this END (row 2, one warning each,
        // attributed to their own START); any foreign kind is row 6 (ONE
        // warning, topmost).
        let jumped_foreign = open[pos + 1..]
            .iter()
            .rev()
            .find(|o| !(o.sub && o.kind == kind))
            .map(|o| o.kind);
        if let Some(open_kind) = jumped_foreign {
            match strictness {
                Strictness::Strict => {
                    return Err(SectionError::Interleaved {
                        open_kind,
                        closing_kind: kind,
                        index: i,
                    });
                }
                Strictness::Lenient => table.warnings.push(SectionWarning::Interleaved {
                    open_kind,
                    closing_kind: kind,
                    index: i,
                }),
            }
        }
        // Close jumped same-kind subs into this section (row 2).
        let mut child_idx = pos + 1;
        while child_idx < open.len() {
            if open[child_idx].sub && open[child_idx].kind == kind {
                let child = open.remove(child_idx);
                match strictness {
                    Strictness::Strict => {
                        return Err(SectionError::UnpairedStart {
                            kind,
                            index: child.start,
                        });
                    }
                    Strictness::Lenient => {
                        table.warnings.push(SectionWarning::UnpairedStart {
                            kind,
                            index: child.start,
                        });
                        let closed = child.close(i);
                        open[pos].children.push(closed);
                    }
                }
            } else {
                child_idx += 1;
            }
        }
    }
    let section = open.remove(pos).close(i);
    if sub {
        // Attach to the enclosing open top-level of the same kind; a
        // promoted orphan has none and stays top-level.
        if let Some(parent) = open.iter_mut().rev().find(|o| o.kind == kind && !o.sub) {
            parent.children.push(section);
        } else {
            table.sections.push(section);
        }
    } else {
        first_completed.entry(kind).or_insert(section.start_marker);
        table.sections.push(section);
    }
    Ok(())
}

/// End-of-directory cleanup: rows 1-2 for whatever is still open.
fn handle_eof(
    mut open: Vec<Open>,
    table: &mut SectionTable,
    strictness: Strictness,
    eof: usize,
) -> Result<(), SectionError> {
    while let Some(o) = open.pop() {
        match strictness {
            Strictness::Strict => {
                return Err(SectionError::UnpairedStart {
                    kind: o.kind,
                    index: o.start,
                });
            }
            Strictness::Lenient => {
                table.warnings.push(SectionWarning::UnpairedStart {
                    kind: o.kind,
                    index: o.start,
                });
                let (kind, sub) = (o.kind, o.sub);
                let section = o.close(eof);
                if sub {
                    if let Some(parent) = open.iter_mut().rev().find(|p| p.kind == kind && !p.sub) {
                        parent.children.push(section);
                        continue;
                    }
                }
                table.sections.push(section);
            }
        }
    }
    Ok(())
}

/// The scan. Strict returns the first anomaly; lenient records a warning
/// per the policy table and recovers. `O(lumps)` single pass; the open set
/// is bounded by one top-level entry per [`SectionKind`] plus one numbered
/// child, and warnings by one per marker lump (ADR-0016 §1).
pub(crate) fn scan(wad: &Wad, strictness: Strictness) -> Result<SectionTable, SectionError> {
    let mut open: Vec<Open> = Vec::new();
    let mut table = SectionTable::default();
    let mut first_completed: FirstCompleted = FirstCompleted::new();

    for (i, lump) in wad.lumps().iter().enumerate() {
        let marker = match classify(lump.name()) {
            None | Some(Marker::Trailer) => continue,
            Some(m) => m,
        };
        match marker {
            Marker::Start { kind, sub: false } => {
                handle_top_start(&mut open, &mut table, &first_completed, strictness, kind, i)?;
            }
            Marker::Start { kind, sub: true } => {
                handle_sub_start(&mut open, &mut table, strictness, kind, i)?;
            }
            Marker::End { kind, sub } => {
                handle_end(
                    &mut open,
                    &mut table,
                    &mut first_completed,
                    strictness,
                    kind,
                    sub,
                    i,
                )?;
            }
            Marker::Trailer => unreachable!("filtered above"),
        }
    }

    handle_eof(open, &mut table, strictness, wad.lumps().len())?;
    table.sections.sort_by_key(|s| s.start_marker);
    Ok(table)
}
