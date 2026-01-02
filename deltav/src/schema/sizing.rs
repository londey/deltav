//! T-shirt sizing definitions for effort estimation.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// T-shirt sizing definitions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Sizing {
    /// Extra Small sizing definition.
    #[serde(rename = "XS", default = "default_xs")]
    pub xs: TShirtSize,

    /// Small sizing definition.
    #[serde(rename = "S", default = "default_s")]
    pub s: TShirtSize,

    /// Medium sizing definition.
    #[serde(rename = "M", default = "default_m")]
    pub m: TShirtSize,

    /// Large sizing definition.
    #[serde(rename = "L", default = "default_l")]
    pub l: TShirtSize,

    /// Extra Large sizing definition.
    #[serde(rename = "XL", default = "default_xl")]
    pub xl: TShirtSize,
}

impl Default for Sizing {
    fn default() -> Self {
        Sizing {
            xs: default_xs(),
            s: default_s(),
            m: default_m(),
            l: default_l(),
            xl: default_xl(),
        }
    }
}

impl Sizing {
    /// Get points for a size label (case-insensitive).
    pub fn points_for(&self, label: &str) -> Option<u32> {
        match label.to_uppercase().as_str() {
            "XS" => Some(self.xs.points),
            "S" => Some(self.s.points),
            "M" => Some(self.m.points),
            "L" => Some(self.l.points),
            "XL" => Some(self.xl.points),
            _ => None,
        }
    }

    /// Get all sizes as a map for iteration.
    pub fn as_map(&self) -> HashMap<&str, &TShirtSize> {
        let mut map = HashMap::new();
        map.insert("XS", &self.xs);
        map.insert("S", &self.s);
        map.insert("M", &self.m);
        map.insert("L", &self.l);
        map.insert("XL", &self.xl);
        map
    }

    /// Get sizes in order from smallest to largest.
    pub fn ordered(&self) -> Vec<(&str, &TShirtSize)> {
        vec![
            ("XS", &self.xs),
            ("S", &self.s),
            ("M", &self.m),
            ("L", &self.l),
            ("XL", &self.xl),
        ]
    }

    /// Format sizing table for reports.
    pub fn format_table(&self) -> String {
        let sizes = self.ordered();
        sizes
            .iter()
            .map(|(label, size)| format!("{}={}pt ({})", label, size.points, size.description))
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

/// A single T-shirt size definition.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TShirtSize {
    /// Story points value for this size.
    pub points: u32,

    /// Human-readable description of what this size means.
    pub description: String,
}

fn default_xs() -> TShirtSize {
    TShirtSize {
        points: 1,
        description: "< 2 hours, trivial change".to_string(),
    }
}

fn default_s() -> TShirtSize {
    TShirtSize {
        points: 2,
        description: "Half day, well-understood".to_string(),
    }
}

fn default_m() -> TShirtSize {
    TShirtSize {
        points: 5,
        description: "1-2 days, some complexity".to_string(),
    }
}

fn default_l() -> TShirtSize {
    TShirtSize {
        points: 8,
        description: "3-5 days, significant work".to_string(),
    }
}

fn default_xl() -> TShirtSize {
    TShirtSize {
        points: 13,
        description: "Week+, should probably be split".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_sizing() {
        let sizing = Sizing::default();
        assert_eq!(sizing.xs.points, 1);
        assert_eq!(sizing.s.points, 2);
        assert_eq!(sizing.m.points, 5);
        assert_eq!(sizing.l.points, 8);
        assert_eq!(sizing.xl.points, 13);
    }

    #[test]
    fn test_points_for() {
        let sizing = Sizing::default();
        assert_eq!(sizing.points_for("XS"), Some(1));
        assert_eq!(sizing.points_for("xs"), Some(1));
        assert_eq!(sizing.points_for("m"), Some(5));
        assert_eq!(sizing.points_for("XXL"), None);
    }

    #[test]
    fn test_format_table() {
        let sizing = Sizing::default();
        let table = sizing.format_table();
        assert!(table.contains("XS=1pt"));
        assert!(table.contains("XL=13pt"));
    }
}
