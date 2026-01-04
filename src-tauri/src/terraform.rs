//! Terraform regions — structured controls for guiding AI content transformation.
//!
//! The type system enforces valid combinations:
//! - `Remove` and `Pin` are terminal states — no other axes apply
//! - `Dissolve` only allows direction — form/mass are irrelevant
//! - `Transform` allows all axes: form, mass, gravity (focus/blur), direction
//!
//! This makes invalid states unrepresentable at compile time.

use serde::{Deserialize, Serialize};

/// A terraform region attached to a line range.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TerraformRegion {
    pub start_line: u32,
    pub end_line: u32,
    /// The transformation intent — type-safe combinations only.
    pub intent: TerraformIntent,
}

/// Type-safe terraform intent — invalid combinations are unrepresentable.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerraformIntent {
    /// Remove this content entirely. Terminal state.
    Remove,

    /// Preserve exactly as written. Terminal state.
    Pin,

    /// Dissolve into surroundings — only direction applies.
    Dissolve {
        direction: Option<DirectionDirective>,
    },

    /// Full transformation — all axes available.
    Transform {
        form: Vec<FormType>,
        mass: Option<MassChange>,
        gravity: Option<GravityChange>,
        direction: Option<DirectionDirective>,
    },
}

impl Default for TerraformIntent {
    fn default() -> Self {
        TerraformIntent::Transform {
            form: vec![],
            mass: None,
            gravity: None,
            direction: None,
        }
    }
}

/// Target format for restructuring.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FormType {
    Table,
    List,
    Prose,
    Diagram,
    Code,
}

impl FormType {
    /// Tag value for FORMAT directive.
    /// Note: "prose" becomes "passage" to work with articles ("a passage" not "a prose").
    fn as_tag(&self) -> &'static str {
        match self {
            FormType::Table => "table",
            FormType::List => "list",
            FormType::Prose => "passage",
            FormType::Diagram => "diagram",
            FormType::Code => "code block",
        }
    }
}

/// Mass change: expand or condense (Remove is a separate intent).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MassChange {
    Expand { intensity: Intensity },
    Condense { intensity: Intensity },
}

/// Gravity change: focus or blur (Pin/Dissolve are separate intents).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum GravityChange {
    Focus { intensity: Intensity },
    Blur { intensity: Intensity },
}

/// Correctness signal: lean-in, move-away, or reframe.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DirectionDirective {
    LeanIn { intensity: Intensity },
    MoveAway { intensity: Intensity },
    Reframe,
}

/// Intensity level for graduated controls.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Intensity {
    Slightly,      // level 1 (gentlest)
    Moderately,    // level 2
    Significantly, // level 3 (strongest)
}

impl Intensity {
    /// Convert to adverb for natural language output.
    pub fn as_adverb(&self) -> &'static str {
        match self {
            Intensity::Slightly => "slightly",
            Intensity::Moderately => "moderately",
            Intensity::Significantly => "significantly",
        }
    }
}

impl TerraformRegion {
    /// Convert terraform region to natural language prose.
    ///
    /// Uses exhaustive match on `TerraformIntent` — the compiler ensures
    /// all valid combinations are handled, and invalid ones are impossible.
    pub fn to_prose(&self) -> String {
        self.intent.to_prose()
    }
}

impl TerraformIntent {
    /// Convert intent to natural language prose.
    pub fn to_prose(&self) -> String {
        match self {
            // Terminal states — single output, nothing else applies
            TerraformIntent::Remove => "Remove this entirely.".to_string(),
            TerraformIntent::Pin => "Preserve this exactly as written.".to_string(),

            // Dissolve — only direction applies
            TerraformIntent::Dissolve { direction } => {
                let mut clauses = vec![
                    "Dissolve this as a unit, integrating its essence into surroundings.".to_string()
                ];
                if let Some(dir) = direction {
                    clauses.push(format!("{}.", dir.to_prose_clause()));
                }
                clauses.join(" ")
            }

            // Full transform — all axes available
            TerraformIntent::Transform {
                form,
                mass,
                gravity,
                direction,
            } => {
                let mut clauses = Vec::new();

                // Form + Mass combined clause
                if !form.is_empty() {
                    clauses.push(form_mass_clause(form, mass.as_ref()));
                } else if let Some(m) = mass {
                    // Mass-only when no form
                    clauses.push(format!("Make this {}.", m.as_verbosity()));
                }

                // Gravity clause
                if let Some(g) = gravity {
                    clauses.push(format!("{}.", g.to_prose_clause()));
                }

                // Direction clause
                if let Some(d) = direction {
                    clauses.push(format!("{}.", d.to_prose_clause()));
                }

                clauses.join(" ")
            }
        }
    }

    /// Check if this intent is empty (no transformation requested).
    pub fn is_empty(&self) -> bool {
        matches!(
            self,
            TerraformIntent::Transform {
                form,
                mass: None,
                gravity: None,
                direction: None,
            } if form.is_empty()
        )
    }
}

