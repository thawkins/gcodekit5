//! # CAM Tool Presets
//!
//! Saved parameter files for the bitmap engraving and probe tools.
//!
//! Format version 2 stores every length in millimetres as a JSON number together with
//! `"units": "mm"`, so a file loads the same whatever measurement system the UI shows.
//!
//! Version 1 files (no `format_version`) stored the text of each entry box. Lengths in
//! the bitmap tool were displayed in the active measurement system, which the file did not
//! record, so they are read with the system given by the caller (the one active at load
//! time, which is what the old loader did) and a warning is returned. Probe-tool lengths
//! were always shown in millimetres.
//!
//! Version 1 bitmap files also used the old power polarity, in which `invert = false`
//! burned the *light* areas. The flag is flipped on load so such files produce the same
//! engraving as before; see [`crate::laser_engraver::ImageTransformations::invert`].
//!
//! Depends on `serde_json` and `gcodekit5-core` (unit parsing).

use anyhow::{Result, anyhow};
use gcodekit5_core::units::{self, MeasurementSystem};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

use crate::laser_engraver::EngravingParameters;
use crate::probe_routines::defaults as probe_defaults;

/// Current preset file format version.
pub const PRESET_FORMAT_VERSION: u32 = 2;

/// Unit tag written to version 2 preset files.
pub const PRESET_UNITS: &str = "mm";

/// A loaded preset together with the problems found while reading it.
#[derive(Debug, Clone)]
pub struct PresetLoad<T> {
    /// The preset values (fields that could not be read keep their defaults).
    pub preset: T,
    /// Human-readable warnings to show to the user (empty if everything was read).
    pub warnings: Vec<String>,
}

/// Saved settings of the bitmap engraving tool. Lengths are in mm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BitmapEngravingPreset {
    /// File format version ([`PRESET_FORMAT_VERSION`]).
    pub format_version: u32,
    /// Unit of all lengths ([`PRESET_UNITS`]).
    pub units: String,
    /// Path of the source image.
    pub image_path: String,
    /// Output width (mm).
    pub width_mm: f32,
    /// Engraving feed rate (mm/min).
    pub feed_rate: f32,
    /// Travel feed rate (mm/min).
    pub travel_rate: f32,
    /// Minimum power (%).
    pub min_power: f32,
    /// Maximum power (%).
    pub max_power: f32,
    /// Resolution (pixels per mm).
    pub pixels_per_mm: f32,
    /// Line spacing multiplier.
    pub line_spacing: f32,
    /// S value at 100 % power.
    pub power_scale: f32,
    /// Image origin X (mm).
    pub offset_x: f32,
    /// Image origin Y (mm).
    pub offset_y: f32,
    /// Scan direction id ("0" horizontal, "1" vertical).
    pub scan_direction: String,
    /// Bidirectional scanning.
    pub bidirectional: bool,
    /// Engrave light areas instead of dark ones (current polarity).
    pub invert: bool,
    /// Mirror horizontally.
    pub mirror_x: bool,
    /// Mirror vertically.
    pub mirror_y: bool,
    /// Rotation id ("0", "90", "180", "270").
    pub rotation: String,
    /// Halftone method id ("none", "threshold", "bayer", "floyd", "atkinson").
    pub halftone: String,
    /// Halftone cell size (pixels).
    pub halftone_dot_size: u32,
    /// Threshold for the threshold halftone (0-255).
    pub halftone_threshold: u8,
    /// Home before the job.
    pub home_before_job: bool,
}

