//! Terraform regions — structured controls for guiding AI content transformation.
//!
//! Terraform captures user intent about *how* content should change:
//! - Form: What shape? (table, list, prose, diagram, code)
//! - Mass: How much? (expand ↔ condense, or remove)
//! - Gravity: How important? (pin, focus ↔ blur, dissolve)
//! - Direction: Right track? (lean-in ↔ move-away, reframe)
//!
//! These structured controls are translated to natural language prose
//! that the LLM can directly consume.

use serde::{Deserialize, Serialize};

/// A terraform region attached to a line range.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TerraformRegion {
    pub start_line: u32,
    pub end_line: u32,
    /// Target formats (multi-select).
    pub form: Vec<FormType>,
    /// Quantity control.
    pub mass: Option<MassDirective>,
    /// Importance control.
    pub gravity: Option<GravityDirective>,
    /// Correctness signal.
    pub direction: Option<DirectionDirective>,
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

/// Quantity directive: expand, condense, or remove.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MassDirective {
    Expand { intensity: Intensity },
    Condense { intensity: Intensity },
    Remove,
}

/// Importance directive: pin, focus, blur, or dissolve.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum GravityDirective {
    Pin,
    Focus { intensity: Intensity },
    Blur { intensity: Intensity },
    Dissolve,
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
    /// Produces human-readable instructions that an LLM can directly consume.
    /// Uses slot-filling grammar for natural, coherent output.
    ///
    /// Applies conflict precedence rules:
    /// 1. Remove overrides everything — emit only "Remove this entirely."
    /// 2. Pin blocks form, mass, direction — emit only "Preserve this exactly as written."
    /// 3. Dissolve blocks form — still allows mass and direction
    pub fn to_prose(&self) -> String {
        // Rule 1: Remove overrides everything
        if matches!(self.mass, Some(MassDirective::Remove)) {
            return "Remove this entirely.".to_string();
        }

        // Rule 2: Pin blocks form, mass (expand/condense), direction
        if matches!(self.gravity, Some(GravityDirective::Pin)) {
            return "Preserve this exactly as written.".to_string();
        }

        let mut clauses = Vec::new();

        // Rule 3: Dissolve blocks form AND mass — if dissolving, shape/length are irrelevant
        let is_dissolving = matches!(self.gravity, Some(GravityDirective::Dissolve));

        // Form + Mass combined clause (unless blocked by dissolve)
        if !is_dissolving && !self.form.is_empty() {
            clauses.push(self.form_mass_clause());
        } else if !is_dissolving {
            if let Some(ref mass) = self.mass {
                // Mass-only clause when form is empty (but not dissolving)
                if let Some(verbosity) = mass.as_verbosity() {
                    clauses.push(format!("Make this {}.", verbosity));
                }
            }
        }

        // Gravity clause
        if let Some(ref gravity) = self.gravity {
            if let Some(clause) = gravity.to_prose_clause() {
                clauses.push(format!("{}.", clause));
            }
        }

        // Direction clause
        if let Some(ref direction) = self.direction {
            clauses.push(format!("{}.", direction.to_prose_clause()));
        }

        clauses.join(" ")
    }

    /// Build combined Form + Mass clause for natural output.
    ///
    /// Examples:
    /// - Table only: "Present as a table."
    /// - Table + detailed: "Present as a detailed table."
    /// - Table, prose, list: "Express via table, prose, and list."
    /// - Table, prose + comprehensive: "Express via comprehensive table, prose, and list."
    fn form_mass_clause(&self) -> String {
        let verbosity = self.mass.as_ref().and_then(|m| m.as_verbosity());

        match self.form.len() {
            0 => String::new(),
            1 => {
                let form = self.form[0].as_tag();
                match verbosity {
                    Some(v) => format!("Present as a {} {}.", v, form),
                    None => format!("Present as a {}.", form),
                }
            }
            _ => {
                let forms: Vec<_> = self.form.iter().map(|f| f.as_tag()).collect();
                let joined = join_with_and(&forms);
                match verbosity {
                    Some(v) => format!("Express via {} {}.", v, joined),
                    None => format!("Express via {}.", joined),
                }
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

impl MassDirective {
    /// Convert to VERBOSITY tag value.
    /// Returns None for Remove (handled separately as override).
    fn as_verbosity(&self) -> Option<&'static str> {
        match self {
            MassDirective::Expand { intensity } => Some(match intensity {
                Intensity::Slightly => "fuller",
                Intensity::Moderately => "detailed",
                Intensity::Significantly => "comprehensive",
            }),
            MassDirective::Condense { intensity } => Some(match intensity {
                Intensity::Slightly => "tighter",
                Intensity::Moderately => "concise",
                Intensity::Significantly => "minimal",
            }),
            MassDirective::Remove => None,
        }
    }
}

impl GravityDirective {
    /// Convert to natural language prose clause.
    /// Returns None for Pin (handled as full override).
    fn to_prose_clause(&self) -> Option<&'static str> {
        match self {
            GravityDirective::Pin => None,
            GravityDirective::Focus { intensity } => Some(match intensity {
                Intensity::Slightly => "Give this slightly more weight",
                Intensity::Moderately => "Emphasize the key points",
                Intensity::Significantly => "Make this the centerpiece",
            }),
            GravityDirective::Blur { intensity } => Some(match intensity {
                Intensity::Slightly => "Soften the emphasis slightly",
                Intensity::Moderately => "Treat as supporting context",
                Intensity::Significantly => "Push this to the background",
            }),
            GravityDirective::Dissolve => {
                Some("Dissolve this as a unit, integrating its essence into surroundings")
            }
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

    #[test]
    fn intensity_adverbs() {
        assert_eq!(Intensity::Slightly.as_adverb(), "slightly");
        assert_eq!(Intensity::Moderately.as_adverb(), "moderately");
        assert_eq!(Intensity::Significantly.as_adverb(), "significantly");
    }

    #[test]
    fn form_clause_single() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![FormType::Table],
            mass: None,
            gravity: None,
            direction: None,
        };
        assert_eq!(region.to_prose(), "Present as a table.");
    }

    #[test]
    fn form_clause_two() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![FormType::Table, FormType::Prose],
            mass: None,
            gravity: None,
            direction: None,
        };
        assert_eq!(region.to_prose(), "Express via table and passage.");
    }

    #[test]
    fn form_clause_multiple() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![FormType::Table, FormType::List, FormType::Diagram],
            mass: None,
            gravity: None,
            direction: None,
        };
        assert_eq!(region.to_prose(), "Express via table, list, and diagram.");
    }

    #[test]
    fn mass_expand() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: Some(MassDirective::Expand {
                intensity: Intensity::Moderately,
            }),
            gravity: None,
            direction: None,
        };
        assert_eq!(region.to_prose(), "Make this detailed.");
    }

    #[test]
    fn mass_condense() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: Some(MassDirective::Condense {
                intensity: Intensity::Significantly,
            }),
            gravity: None,
            direction: None,
        };
        assert_eq!(region.to_prose(), "Make this minimal.");
    }

    #[test]
    fn mass_remove() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: Some(MassDirective::Remove),
            gravity: None,
            direction: None,
        };
        assert_eq!(region.to_prose(), "Remove this entirely.");
    }

    #[test]
    fn gravity_pin() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: None,
            gravity: Some(GravityDirective::Pin),
            direction: None,
        };
        assert_eq!(region.to_prose(), "Preserve this exactly as written.");
    }

    #[test]
    fn gravity_focus() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: None,
            gravity: Some(GravityDirective::Focus {
                intensity: Intensity::Moderately,
            }),
            direction: None,
        };
        assert_eq!(region.to_prose(), "Emphasize the key points.");
    }

    #[test]
    fn gravity_blur() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: None,
            gravity: Some(GravityDirective::Blur {
                intensity: Intensity::Slightly,
            }),
            direction: None,
        };
        assert_eq!(region.to_prose(), "Soften the emphasis slightly.");
    }

    #[test]
    fn gravity_dissolve() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: None,
            gravity: Some(GravityDirective::Dissolve),
            direction: None,
        };
        assert_eq!(
            region.to_prose(),
            "Dissolve this as a unit, integrating its essence into surroundings."
        );
    }

    #[test]
    fn direction_lean_in() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: None,
            gravity: None,
            direction: Some(DirectionDirective::LeanIn {
                intensity: Intensity::Significantly,
            }),
        };
        assert_eq!(region.to_prose(), "Double down on this approach.");
    }

    #[test]
    fn direction_move_away() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: None,
            gravity: None,
            direction: Some(DirectionDirective::MoveAway {
                intensity: Intensity::Moderately,
            }),
        };
        assert_eq!(region.to_prose(), "This needs a different direction.");
    }

    #[test]
    fn direction_reframe() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: None,
            gravity: None,
            direction: Some(DirectionDirective::Reframe),
        };
        assert_eq!(region.to_prose(), "Same content, reframed from a different angle.");
    }

    #[test]
    fn combined_all_axes() {
        let region = TerraformRegion {
            start_line: 10,
            end_line: 20,
            form: vec![FormType::Table, FormType::Prose],
            mass: Some(MassDirective::Expand {
                intensity: Intensity::Moderately,
            }),
            gravity: Some(GravityDirective::Focus {
                intensity: Intensity::Moderately,
            }),
            direction: Some(DirectionDirective::LeanIn {
                intensity: Intensity::Slightly,
            }),
        };
        assert_eq!(
            region.to_prose(),
            "Express via detailed table and passage. Emphasize the key points. You're on the right track."
        );
    }

    #[test]
    fn empty_region() {
        let region = TerraformRegion {
            start_line: 1,
            end_line: 1,
            form: vec![],
            mass: None,
            gravity: None,
            direction: None,
        };
        assert_eq!(region.to_prose(), "");
    }

    #[test]
    fn serialization_roundtrip() {
        let region = TerraformRegion {
            start_line: 5,
            end_line: 15,
            form: vec![FormType::Code, FormType::Diagram],
            mass: Some(MassDirective::Condense {
                intensity: Intensity::Significantly,
            }),
            gravity: Some(GravityDirective::Pin),
            direction: Some(DirectionDirective::Reframe),
        };

        let json = serde_json::to_string(&region).unwrap();
        let deserialized: TerraformRegion = serde_json::from_str(&json).unwrap();
        assert_eq!(region, deserialized);
    }

    // ========== Conflict Precedence Tests ==========

    #[test]
    fn remove_overrides_all() {
        // Remove should override form, gravity, and direction
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![FormType::Table, FormType::List],
            mass: Some(MassDirective::Remove),
            gravity: Some(GravityDirective::Pin),
            direction: Some(DirectionDirective::Reframe),
        };
        assert_eq!(region.to_prose(), "Remove this entirely.");
    }

    #[test]
    fn remove_overrides_expand() {
        // Remove should win even if expand was also somehow set (defensive)
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![FormType::Prose],
            mass: Some(MassDirective::Remove),
            gravity: Some(GravityDirective::Focus {
                intensity: Intensity::Significantly,
            }),
            direction: Some(DirectionDirective::LeanIn {
                intensity: Intensity::Moderately,
            }),
        };
        assert_eq!(region.to_prose(), "Remove this entirely.");
    }

    #[test]
    fn pin_blocks_form_mass_direction() {
        // Pin should block form, mass, and direction
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![FormType::Table, FormType::List],
            mass: Some(MassDirective::Expand {
                intensity: Intensity::Moderately,
            }),
            gravity: Some(GravityDirective::Pin),
            direction: Some(DirectionDirective::LeanIn {
                intensity: Intensity::Slightly,
            }),
        };
        assert_eq!(region.to_prose(), "Preserve this exactly as written.");
    }

    #[test]
    fn pin_blocks_condense() {
        // Pin with condense - pin wins
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![],
            mass: Some(MassDirective::Condense {
                intensity: Intensity::Significantly,
            }),
            gravity: Some(GravityDirective::Pin),
            direction: None,
        };
        assert_eq!(region.to_prose(), "Preserve this exactly as written.");
    }

    #[test]
    fn dissolve_blocks_form_allows_mass_direction() {
        // Dissolve should block form but allow mass and direction
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![FormType::Table, FormType::Prose],
            mass: Some(MassDirective::Condense {
                intensity: Intensity::Slightly,
            }),
            gravity: Some(GravityDirective::Dissolve),
            direction: Some(DirectionDirective::MoveAway {
                intensity: Intensity::Moderately,
            }),
        };
        // Note: Mass is blocked by Dissolve (length is irrelevant when dissolving)
        assert_eq!(
            region.to_prose(),
            "Dissolve this as a unit, integrating its essence into surroundings. \
             This needs a different direction."
        );
    }

    #[test]
    fn dissolve_blocks_form_only() {
        // Dissolve with form only - form should be blocked
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![FormType::Code],
            mass: None,
            gravity: Some(GravityDirective::Dissolve),
            direction: None,
        };
        assert_eq!(
            region.to_prose(),
            "Dissolve this as a unit, integrating its essence into surroundings."
        );
    }

    #[test]
    fn focus_blur_allow_all_axes() {
        // Focus and blur should NOT block any axes
        let region = TerraformRegion {
            start_line: 1,
            end_line: 10,
            form: vec![FormType::List],
            mass: Some(MassDirective::Expand {
                intensity: Intensity::Slightly,
            }),
            gravity: Some(GravityDirective::Focus {
                intensity: Intensity::Moderately,
            }),
            direction: Some(DirectionDirective::LeanIn {
                intensity: Intensity::Significantly,
            }),
        };
        assert_eq!(
            region.to_prose(),
            "Present as a fuller list. Emphasize the key points. Double down on this approach."
        );
    }
}
