// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The identifiers the ITS-REST wire carries.
//!
//! Every one of these is a bare string in a URL path or a header, and swapping
//! two of them writes a clinical document into the wrong record. Each gets its
//! own type, so a swapped argument fails to compile
//! (`.claude/rules/reliability.md`).

use std::fmt;

/// The identifier of one EHR, as it appears in a path.
///
/// openEHR ITS-REST Release-1.1.0 `ehr.html` spells it `ehr_id` and types it
/// as the string form of an `HIER_OBJECT_ID`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EhrId(String);

impl EhrId {
    /// Wraps `value` verbatim.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The identifier as the path carries it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EhrId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A `uid_based_id` naming a COMPOSITION, in one of the two forms the read and
/// update paths accept.
///
/// openEHR ITS-REST Release-1.1.0 `ehr.html`: the path segment takes either
/// the versioned object uid, which names the latest version, or a full version
/// uid, which names one version and never moves. The distinction is not
/// recoverable from the string alone by anything that has not parsed it, and
/// the two mean different things to a reader, so the client keeps them apart.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompositionId {
    /// The versioned object uid. Resolves to the latest version, so what it
    /// names changes as the record is edited.
    LatestVersionOf(VersionedObjectUid),
    /// A full version uid, `<object>::<creating system>::<version tree id>`.
    /// Names one immutable version.
    Version(VersionUid),
}

impl CompositionId {
    /// The path segment for this identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match *self {
            Self::LatestVersionOf(ref uid) => uid.as_str(),
            Self::Version(ref uid) => uid.as_str(),
        }
    }
}

impl fmt::Display for CompositionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The uid of a versioned object, without a version part.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionedObjectUid(String);

impl VersionedObjectUid {
    /// Wraps `value` verbatim.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The uid as the path carries it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VersionedObjectUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A full version uid: the object uid, the creating system id, and the version
/// tree id, joined by `::`.
///
/// openEHR RM Release-1.1.0 `common.html` section 4.3 `OBJECT_VERSION_ID`.
/// This is also what an update sends in `If-Match`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionUid(String);

impl VersionUid {
    /// Wraps `value` verbatim.
    ///
    /// The text is never normalized. A CDR issues it and the client echoes it
    /// back, so reformatting it here could only make the two disagree.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The uid as the CDR issued it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The versioned object uid this version belongs to.
    ///
    /// The part before the first `::`. Returns the whole string where there is
    /// no separator, because a CDR that issued an unseparated uid has named
    /// the object with it.
    #[must_use]
    pub fn object(&self) -> VersionedObjectUid {
        let (object, _) = self.0.split_once("::").unwrap_or((&self.0, ""));
        VersionedObjectUid::new(object)
    }
}

impl fmt::Display for VersionUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