impl Default for BitmapEngravingPreset {
    fn default() -> Self {
        let p = EngravingParameters::default();
        Self {
            format_version: PRESET_FORMAT_VERSION,
            units: PRESET_UNITS.to_string(),
            image_path: String::new(),
            width_mm: p.width_mm,
            feed_rate: p.feed_rate,
            travel_rate: p.travel_rate,
            min_power: p.min_power,
            max_power: p.max_power,
            pixels_per_mm: p.pixels_per_mm,
            line_spacing: p.line_spacing,
            power_scale: p.power_scale,
            offset_x: p.offset_x,
            offset_y: p.offset_y,
            scan_direction: "0".to_string(),
            bidirectional: p.bidirectional,
            invert: p.transformations.invert,
            mirror_x: false,
            mirror_y: false,
            rotation: "0".to_string(),
            halftone: "none".to_string(),
            halftone_dot_size: p.transformations.halftone_dot_size as u32,
            halftone_threshold: p.transformations.halftone_threshold,
            home_before_job: p.home_before_job,
        }
    }
}

impl BitmapEngravingPreset {
    /// Serialize to pretty JSON in the current format (lengths in mm).
    ///
    /// # Returns
    /// The JSON text, or an error if serialization fails.
    pub fn to_json(&self) -> Result<String> {
        let mut out = self.clone();
        out.format_version = PRESET_FORMAT_VERSION;
        out.units = PRESET_UNITS.to_string();
        Ok(serde_json::to_string_pretty(&out)?)
    }

    /// Parse a preset file of any supported version.
    ///
    /// # Arguments
    /// * `json` - File contents.
    /// * `legacy_system` - Measurement system used to read lengths of version 1 files, which
    ///   stored display text without a unit.
    ///
    /// # Returns
    /// The preset and any warnings, or an error if the file is not valid JSON or uses an
    /// unsupported unit/version.
    pub fn from_json(json: &str, legacy_system: MeasurementSystem) -> Result<PresetLoad<Self>> {
        let value: Value =
            serde_json::from_str(json).map_err(|e| anyhow!("Invalid preset file: {}", e))?;
        if value.get("format_version").is_some() {
            check_version_and_units(&value)?;
            let preset: Self =
                serde_json::from_value(value).map_err(|e| anyhow!("Invalid preset file: {}", e))?;
            return Ok(PresetLoad {
                preset,
                warnings: Vec::new(),
            });
        }

        let mut r = LegacyReader::new(&value);
        let mut p = Self::default();
        r.string("image_path", &mut p.image_path);
        r.length("width_mm", legacy_system, &mut p.width_mm);
        r.number("feed_rate", &mut p.feed_rate);
        r.number("travel_rate", &mut p.travel_rate);
        r.number("min_power", &mut p.min_power);
        r.number("max_power", &mut p.max_power);
        r.number("pixels_per_mm", &mut p.pixels_per_mm);
        r.number("line_spacing", &mut p.line_spacing);
        r.number("power_scale", &mut p.power_scale);
        r.length("offset_x", legacy_system, &mut p.offset_x);
        r.length("offset_y", legacy_system, &mut p.offset_y);
        r.string("scan_direction", &mut p.scan_direction);
        r.boolean("bidirectional", &mut p.bidirectional);
        r.boolean("mirror_x", &mut p.mirror_x);
        r.boolean("mirror_y", &mut p.mirror_y);
        r.string("rotation", &mut p.rotation);
        r.string("halftone", &mut p.halftone);
        r.number("halftone_dot_size", &mut p.halftone_dot_size);
        r.number("halftone_threshold", &mut p.halftone_threshold);
        let mut legacy_invert = false;
        r.boolean("invert", &mut legacy_invert);
        // Old polarity burned light areas unless "invert" was set; keep the same output.
        p.invert = !legacy_invert;

        let mut warnings = r.warnings;
        warnings.push(format!(
            "Old preset format: lengths were read as {}. 'Invert' was converted to the new \
             meaning (engrave light areas) so the engraving matches the old result; check it.",
            units::get_unit_label(legacy_system)
        ));
        Ok(PresetLoad {
            preset: p,
            warnings,
        })
    }
}

