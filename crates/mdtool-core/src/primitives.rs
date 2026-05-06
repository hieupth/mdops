use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::hash::Hash;

/// Unique identifier for a block node in the document tree.
/// Assigned sequentially (0, 1, 2, …) during parsing.
/// Stable within a single parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct BlockId(pub u32);

/// Half-open byte range [start, end) in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ByteRange {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

impl ByteRange {
    pub fn length(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// 1-based inclusive line range [start, end] in source text.
/// Serializes as compact "start-end" string for efficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub struct LineRange {
    /// 1-based inclusive start line.
    pub start: usize,
    /// 1-based inclusive end line.
    pub end: usize,
}

impl Serialize for LineRange {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.start == self.end {
            serializer.serialize_str(&self.start.to_string())
        } else {
            serializer.serialize_str(&format!("{}-{}", self.start, self.end))
        }
    }
}

impl<'de> Deserialize<'de> for LineRange {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if let Some(dash) = s.find('-') {
            let start: usize = s[..dash].parse().map_err(serde::de::Error::custom)?;
            let end: usize = s[dash+1..].parse().map_err(serde::de::Error::custom)?;
            Ok(LineRange { start, end })
        } else {
            let n: usize = s.parse().map_err(serde::de::Error::custom)?;
            Ok(LineRange { start: n, end: n })
        }
    }
}

impl LineRange {
    pub fn line_count(&self) -> usize {
        if self.end >= self.start {
            self.end - self.start + 1
        } else {
            0
        }
    }

    pub fn contains(&self, line: usize) -> bool {
        self.start <= line && line <= self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_id_equality() {
        assert_eq!(BlockId(1), BlockId(1));
        assert_ne!(BlockId(1), BlockId(2));
    }

    #[test]
    fn block_id_ordering() {
        assert!(BlockId(0) < BlockId(1));
        assert!(BlockId(100) > BlockId(99));
    }

    #[test]
    fn block_id_serialization() {
        let id = BlockId(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");
        let deserialized: BlockId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn byte_range_length() {
        let range = ByteRange { start: 10, end: 25 };
        assert_eq!(range.length(), 15);
    }

    #[test]
    fn byte_range_zero_length() {
        let range = ByteRange { start: 10, end: 10 };
        assert_eq!(range.length(), 0);
    }

    #[test]
    fn line_range_line_count() {
        let range = LineRange { start: 1, end: 5 };
        assert_eq!(range.line_count(), 5);
    }

    #[test]
    fn line_range_single_line() {
        let range = LineRange { start: 3, end: 3 };
        assert_eq!(range.line_count(), 1);
    }

    #[test]
    fn line_range_contains() {
        let range = LineRange { start: 2, end: 5 };
        assert!(!range.contains(1));
        assert!(range.contains(2));
        assert!(range.contains(3));
        assert!(range.contains(5));
        assert!(!range.contains(6));
    }

    #[test]
    fn line_range_empty() {
        let range = LineRange { start: 5, end: 3 };
        assert_eq!(range.line_count(), 0);
    }
}