/// Build combined Form + Mass clause for natural output.
fn form_mass_clause(form: &[FormType], mass: Option<&MassChange>) -> String {
    let verbosity = mass.map(|m| m.as_verbosity());

    match form.len() {
        0 => String::new(),
        1 => {
            let f = form[0].as_tag();
            match verbosity {
                Some(v) => format!("Present as a {} {}.", v, f),
                None => format!("Present as a {}.", f),
            }
        }
        _ => {
            let forms: Vec<_> = form.iter().map(|f| f.as_tag()).collect();
            let joined = join_with_and(&forms);
            match verbosity {
                Some(v) => format!("Express via {} {}.", v, joined),
                None => format!("Express via {}.", joined),
            }
        }
    }
}

/// Join items with commas and "and" for the last item.
/// ["a"] -> "a"
/// ["a", "b"] -> "a and b"
/// ["a", "b", "c"] -> "a, b, and c"
fn join_with_and(items: &[&str]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].to_string(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let (last, rest) = items.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

impl MassChange {
    /// Convert to verbosity descriptor for natural language output.
    fn as_verbosity(&self) -> &'static str {
        match self {
            MassChange::Expand { intensity } => match intensity {
                Intensity::Slightly => "fuller",
                Intensity::Moderately => "detailed",
                Intensity::Significantly => "comprehensive",
            },
            MassChange::Condense { intensity } => match intensity {
                Intensity::Slightly => "tighter",
                Intensity::Moderately => "concise",
                Intensity::Significantly => "minimal",
            },
        }
    }
}

impl GravityChange {
    /// Convert to natural language prose clause.
    fn to_prose_clause(&self) -> &'static str {
        match self {
            GravityChange::Focus { intensity } => match intensity {
                Intensity::Slightly => "Give this slightly more weight",
                Intensity::Moderately => "Emphasize the key points",
                Intensity::Significantly => "Make this the centerpiece",
            },
            GravityChange::Blur { intensity } => match intensity {
                Intensity::Slightly => "Soften the emphasis slightly",
                Intensity::Moderately => "Treat as supporting context",
                Intensity::Significantly => "Push this to the background",
            },
        }
    }
}