/// Saved settings of the probe tool. Lengths are in mm, feeds in mm/min.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProbePreset {
    /// File format version ([`PRESET_FORMAT_VERSION`]).
    pub format_version: u32,
    /// Unit of all lengths ([`PRESET_UNITS`]).
    pub units: String,
    /// Routine id ("z_touch", "edge_find", ...).
    pub routine: Option<String>,
    /// Safe height (mm).
    pub safe_height: f64,
    /// Fast probe feed (mm/min).
    pub fast_feed: f64,
    /// Slow probe feed (mm/min).
    pub slow_feed: f64,
    /// Backoff distance (mm).
    pub backoff: f64,
    /// Maximum probe depth (mm).
    pub max_depth: f64,
    /// Edge axis id ("X", "Y", "Z").
    pub edge_axis: Option<String>,
    /// Edge direction id ("negative", "positive").
    pub edge_direction: Option<String>,
    /// Edge/corner probe distance (mm).
    pub edge_distance: f64,
    /// Corner id ("xmin_ymin", ...).
    pub corner: Option<String>,
    /// Bore/boss diameter (mm).
    pub diameter: f64,
    /// Setter plate X (mm).
    pub setter_x: f64,
    /// Setter plate Y (mm).
    pub setter_y: f64,
    /// Setter plate Z (mm).
    pub setter_z: f64,
    /// Depth below the start height for sideways probes (mm).
    pub probe_depth: f64,
    /// Probe tip radius (mm).
    pub tip_radius: f64,
    /// Touch plate thickness (mm).
    pub plate_thickness: f64,
    /// Target WCS id ("G54" ... "G59").
    pub target_wcs: Option<String>,
    /// Write the result into the target WCS from the generated G-code.
    pub auto_update_wcs: bool,
}

impl Default for ProbePreset {
    fn default() -> Self {
        Self {
            format_version: PRESET_FORMAT_VERSION,
            units: PRESET_UNITS.to_string(),
            routine: Some("z_touch".to_string()),
            safe_height: probe_defaults::SAFE_HEIGHT,
            fast_feed: probe_defaults::FAST_FEED,
            slow_feed: probe_defaults::SLOW_FEED,
            backoff: probe_defaults::BACKOFF,
            max_depth: probe_defaults::MAX_PROBE_DEPTH,
            edge_axis: None,
            edge_direction: None,
            edge_distance: probe_defaults::PROBE_DISTANCE,
            corner: None,
            diameter: 20.0,
            setter_x: 100.0,
            setter_y: 100.0,
            setter_z: -50.0,
            probe_depth: probe_defaults::PROBE_DEPTH,
            tip_radius: 0.0,
            plate_thickness: 0.0,
            target_wcs: Some("G54".to_string()),
            auto_update_wcs: false,
        }
    }
}

impl ProbePreset {
    /// Serialize to pretty JSON in the current format (lengths in mm).
    ///
    /// # Returns
    /// The JSON text, or an error if serialization fails.
    pub fn to_json(&self) -> Result<String> {
        let mut out = self.clone();
        out.format_version = PRESET_FORMAT_VERSION;
        out.units = PRESET_UNITS.to_string();
        Ok(serde_json::to_string_pretty(&out)?)
    }

