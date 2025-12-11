//! Unit conversion utilities
//!
//! Handles conversion between Metric (mm) and Imperial (inch) systems.
//! Supports decimal and fractional inch parsing and formatting.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Measurement system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MeasurementSystem {
    /// Metric system (mm)
    Metric,
    /// Imperial system (inches)
    Imperial,
}

impl Default for MeasurementSystem {
    fn default() -> Self {
        Self::Metric
    }
}

impl fmt::Display for MeasurementSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Metric => write!(f, "Metric"),
            Self::Imperial => write!(f, "Imperial"),
        }
    }
}

impl FromStr for MeasurementSystem {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "metric" | "mm" => Ok(Self::Metric),
            "imperial" | "inch" | "in" => Ok(Self::Imperial),
            _ => Err(format!("Unknown measurement system: {}", s)),
        }
    }
}

/// Feed rate units selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedRateUnits {
    /// Millimeters per minute
    MmPerMin,
    /// Millimeters per second
    MmPerSec,
    /// Inches per minute
    InPerMin,
    /// Inches per second
    InPerSec,
}

impl Default for FeedRateUnits {
    fn default() -> Self {
        Self::MmPerMin
    }
}

impl fmt::Display for FeedRateUnits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MmPerMin => write!(f, "mm/min"),
            Self::MmPerSec => write!(f, "mm/sec"),
            Self::InPerMin => write!(f, "in/min"),
            Self::InPerSec => write!(f, "in/sec"),
        }
    }
}

/// Format length value for display
/// 
/// * `value_mm` - Value in millimeters
/// * `system` - Target measurement system
pub fn format_length(value_mm: f32, system: MeasurementSystem) -> String {
    match system {
        MeasurementSystem::Metric => {
            // Format to 3 decimal places
            format!("{:.3}", value_mm)
        }
        MeasurementSystem::Imperial => {
            let inches = value_mm / 25.4;
            // Format to 3 decimal places
            format!("{:.3}", inches)
        }
    }
}

/// Format feed rate value for display
/// 
/// * `value_mm_per_min` - Feed rate in mm/min
/// * `units` - Target feed rate units
pub fn format_feed_rate(value_mm_per_min: f32, units: FeedRateUnits) -> String {
    let value = match units {
        FeedRateUnits::MmPerMin => value_mm_per_min,
        FeedRateUnits::MmPerSec => value_mm_per_min / 60.0,
        FeedRateUnits::InPerMin => value_mm_per_min / 25.4,
        FeedRateUnits::InPerSec => (value_mm_per_min / 25.4) / 60.0,
    };
    format!("{:.3}", value)
}

/// Parse length string to millimeters
/// 
/// * `input` - String to parse
/// * `system` - Assumed measurement system
pub fn parse_length(input: &str, system: MeasurementSystem) -> Result<f32, String> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(0.0);
    }

    match system {
        MeasurementSystem::Metric => {
            input.parse::<f32>().map_err(|e| e.to_string())
        }
        MeasurementSystem::Imperial => {
            // Check for fraction
            if input.contains('/') {
                let parts: Vec<&str> = input.split_whitespace().collect();
                let mut total_inches = 0.0;

                for part in parts {
                    if part.contains('/') {
                        let frac_parts: Vec<&str> = part.split('/').collect();
                        if frac_parts.len() == 2 {
                            let num = frac_parts[0].parse::<f32>().map_err(|_| "Invalid numerator")?;
                            let den = frac_parts[1].parse::<f32>().map_err(|_| "Invalid denominator")?;
                            if den == 0.0 {
                                return Err("Division by zero".to_string());
                            }
                            total_inches += num / den;
                        } else {
                            return Err("Invalid fraction format".to_string());
                        }
                    } else {
                        total_inches += part.parse::<f32>().map_err(|_| "Invalid number part")?;
                    }
                }
                Ok(total_inches * 25.4)
            } else {
                // Decimal inches
                let inches = input.parse::<f32>().map_err(|e| e.to_string())?;
                Ok(inches * 25.4)
            }
        }
    }
}

/// Parse feed rate string to mm/min
/// 
/// * `input` - String to parse
/// * `units` - Assumed feed rate units
pub fn parse_feed_rate(input: &str, units: FeedRateUnits) -> Result<f32, String> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(0.0);
    }

    let value = input.parse::<f32>().map_err(|e| e.to_string())?;

    let mm_per_min = match units {
        FeedRateUnits::MmPerMin => value,
        FeedRateUnits::MmPerSec => value * 60.0,
        FeedRateUnits::InPerMin => value * 25.4,
        FeedRateUnits::InPerSec => value * 25.4 * 60.0,
    };

    Ok(mm_per_min)
}

