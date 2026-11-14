//! Unit conversion utilities
//!
//! Handles conversion between Metric (mm) and Imperial (inch) systems.
//! Supports decimal and fractional inch parsing and formatting.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Measurement system for dimensional values.
///
/// Determines how dimensions are displayed and parsed. Internal storage
/// always uses metric (mm); this controls the UI presentation layer.
///
/// # Example
/// ```
/// use gcodekit5_core::units::{MeasurementSystem, format_length, get_unit_label};
///
/// let value_mm = 25.4;
/// assert_eq!(format_length(value_mm, MeasurementSystem::Metric), "25.400");
/// assert_eq!(format_length(value_mm, MeasurementSystem::Imperial), "1.000");
/// assert_eq!(get_unit_label(MeasurementSystem::Metric), "mm");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MeasurementSystem {
    /// Metric system (mm)
    #[default]
    Metric,
    /// Imperial system (inches)
    Imperial,
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

/// Feed rate units selection.
///
/// Controls how feed rates are displayed and parsed. Internal storage
/// always uses mm/min; this controls the UI presentation layer.
///
/// # Example
/// ```
/// use gcodekit5_core::units::{FeedRateUnits, format_feed_rate};
///
/// let rate_mm_min = 1000.0;
/// assert_eq!(format_feed_rate(rate_mm_min, FeedRateUnits::MmPerMin), "1000.000");
/// assert_eq!(format_feed_rate(rate_mm_min, FeedRateUnits::MmPerSec), "16.667");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeedRateUnits {
    /// Millimeters per minute
    #[default]
    MmPerMin,
    /// Millimeters per second
    MmPerSec,
    /// Inches per minute
    InPerMin,
    /// Inches per second
    InPerSec,
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

/// Parse length string to millimeters.
///
/// Accepts decimal values and imperial fractions (e.g., `"1 1/2"` for 1.5 inches).
/// Empty strings return `Ok(0.0)`.
///
/// # Arguments
/// * `input` - String to parse (whitespace is trimmed)
/// * `system` - Assumed measurement system for the input
///
/// # Errors
/// Returns `Err` if:
/// - The input is not a valid number
/// - A fraction has an invalid numerator or denominator
/// - A fraction has a zero denominator (division by zero)
/// - A fraction has more than one `/` separator
pub fn parse_length(input: &str, system: MeasurementSystem) -> Result<f32, String> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(0.0);
    }

    match system {
        MeasurementSystem::Metric => input.parse::<f32>().map_err(|e| e.to_string()),
        MeasurementSystem::Imperial => {
            // Check for fraction
            if input.contains('/') {
                // A leading sign applies to the whole mixed number: "-1 1/2" is -1.5 in,
                // not -1 + 0.5.
                let (sign, unsigned) = match input.strip_prefix('-') {
                    Some(rest) => (-1.0, rest.trim_start()),
                    None => (1.0, input.strip_prefix('+').unwrap_or(input).trim_start()),
                };
                let parts: Vec<&str> = unsigned.split_whitespace().collect();
                let mut total_inches = 0.0;

                for part in parts {
                    if part.contains('/') {
                        let frac_parts: Vec<&str> = part.split('/').collect();
                        if frac_parts.len() == 2 {
                            let num = frac_parts[0]
                                .parse::<f32>()
                                .map_err(|_| "Invalid numerator")?;
                            let den = frac_parts[1]
                                .parse::<f32>()
                                .map_err(|_| "Invalid denominator")?;
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
                Ok(sign * total_inches * 25.4)
            } else {
                // Decimal inches
                let inches = input.parse::<f32>().map_err(|e| e.to_string())?;
                Ok(inches * 25.4)
            }
        }
    }
}

/// Parse feed rate string to mm/min.
///
/// Converts a numeric string from the given units to mm/min.
/// Empty strings return `Ok(0.0)`.
///
/// # Arguments
/// * `input` - String to parse (whitespace is trimmed)
/// * `units` - Assumed feed rate units of the input
///
/// # Errors
/// Returns `Err` if the input is not a valid floating-point number.
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