    /// Parse a probe preset file of any supported version.
    ///
    /// Version 1 files stored entry text; probe lengths were always entered in mm.
    ///
    /// # Arguments
    /// * `json` - File contents.
    ///
    /// # Returns
    /// The preset and any warnings, or an error if the file is not valid JSON or uses an
    /// unsupported unit/version.
    pub fn from_json(json: &str) -> Result<PresetLoad<Self>> {
        let value: Value =
            serde_json::from_str(json).map_err(|e| anyhow!("Invalid preset file: {}", e))?;
        if value.get("format_version").is_some() {
            check_version_and_units(&value)?;
            let preset: Self =
                serde_json::from_value(value).map_err(|e| anyhow!("Invalid preset file: {}", e))?;
            return Ok(PresetLoad {
                preset,
                warnings: Vec::new(),
            });
        }

        let mut r = LegacyReader::new(&value);
        let mut p = Self::default();
        r.opt_string("routine", &mut p.routine);
        r.number("safe_height", &mut p.safe_height);
        r.number("fast_feed", &mut p.fast_feed);
        r.number("slow_feed", &mut p.slow_feed);
        r.number("backoff", &mut p.backoff);
        r.number("max_depth", &mut p.max_depth);
        r.opt_string("edge_axis", &mut p.edge_axis);
        r.opt_string("edge_direction", &mut p.edge_direction);
        r.number("edge_distance", &mut p.edge_distance);
        r.opt_string("corner", &mut p.corner);
        r.number("diameter", &mut p.diameter);
        r.number("setter_x", &mut p.setter_x);
        r.number("setter_y", &mut p.setter_y);
        r.number("setter_z", &mut p.setter_z);
        Ok(PresetLoad {
            preset: p,
            warnings: r.warnings,
        })
    }
}

/// Reject files from a newer format or with a unit other than mm.
fn check_version_and_units(value: &Value) -> Result<()> {
    let version = value
        .get("format_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("Invalid preset file: format_version is not a number"))?;
    if version > u64::from(PRESET_FORMAT_VERSION) {
        return Err(anyhow!(
            "Preset file format {} is newer than this version supports ({})",
            version,
            PRESET_FORMAT_VERSION
        ));
    }
    match value.get("units").and_then(Value::as_str) {
        None | Some(PRESET_UNITS) => Ok(()),
        Some(other) => Err(anyhow!("Unsupported preset units '{}'", other)),
    }
}

/// Reads version 1 (text-valued) preset fields, collecting a warning for each bad field.
struct LegacyReader<'a> {
    value: &'a Value,
    warnings: Vec<String>,
}

impl<'a> LegacyReader<'a> {
    fn new(value: &'a Value) -> Self {
        Self {
            value,
            warnings: Vec::new(),
        }
    }

    fn bad(&mut self, key: &str, raw: &Value) {
        warn!(field = key, value = %raw, "Ignoring unreadable preset field");
        self.warnings.push(format!(
            "'{}' has an invalid value ({}); default kept",
            key, raw
        ));
    }

    fn string(&mut self, key: &str, out: &mut String) {
        if let Some(raw) = self.value.get(key) {
            match raw.as_str() {
                Some(v) => *out = v.to_string(),
                None => self.bad(key, raw),
            }
        }
    }

    fn opt_string(&mut self, key: &str, out: &mut Option<String>) {
        if let Some(raw) = self.value.get(key) {
            match raw {
                Value::Null => {}
                Value::String(v) => *out = Some(v.clone()),
                other => self.bad(key, other),
            }
        }
    }

    fn boolean(&mut self, key: &str, out: &mut bool) {
        if let Some(raw) = self.value.get(key) {
            match raw.as_bool() {
                Some(v) => *out = v,
                None => self.bad(key, raw),
            }
        }
    }

    /// Plain number stored as text (or as a JSON number).
    fn number<T: std::str::FromStr>(&mut self, key: &str, out: &mut T) {
        if let Some(raw) = self.value.get(key) {
            let parsed = match raw {
                Value::String(s) => s.trim().parse::<T>().ok(),
                Value::Number(n) => n.to_string().parse::<T>().ok(),
                _ => None,
            };
            match parsed {
                Some(v) => *out = v,
                None => self.bad(key, raw),
            }
        }
    }

    /// Length stored as display text in `system`.
    fn length(&mut self, key: &str, system: MeasurementSystem, out: &mut f32) {
        if let Some(raw) = self.value.get(key) {
            let parsed = match raw {
                Value::String(s) => units::parse_length(s, system).ok(),
                _ => None,
            };
            match parsed {
                Some(v) if v.is_finite() => *out = v,
                _ => self.bad(key, raw),
            }
        }
    }
}
