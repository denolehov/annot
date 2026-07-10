//! Annotation identity and position.
//!
//! An `Annotation`'s `id` is its identity; `anchor` is a mutable property that
//! can move (re-diff, thread re-anchoring) without changing what annotation it is.

use serde::{Deserialize, Serialize};

use crate::source::Side;
use crate::state::ContentNode;

/// One endpoint of an anchor: which side of a diff, and the 1-indexed source line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub side: Side,
    pub line: u32,
}

/// A position in a file. `start == end` for single-line annotations.
/// File/content/markdown modes use `side: Side::New` everywhere.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub path: String,
    pub start: Endpoint,
    pub end: Endpoint,
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
        self.anchor.start.line
    }

    pub fn end_line(&self) -> u32 {
        self.anchor.end.line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_side_anchor_round_trips_through_serde() {
        let anchor = Anchor {
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
        assert_eq!(round_tripped.start.side, Side::Old);
        assert_eq!(round_tripped.end.side, Side::New);
    }
}
