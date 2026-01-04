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
    /// Human-readable name for output.
    fn as_str(&self) -> &'static str {
        match self {
            FormType::Table => "table",
            FormType::List => "list",
            FormType::Prose => "prose",
            FormType::Diagram => "diagram",
            FormType::Code => "code",
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

        // Rule 3: Dissolve blocks form (but allows mass/direction)
        let emit_form = !matches!(self.gravity, Some(GravityDirective::Dissolve));

        // Form clause (unless blocked by dissolve)
        if emit_form && !self.form.is_empty() {
            clauses.push(self.form_clause());
        }

        // Mass clause
        if let Some(ref mass) = self.mass {
            clauses.push(mass.to_clause());
        }

        // Gravity clause
        if let Some(ref gravity) = self.gravity {
            clauses.push(gravity.to_clause());
        }

        // Direction clause
        if let Some(ref direction) = self.direction {
            clauses.push(direction.to_clause());
        }

        clauses.join(" ")
    }

    /// Build form clause based on number of selected formats.
    fn form_clause(&self) -> String {
        match self.form.len() {
            0 => String::new(),
            1 => format!("Restructure this into a {}.", self.form[0].as_str()),
            2 => format!(
                "Restructure this into a {}. Also provide a {} version.",
                self.form[0].as_str(),
                self.form[1].as_str()
            ),
            _ => {
                let forms: Vec<_> = self.form.iter().map(|f| f.as_str()).collect();
                format!("Restructure this into multiple formats: {}.", forms.join(", "))
            }
        }
    }
}

impl MassDirective {
    /// Convert to natural language clause.
    fn to_clause(&self) -> String {
        match self {
            MassDirective::Expand { intensity } => {
                format!("Expand {} with more depth and examples.", intensity.as_adverb())
            }
            MassDirective::Condense { intensity } => {
                format!("Condense {} to essentials.", intensity.as_adverb())
            }
            MassDirective::Remove => "Remove this entirely.".to_string(),
        }
    }
}

impl GravityDirective {
    /// Convert to natural language clause.
    fn to_clause(&self) -> String {
        match self {
            GravityDirective::Pin => "Preserve this exactly as written.".to_string(),
            GravityDirective::Focus { intensity } => {
                format!("Make this {} more central/prominent.", intensity.as_adverb())
            }
            GravityDirective::Blur { intensity } => {
                format!(
                    "Reduce prominence {}; treat as supporting context.",
                    intensity.as_adverb()
                )
            }
            GravityDirective::Dissolve => {
                "Remove as a unit; integrate essence into surroundings.".to_string()
            }
        }
    }
}

impl DirectionDirective {
    /// Convert to natural language clause.
    fn to_clause(&self) -> String {
        match self {
            DirectionDirective::LeanIn { intensity } => {
                format!(
                    "You're {} on the right track. Amplify this thinking.",
                    intensity.as_adverb()
                )
            }
            DirectionDirective::MoveAway { intensity } => {
                format!(
                    "This is {} off-target. Pivot the perspective.",
                    intensity.as_adverb()
                )
            }
            DirectionDirective::Reframe => "Same content, different framing/angle.".to_string(),
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
        assert_eq!(region.to_prose(), "Restructure this into a table.");
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
        assert_eq!(
            region.to_prose(),
            "Restructure this into a table. Also provide a prose version."
        );
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
        assert_eq!(
            region.to_prose(),
            "Restructure this into multiple formats: table, list, diagram."
        );
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
        assert_eq!(region.to_prose(), "Expand moderately with more depth and examples.");
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
        assert_eq!(region.to_prose(), "Condense significantly to essentials.");
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
        assert_eq!(region.to_prose(), "Make this moderately more central/prominent.");
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
        assert_eq!(
            region.to_prose(),
            "Reduce prominence slightly; treat as supporting context."
        );
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
            "Remove as a unit; integrate essence into surroundings."
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
        assert_eq!(
            region.to_prose(),
            "You're significantly on the right track. Amplify this thinking."
        );
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
        assert_eq!(region.to_prose(), "This is moderately off-target. Pivot the perspective.");
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
        assert_eq!(region.to_prose(), "Same content, different framing/angle.");
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
            "Restructure this into a table. Also provide a prose version. \
             Expand moderately with more depth and examples. \
             Make this moderately more central/prominent. \
             You're slightly on the right track. Amplify this thinking."
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
        assert_eq!(
            region.to_prose(),
            "Condense slightly to essentials. \
             Remove as a unit; integrate essence into surroundings. \
             This is moderately off-target. Pivot the perspective."
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
            "Remove as a unit; integrate essence into surroundings."
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
            "Restructure this into a list. \
             Expand slightly with more depth and examples. \
             Make this moderately more central/prominent. \
             You're significantly on the right track. Amplify this thinking."
        );
    }
}