/// Get the unit label for the given system ("mm" or "in")
pub fn get_unit_label(system: MeasurementSystem) -> &'static str {
    match system {
        MeasurementSystem::Metric => "mm",
        MeasurementSystem::Imperial => "in",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_conversion() {
        assert_eq!(format_length(10.5, MeasurementSystem::Metric), "10.500");
        assert_eq!(parse_length("10.5", MeasurementSystem::Metric).unwrap(), 10.5);
    }

    #[test]
    fn test_imperial_decimal() {
        // 1 inch = 25.4 mm
        assert_eq!(format_length(25.4, MeasurementSystem::Imperial), "1.000");
        assert_eq!(parse_length("1", MeasurementSystem::Imperial).unwrap(), 25.4);
        
        // 0.5 inch = 12.7 mm
        assert_eq!(format_length(12.7, MeasurementSystem::Imperial), "0.500");
        assert_eq!(parse_length("0.5", MeasurementSystem::Imperial).unwrap(), 12.7);
    }

    #[test]
    fn test_imperial_fraction() {
        // 1 1/2 inch = 1.5 inch = 38.1 mm
        assert_eq!(parse_length("1 1/2", MeasurementSystem::Imperial).unwrap(), 38.1);
        
        // 5 1/8 inch = 5.125 inch = 130.175 mm
        assert_eq!(parse_length("5 1/8", MeasurementSystem::Imperial).unwrap(), 130.175);
        
        // Just fraction: 1/4 inch = 0.25 inch = 6.35 mm
        assert_eq!(parse_length("1/4", MeasurementSystem::Imperial).unwrap(), 6.35);
    }

    #[test]
    fn test_feed_rate_conversion() {
        // 1000 mm/min
        assert_eq!(format_feed_rate(1000.0, FeedRateUnits::MmPerMin), "1000.000");
        assert_eq!(parse_feed_rate("1000", FeedRateUnits::MmPerMin).unwrap(), 1000.0);

        // 1000 mm/min = 16.667 mm/sec
        assert_eq!(format_feed_rate(1000.0, FeedRateUnits::MmPerSec), "16.667");
        assert_eq!(parse_feed_rate("16.666666", FeedRateUnits::MmPerSec).unwrap().round(), 1000.0);

        // 1000 mm/min = 39.370 in/min
        assert_eq!(format_feed_rate(1000.0, FeedRateUnits::InPerMin), "39.370");
        assert_eq!(parse_feed_rate("39.370078", FeedRateUnits::InPerMin).unwrap().round(), 1000.0);
    }

    #[test]
    fn test_unit_labels() {
        assert_eq!(get_unit_label(MeasurementSystem::Metric), "mm");
        assert_eq!(get_unit_label(MeasurementSystem::Imperial), "in");
    }

    #[test]
    fn test_negative_values() {
        assert_eq!(parse_length("-10.5", MeasurementSystem::Metric).unwrap(), -10.5);
        assert_eq!(parse_length("-1", MeasurementSystem::Imperial).unwrap(), -25.4);
        assert_eq!(parse_length("-1/2", MeasurementSystem::Imperial).unwrap(), -12.7);
    }

    #[test]
    fn test_zero_values() {
        assert_eq!(parse_length("0", MeasurementSystem::Metric).unwrap(), 0.0);
        assert_eq!(parse_length("0", MeasurementSystem::Imperial).unwrap(), 0.0);
        assert_eq!(parse_length("", MeasurementSystem::Metric).unwrap(), 0.0);
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(parse_length("  10.5  ", MeasurementSystem::Metric).unwrap(), 10.5);
        assert_eq!(parse_length("  1  1/2  ", MeasurementSystem::Imperial).unwrap(), 38.1);
    }

    #[test]
    fn test_invalid_inputs() {
        assert!(parse_length("abc", MeasurementSystem::Metric).is_err());
        assert!(parse_length("1/0", MeasurementSystem::Imperial).is_err()); // Division by zero
        assert!(parse_length("1/2/3", MeasurementSystem::Imperial).is_err()); // Invalid fraction
    }
}