impl DirectionDirective {
    /// Convert to natural language prose clause.
    fn to_prose_clause(&self) -> &'static str {
        match self {
            DirectionDirective::LeanIn { intensity } => match intensity {
                Intensity::Slightly => "You're on the right track",
                Intensity::Moderately => "This direction is working — keep going",
                Intensity::Significantly => "Double down on this approach",
            },
            DirectionDirective::MoveAway { intensity } => match intensity {
                Intensity::Slightly => "Consider adjusting the angle slightly",
                Intensity::Moderately => "This needs a different direction",
                Intensity::Significantly => "This is off-target — overhaul the approach",
            },
            DirectionDirective::Reframe => "Same content, reframed from a different angle",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a region with given intent.
    fn region(intent: TerraformIntent) -> TerraformRegion {
        TerraformRegion {
            start_line: 1,
            end_line: 10,
            intent,
        }
    }

    /// Helper to create a Transform intent.
    fn transform(
        form: Vec<FormType>,
        mass: Option<MassChange>,
        gravity: Option<GravityChange>,
        direction: Option<DirectionDirective>,
    ) -> TerraformIntent {
        TerraformIntent::Transform {
            form,
            mass,
            gravity,
            direction,
        }
    }

    // ========== Terminal States ==========

    #[test]
    fn remove() {
        let r = region(TerraformIntent::Remove);
        assert_eq!(r.to_prose(), "Remove this entirely.");
    }

    #[test]
    fn pin() {
        let r = region(TerraformIntent::Pin);
        assert_eq!(r.to_prose(), "Preserve this exactly as written.");
    }

    // ========== Dissolve (only direction applies) ==========

    #[test]
    fn dissolve_only() {
        let r = region(TerraformIntent::Dissolve { direction: None });
        assert_eq!(
            r.to_prose(),
            "Dissolve this as a unit, integrating its essence into surroundings."
        );
    }

    #[test]
    fn dissolve_with_direction() {
        let r = region(TerraformIntent::Dissolve {
            direction: Some(DirectionDirective::MoveAway {
                intensity: Intensity::Moderately,
            }),
        });
        assert_eq!(
            r.to_prose(),
            "Dissolve this as a unit, integrating its essence into surroundings. \
             This needs a different direction."
        );
    }

    // ========== Transform: Form ==========

    #[test]
    fn form_single() {
        let r = region(transform(vec![FormType::Table], None, None, None));
        assert_eq!(r.to_prose(), "Present as a table.");
    }

    #[test]
    fn form_two() {
        let r = region(transform(
            vec![FormType::Table, FormType::Prose],
            None,
            None,
            None,
        ));
        assert_eq!(r.to_prose(), "Express via table and passage.");
    }

    #[test]
    fn form_multiple() {
        let r = region(transform(
            vec![FormType::Table, FormType::List, FormType::Diagram],
            None,
            None,
            None,
        ));
        assert_eq!(r.to_prose(), "Express via table, list, and diagram.");
    }

    // ========== Transform: Mass ==========

    #[test]
    fn mass_expand() {
        let r = region(transform(
            vec![],
            Some(MassChange::Expand {
                intensity: Intensity::Moderately,
            }),
            None,
            None,
        ));
        assert_eq!(r.to_prose(), "Make this detailed.");
    }

    #[test]
    fn mass_condense() {
        let r = region(transform(
            vec![],
            Some(MassChange::Condense {
                intensity: Intensity::Significantly,
            }),
            None,
            None,
        ));
        assert_eq!(r.to_prose(), "Make this minimal.");
    }

    // ========== Transform: Gravity ==========

    #[test]
    fn gravity_focus() {
        let r = region(transform(
            vec![],
            None,
            Some(GravityChange::Focus {
                intensity: Intensity::Moderately,
            }),
            None,
        ));
        assert_eq!(r.to_prose(), "Emphasize the key points.");
    }

    #[test]
    fn gravity_blur() {
        let r = region(transform(
            vec![],
            None,
            Some(GravityChange::Blur {
                intensity: Intensity::Slightly,
            }),
            None,
        ));
        assert_eq!(r.to_prose(), "Soften the emphasis slightly.");
    }

    // ========== Transform: Direction ==========

    #[test]
    fn direction_lean_in() {
        let r = region(transform(
            vec![],
            None,
            None,
            Some(DirectionDirective::LeanIn {
                intensity: Intensity::Significantly,
            }),
        ));
        assert_eq!(r.to_prose(), "Double down on this approach.");
    }

    #[test]
    fn direction_move_away() {
        let r = region(transform(
            vec![],
            None,
            None,
            Some(DirectionDirective::MoveAway {
                intensity: Intensity::Moderately,
            }),
        ));
        assert_eq!(r.to_prose(), "This needs a different direction.");
    }

    #[test]
    fn direction_reframe() {
        let r = region(transform(
            vec![],
            None,
            None,
            Some(DirectionDirective::Reframe),
        ));
        assert_eq!(r.to_prose(), "Same content, reframed from a different angle.");
    }

    // ========== Transform: Combined ==========

    #[test]
    fn combined_all_axes() {
        let r = region(transform(
            vec![FormType::Table, FormType::Prose],
            Some(MassChange::Expand {
                intensity: Intensity::Moderately,
            }),
            Some(GravityChange::Focus {
                intensity: Intensity::Moderately,
            }),
            Some(DirectionDirective::LeanIn {
                intensity: Intensity::Slightly,
            }),
        ));
        assert_eq!(
            r.to_prose(),
            "Express via detailed table and passage. Emphasize the key points. You're on the right track."
        );
    }

    #[test]
    fn form_with_mass() {
        let r = region(transform(
            vec![FormType::List],
            Some(MassChange::Expand {
                intensity: Intensity::Slightly,
            }),
            Some(GravityChange::Focus {
                intensity: Intensity::Moderately,
            }),
            Some(DirectionDirective::LeanIn {
                intensity: Intensity::Significantly,
            }),
        ));
        assert_eq!(
            r.to_prose(),
            "Present as a fuller list. Emphasize the key points. Double down on this approach."
        );
    }

    // ========== Empty / Default ==========

    #[test]
    fn empty_transform() {
        let r = region(transform(vec![], None, None, None));
        assert_eq!(r.to_prose(), "");
    }

    #[test]
    fn intent_is_empty() {
        assert!(TerraformIntent::default().is_empty());
        assert!(!TerraformIntent::Remove.is_empty());
        assert!(!TerraformIntent::Pin.is_empty());
        assert!(!TerraformIntent::Dissolve { direction: None }.is_empty());
    }

    // ========== Serialization ==========

    #[test]
    fn serialization_roundtrip_remove() {
        let r = region(TerraformIntent::Remove);
        let json = serde_json::to_string(&r).unwrap();
        let deserialized: TerraformRegion = serde_json::from_str(&json).unwrap();
        assert_eq!(r, deserialized);
    }

    #[test]
    fn serialization_roundtrip_transform() {
        let r = region(transform(
            vec![FormType::Code, FormType::Diagram],
            Some(MassChange::Condense {
                intensity: Intensity::Significantly,
            }),
            Some(GravityChange::Focus {
                intensity: Intensity::Moderately,
            }),
            Some(DirectionDirective::Reframe),
        ));
        let json = serde_json::to_string(&r).unwrap();
        let deserialized: TerraformRegion = serde_json::from_str(&json).unwrap();
        assert_eq!(r, deserialized);
    }

    #[test]
    fn serialization_roundtrip_dissolve() {
        let r = region(TerraformIntent::Dissolve {
            direction: Some(DirectionDirective::LeanIn {
                intensity: Intensity::Slightly,
            }),
        });
        let json = serde_json::to_string(&r).unwrap();
        let deserialized: TerraformRegion = serde_json::from_str(&json).unwrap();
        assert_eq!(r, deserialized);
    }

    #[test]
    fn intensity_adverbs() {
        assert_eq!(Intensity::Slightly.as_adverb(), "slightly");
        assert_eq!(Intensity::Moderately.as_adverb(), "moderately");
        assert_eq!(Intensity::Significantly.as_adverb(), "significantly");
    }
}
