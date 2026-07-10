//! Annotation identity and position.
//!
//! An `Annotation`'s `id` is its identity; `anchor` is a mutable property that
//! can move (re-diff, thread re-anchoring) without changing what annotation it is.

use serde::{Deserialize, Serialize};

use crate::source::Side;
use crate::state::ContentNode;

/// One endpoint of a diff anchor: which side of the diff, and the 1-indexed
/// source line on that side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub side: Side,
    pub line: u32,
}

/// A position in a file. `start == end` for single-line annotations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Anchor {
    /// File/content/markdown modes: plain line coordinates. Sides only exist
    /// where a diff does.
    Source { path: String, start: u32, end: u32 },
    /// Diff mode: each endpoint carries its side; mixed sides span a deletion
    /// and its added replacement.
    Diff {
        path: String,
        start: Endpoint,
        end: Endpoint,
    },
}

impl Anchor {
    pub fn start_line(&self) -> u32 {
        match self {
            Anchor::Source { start, .. } => *start,
            Anchor::Diff { start, .. } => start.line,
        }
    }

    pub fn end_line(&self) -> u32 {
        match self {
            Anchor::Source { end, .. } => *end,
            Anchor::Diff { end, .. } => end.line,
        }
    }
}

/// An annotation: a stable id plus the (mutable) anchor and content it carries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub anchor: Anchor,
    pub content: Vec<ContentNode>,
}

impl Annotation {
    pub fn start_line(&self) -> u32 {
        self.anchor.start_line()
    }

    pub fn end_line(&self) -> u32 {
        self.anchor.end_line()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_side_anchor_round_trips_through_serde() {
        let anchor = Anchor::Diff {
            path: "test.rs".to_string(),
            start: Endpoint {
                side: Side::Old,
                line: 10,
            },
            end: Endpoint {
                side: Side::New,
                line: 12,
            },
        };

        let json = serde_json::to_string(&anchor).unwrap();
        let round_tripped: Anchor = serde_json::from_str(&json).unwrap();

        assert_eq!(anchor, round_tripped);
        let Anchor::Diff { start, end, .. } = round_tripped else {
            panic!("variant changed in round trip");
        };
        assert_eq!(start.side, Side::Old);
        assert_eq!(end.side, Side::New);
    }

    #[test]
    fn source_anchor_wire_shape_is_tagged_and_sideless() {
        let anchor = Anchor::Source {
            path: "notes.md".to_string(),
            start: 3,
            end: 7,
        };

        let json = serde_json::to_value(&anchor).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "source", "path": "notes.md", "start": 3, "end": 7 })
        );

        let round_tripped: Anchor = serde_json::from_value(json).unwrap();
        assert_eq!(anchor, round_tripped);
    }
}
