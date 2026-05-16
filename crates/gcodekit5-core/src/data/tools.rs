//! CAM Tools Palette module - Tool definitions and library management
//!
//! This module provides:
//! - Tool types and categories
//! - Tool geometry and specifications
//! - Tool library management (add, remove, search, filter)
//! - Material-specific tool cutting parameters
//! - Standard tool library initialization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool types for classification.
///
/// Categorizes CNC cutting tools by their geometry and intended use.
/// Each type has different cutting characteristics and suitable operations.
///
/// # Example
/// ```
/// use gcodekit5_core::data::tools::ToolType;
///
/// let tool_type = ToolType::EndMillFlat;
/// let all_types = ToolType::all();
/// assert!(all_types.contains(&tool_type));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ToolType {
    /// Flat end mill
    EndMillFlat,
    /// Ball end mill / ball nose
    EndMillBall,
    /// Corner radius end mill
    EndMillCornerRadius,
    /// V-bit engraving tool
    VBit,
    /// Drill bit (twist drill)
    DrillBit,
    /// Spot drill
    SpotDrill,
    /// Engraving tool
    EngravingBit,
    /// Chamfer tool
    ChamferTool,
    /// Specialty tool
    Specialty,
}

impl ToolType {
    /// Get all tool types
    pub fn all() -> &'static [ToolType] {
        &[
            ToolType::EndMillFlat,
            ToolType::EndMillBall,
            ToolType::EndMillCornerRadius,
            ToolType::VBit,
            ToolType::DrillBit,
            ToolType::SpotDrill,
            ToolType::EngravingBit,
            ToolType::ChamferTool,
            ToolType::Specialty,
        ]
    }
}

impl std::fmt::Display for ToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EndMillFlat => write!(f, "Flat End Mill"),
            Self::EndMillBall => write!(f, "Ball End Mill"),
            Self::EndMillCornerRadius => write!(f, "Corner Radius End Mill"),
            Self::VBit => write!(f, "V-Bit"),
            Self::DrillBit => write!(f, "Drill Bit"),
            Self::SpotDrill => write!(f, "Spot Drill"),
            Self::EngravingBit => write!(f, "Engraving Bit"),
            Self::ChamferTool => write!(f, "Chamfer Tool"),
            Self::Specialty => write!(f, "Specialty"),
        }
    }
}

/// Tool material composition.
///
/// Determines tool hardness, heat resistance, and suitable workpiece materials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolMaterial {
    /// High Speed Steel
    HSS,
    /// Carbide
    Carbide,
    /// Coated carbide
    CoatedCarbide,
    /// Diamond coated
    Diamond,
}

impl std::fmt::Display for ToolMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HSS => write!(f, "HSS"),
            Self::Carbide => write!(f, "Carbide"),
            Self::CoatedCarbide => write!(f, "Coated Carbide"),
            Self::Diamond => write!(f, "Diamond Coated"),
        }
    }
}

/// Tool coating type.
///
/// Surface coatings that improve tool life, heat resistance, and cutting performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCoating {
    /// Titanium Nitride coating
    TiN,
    /// Titanium Aluminum Nitride coating
    TiAlN,
    /// Diamond-like carbon coating
    DLC,
    /// Aluminum Oxide coating
    AlOx,
}

impl std::fmt::Display for ToolCoating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TiN => write!(f, "TiN"),
            Self::TiAlN => write!(f, "TiAlN"),
            Self::DLC => write!(f, "DLC"),
            Self::AlOx => write!(f, "Al2O3"),
        }
    }
}

/// Shank type for tool holder compatibility.
///
/// Determines which collet or holder the tool requires.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShankType {
    /// Straight shank with specified diameter in 1/10mm units (e.g., 60 = 6.0mm).
    Straight(u32),
    /// Tapered shank
    Tapered,
    /// Collet size (e.g., ER-20, ER-25)
    Collet,
}

/// Tool identifier.
///
/// A string-based unique identifier for tools within a library.
/// Convention: `"std_<type>_<diameter>"` for standard tools,
/// `"gtc_<id>"` for imported tools.
///
/// # Example
/// ```
/// use gcodekit5_core::data::tools::ToolId;
///
/// let id = ToolId("std_endmill_6mm".to_string());
/// assert_eq!(id.to_string(), "std_endmill_6mm");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ToolId(
    /// The unique string identifier for the tool.
    pub String,
);

impl std::fmt::Display for ToolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tool default cutting parameters.
///
/// Recommended operating parameters for the tool. Values are in mm and mm/min.
/// These serve as starting points; actual values depend on workpiece material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCuttingParams {
    /// Recommended RPM
    pub rpm: u32,
    /// RPM range (min, max)
    pub rpm_range: (u32, u32),
    /// Default feed rate in mm/min
    pub feed_rate: f32,
    /// Default plunge rate in mm/min
    pub plunge_rate: f32,
    /// Default stepover as percentage of diameter
    pub stepover_percent: f32,
    /// Default depth per pass in mm
    pub depth_per_pass: f32,
    /// Minimum depth of cut for engagement in mm
    #[serde(default)]
    pub min_doc: f32,
    /// Maximum depth of cut as percentage of diameter
    #[serde(default)]
    pub max_doc_percent: f32,
}

/// Extended tool geometry specifications.
///
/// Advanced geometry parameters for feeds and speeds calculations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolGeometry {
    /// Helix angle in degrees (30-45° typical, higher = better chip evacuation)
    #[serde(default)]
    pub helix_angle: Option<f32>,
    /// Tool core diameter in mm (for deflection calculations)
    #[serde(default)]
    pub core_diameter: Option<f32>,
    /// Rake angle in degrees (affects chip formation)
    #[serde(default)]
    pub rake_angle: Option<f32>,
    /// Relief/clearance angle in degrees (affects cutting efficiency)
    #[serde(default)]
    pub relief_angle: Option<f32>,
}

/// Tool operating limits and ratings.
///
/// Manufacturer-specified limits for safe operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolLimits {
    /// Maximum safe RPM rating from manufacturer
    #[serde(default)]
    pub max_rpm_rating: Option<u32>,
    /// Maximum recommended stick-out length in mm
    #[serde(default)]
    pub stick_out_max: Option<f32>,
    /// Maximum chip load per tooth in mm
    #[serde(default)]
    pub max_chip_load: Option<f32>,
}

/// Tool feature flags for advanced capabilities.
///
/// Boolean flags indicating special tool features.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolFeatures {
    /// Tool has chip breaker geometry
    #[serde(default)]
    pub chip_breaker: bool,
    /// Tool supports through-spindle coolant
    #[serde(default)]
    pub through_coolant: bool,
}

impl Default for ToolCuttingParams {
    fn default() -> Self {
        Self {
            rpm: 12000,
            rpm_range: (8000, 18000),
            feed_rate: 1500.0,
            plunge_rate: 750.0,
            stepover_percent: 50.0,
            depth_per_pass: 3.0,
            min_doc: 0.1,
            max_doc_percent: 100.0,
        }
    }
}

impl ToolCuttingParams {
    /// Builder method to set RPM.
    pub fn with_rpm(mut self, rpm: u32) -> Self {
        self.rpm = rpm;
        self
    }

    /// Builder method to set RPM range.
    pub fn with_rpm_range(mut self, min: u32, max: u32) -> Self {
        self.rpm_range = (min, max);
        self
    }

    /// Builder method to set feed rate in mm/min.
    pub fn with_feed_rate(mut self, rate: f32) -> Self {
        self.feed_rate = rate;
        self
    }

    /// Builder method to set plunge rate in mm/min.
    pub fn with_plunge_rate(mut self, rate: f32) -> Self {
        self.plunge_rate = rate;
        self
    }

    /// Builder method to set stepover as percentage of tool diameter.
    pub fn with_stepover_percent(mut self, percent: f32) -> Self {
        self.stepover_percent = percent;
        self
    }

    /// Builder method to set depth per pass in mm.
    pub fn with_depth_per_pass(mut self, depth: f32) -> Self {
        self.depth_per_pass = depth;
        self
    }

    /// Builder method to set minimum depth of cut in mm.
    pub fn with_min_doc(mut self, min_doc: f32) -> Self {
        self.min_doc = min_doc;
        self
    }

    /// Builder method to set max depth of cut as percentage of diameter.
    pub fn with_max_doc_percent(mut self, percent: f32) -> Self {
        self.max_doc_percent = percent;
        self
    }
}

impl ToolGeometry {
    /// Create new ToolGeometry with all None values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method to set helix angle in degrees.
    pub fn with_helix_angle(mut self, angle: f32) -> Self {
        self.helix_angle = Some(angle);
        self
    }

    /// Builder method to set core diameter in mm.
    pub fn with_core_diameter(mut self, diameter: f32) -> Self {
        self.core_diameter = Some(diameter);
        self
    }

    /// Builder method to set rake angle in degrees.
    pub fn with_rake_angle(mut self, angle: f32) -> Self {
        self.rake_angle = Some(angle);
        self
    }

    /// Builder method to set relief/clearance angle in degrees.
    pub fn with_relief_angle(mut self, angle: f32) -> Self {
        self.relief_angle = Some(angle);
        self
    }
}

impl ToolLimits {
    /// Create new ToolLimits with all None values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method to set max RPM rating.
    pub fn with_max_rpm_rating(mut self, rpm: u32) -> Self {
        self.max_rpm_rating = Some(rpm);
        self
    }

    /// Builder method to set max stick-out length in mm.
    pub fn with_stick_out_max(mut self, length: f32) -> Self {
        self.stick_out_max = Some(length);
        self
    }

    /// Builder method to set max chip load in mm/tooth.
    pub fn with_max_chip_load(mut self, load: f32) -> Self {
        self.max_chip_load = Some(load);
        self
    }
}

impl ToolFeatures {
    /// Create new ToolFeatures with all false values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method to set chip breaker flag.
    pub fn with_chip_breaker(mut self, enabled: bool) -> Self {
        self.chip_breaker = enabled;
        self
    }

    /// Builder method to set through-coolant flag.
    pub fn with_through_coolant(mut self, enabled: bool) -> Self {
        self.through_coolant = enabled;
        self
    }
}

/// Complete tool definition.
///
/// Stores all properties of a CNC cutting tool including geometry, material,
/// cutting parameters, and metadata. All dimensions are in millimeters.
///
/// # Example
/// ```
/// use gcodekit5_core::data::tools::{Tool, ToolId, ToolType};
///
/// let tool = Tool::new(
///     ToolId("endmill_6mm".to_string()),
///     1,
///     "6mm Flat End Mill".to_string(),
///     ToolType::EndMillFlat,
///     6.0,
///     50.0,
/// );
/// assert_eq!(tool.diameter, 6.0);
/// assert_eq!(tool.flutes, 2);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Unique tool identifier
    pub id: ToolId,
    /// Tool number (for reference)
    pub number: u32,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Tool type
    pub tool_type: ToolType,

    // Geometry
    /// Cutting diameter in mm
    pub diameter: f32,
    /// Shaft diameter in mm (if different)
    pub shaft_diameter: Option<f32>,
    /// Overall length in mm
    pub length: f32,
    /// Flute length in mm
    pub flute_length: f32,
    /// Number of flutes
    pub flutes: u32,
    /// Corner radius in mm (for corner radius end mills)
    pub corner_radius: Option<f32>,
    /// Tip angle in degrees (for v-bits, drills)
    pub tip_angle: Option<f32>,

    // Material specs
    /// Tool material composition
    pub material: ToolMaterial,
    /// Optional coating
    pub coating: Option<ToolCoating>,
    /// Shank type
    pub shank: ShankType,

    // Parameters
    /// Default cutting parameters
    pub params: ToolCuttingParams,
    /// Extended geometry specifications for feeds/speeds calculations
    #[serde(default)]
    pub geometry: ToolGeometry,
    /// Tool operating limits and ratings
    #[serde(default)]
    pub limits: ToolLimits,
    /// Tool feature flags
    #[serde(default)]
    pub features: ToolFeatures,

    // Metadata    /// Manufacturer name
    pub manufacturer: Option<String>,
    /// Manufacturer part number
    pub part_number: Option<String>,
    /// Cost per unit
    pub cost: Option<f32>,
    /// Notes and tips
    pub notes: String,
    /// Whether this is a user-defined custom tool
    pub custom: bool,
}

impl Tool {
    /// Create a new tool with basic properties
    pub fn new(
        id: ToolId,
        number: u32,
        name: String,
        tool_type: ToolType,
        diameter: f32,
        length: f32,
    ) -> Self {
        Self {
            id,
            number,
            name,
            description: String::new(),
            tool_type,
            diameter,
            shaft_diameter: None,
            length,
            flute_length: length - 10.0,
            flutes: 2,
            corner_radius: None,
            tip_angle: None,
            material: ToolMaterial::Carbide,
            coating: Some(ToolCoating::TiN),
            shank: ShankType::Collet,
            params: ToolCuttingParams::default(),
            geometry: ToolGeometry::default(),
            limits: ToolLimits::default(),
            features: ToolFeatures::default(),
            manufacturer: None,
            part_number: None,
            cost: None,
            notes: String::new(),
            custom: false,
        }
    }

    /// Get a descriptive string for the tool
    pub fn description_short(&self) -> String {
        format!(
            "{} - {} dia x {} length, {} flutes",
            self.name, self.diameter, self.length, self.flutes
        )
    }

    /// Check if tool is suitable for a specific material
    pub fn is_suitable_for_material(&self, material_category: &str) -> bool {
        // For now, most tools work with most materials
        // This can be expanded with material compatibility rules
        !material_category.is_empty()
    }

    /// Builder method to set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Builder method to set shaft diameter.
    pub fn with_shaft_diameter(mut self, diameter: f32) -> Self {
        self.shaft_diameter = Some(diameter);
        self
    }

    /// Builder method to set flute length.
    pub fn with_flute_length(mut self, length: f32) -> Self {
        self.flute_length = length;
        self
    }

    /// Builder method to set number of flutes.
    pub fn with_flutes(mut self, flutes: u32) -> Self {
        self.flutes = flutes;
        self
    }

    /// Builder method to set corner radius.
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = Some(radius);
        self
    }

    /// Builder method to set tip angle in degrees.
    pub fn with_tip_angle(mut self, angle: f32) -> Self {
        self.tip_angle = Some(angle);
        self
    }

    /// Builder method to set tool material.
    pub fn with_material(mut self, material: ToolMaterial) -> Self {
        self.material = material;
        self
    }

    /// Builder method to set tool coating.
    pub fn with_coating(mut self, coating: ToolCoating) -> Self {
        self.coating = Some(coating);
        self
    }

    /// Builder method to set shank type.
    pub fn with_shank(mut self, shank: ShankType) -> Self {
        self.shank = shank;
        self
    }

    /// Builder method to set cutting parameters.
    pub fn with_params(mut self, params: ToolCuttingParams) -> Self {
        self.params = params;
        self
    }

    /// Builder method to set manufacturer.
    pub fn with_manufacturer(mut self, manufacturer: impl Into<String>) -> Self {
        self.manufacturer = Some(manufacturer.into());
        self
    }

    /// Builder method to set part number.
    pub fn with_part_number(mut self, part_number: impl Into<String>) -> Self {
        self.part_number = Some(part_number.into());
        self
    }

    /// Builder method to set cost.
    pub fn with_cost(mut self, cost: f32) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Builder method to set notes.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }

    /// Builder method to mark as custom tool.
    pub fn with_custom(mut self, custom: bool) -> Self {
        self.custom = custom;
        self
    }

    /// Builder method to set extended geometry.
    pub fn with_geometry(mut self, geometry: ToolGeometry) -> Self {
        self.geometry = geometry;
        self
    }

    /// Builder method to set tool limits.
    pub fn with_limits(mut self, limits: ToolLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Builder method to set tool features.
    pub fn with_features(mut self, features: ToolFeatures) -> Self {
        self.features = features;
        self
    }
}
/// Tool library — manages a collection of tools.
///
/// Provides add, remove, search, and filter operations. Standard tools
/// are loaded via [`init_standard_library`].
///
/// # Example
/// ```
/// use gcodekit5_core::data::tools::{ToolLibrary, Tool, ToolId, ToolType};
///
/// let mut library = ToolLibrary::new();
/// let tool = Tool::new(
///     ToolId("t1".to_string()), 1,
///     "Test".to_string(), ToolType::DrillBit, 3.0, 40.0,
/// );
/// library.add_tool(tool);
/// assert_eq!(library.len(), 1);
/// assert!(library.get_tool(&ToolId("t1".to_string())).is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolLibrary {
    /// Collection of tools by ID
    tools: HashMap<ToolId, Tool>,
    /// Next available tool number
    next_tool_number: u32,
}

impl ToolLibrary {
    /// Create a new empty tool library
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            next_tool_number: 1,
        }
    }

    /// Add a tool to the library
    pub fn add_tool(&mut self, tool: Tool) {
        if tool.number >= self.next_tool_number {
            self.next_tool_number = tool.number + 1;
        }
        self.tools.insert(tool.id.clone(), tool);
    }

    /// Get a tool by ID
    pub fn get_tool(&self, id: &ToolId) -> Option<&Tool> {
        self.tools.get(id)
    }

    /// Get a mutable reference to a tool
    pub fn get_tool_mut(&mut self, id: &ToolId) -> Option<&mut Tool> {
        self.tools.get_mut(id)
    }

    /// Remove a tool from the library
    pub fn remove_tool(&mut self, id: &ToolId) -> Option<Tool> {
        self.tools.remove(id)
    }

    /// Get all tools
    pub fn get_all_tools(&self) -> Vec<&Tool> {
        self.tools.values().collect()
    }

    /// Get tools by type
    pub fn get_tools_by_type(&self, tool_type: ToolType) -> Vec<&Tool> {
        self.tools
            .values()
            .filter(|t| t.tool_type == tool_type)
            .collect()
    }

    /// Search tools by name (partial match, case-insensitive)
    pub fn search_by_name(&self, query: &str) -> Vec<&Tool> {
        let query_lower = query.to_lowercase();
        self.tools
            .values()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower)
                    || t.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Search tools by diameter range
    pub fn search_by_diameter(&self, min: f32, max: f32) -> Vec<&Tool> {
        self.tools
            .values()
            .filter(|t| t.diameter >= min && t.diameter <= max)
            .collect()
    }

    /// Get the next available tool number
    pub fn next_tool_number(&self) -> u32 {
        self.next_tool_number
    }

    /// Get the number of tools in the library
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if library is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolLibrary {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize standard tool library with common tools
pub fn init_standard_library() -> ToolLibrary {
    let mut library = ToolLibrary::new();

    // 1/4" Flat End Mill
    let mut tool1 = Tool::new(
        ToolId("tool_1_4_flat".to_string()),
        1,
        "1/4\" Flat End Mill".to_string(),
        ToolType::EndMillFlat,
        6.35,
        50.0,
    );
    tool1.flutes = 2;
    tool1.flute_length = 40.0;
    tool1.material = ToolMaterial::Carbide;
    tool1.coating = Some(ToolCoating::TiN);
    tool1.manufacturer = Some("Generic".to_string());
    tool1.params.rpm = 18000;
    tool1.params.rpm_range = (12000, 24000);
    tool1.params.feed_rate = 1500.0;
    library.add_tool(tool1);

    // 1/8" Flat End Mill
    let mut tool2 = Tool::new(
        ToolId("tool_1_8_flat".to_string()),
        2,
        "1/8\" Flat End Mill".to_string(),
        ToolType::EndMillFlat,
        3.175,
        45.0,
    );
    tool2.flutes = 2;
    tool2.flute_length = 35.0;
    tool2.material = ToolMaterial::Carbide;
    tool2.coating = Some(ToolCoating::TiN);
    tool2.params.rpm = 24000;
    tool2.params.rpm_range = (18000, 30000);
    tool2.params.feed_rate = 1000.0;
    library.add_tool(tool2);

    // 90 degree V-Bit
    let mut tool3 = Tool::new(
        ToolId("tool_vbit_90".to_string()),
        3,
        "90° V-Bit".to_string(),
        ToolType::VBit,
        6.0,
        50.0,
    );
    tool3.flutes = 1;
    tool3.tip_angle = Some(90.0);
    tool3.material = ToolMaterial::Carbide;
    tool3.coating = Some(ToolCoating::TiN);
    tool3.params.rpm = 20000;
    tool3.params.rpm_range = (15000, 25000);
    tool3.params.feed_rate = 1200.0;
    tool3.params.depth_per_pass = 2.0;
    library.add_tool(tool3);

    // 1/4" Drill Bit
    let mut tool4 = Tool::new(
        ToolId("tool_drill_1_4".to_string()),
        4,
        "1/4\" Drill Bit".to_string(),
        ToolType::DrillBit,
        6.35,
        60.0,
    );
    tool4.flutes = 2;
    tool4.tip_angle = Some(118.0);
    tool4.material = ToolMaterial::HSS;
    tool4.params.rpm = 3000;
    tool4.params.rpm_range = (2000, 4000);
    tool4.params.feed_rate = 300.0;
    tool4.params.plunge_rate = 300.0;
    library.add_tool(tool4);

    // Ball End Mill 1/8"
    let mut tool5 = Tool::new(
        ToolId("tool_1_8_ball".to_string()),
        5,
        "1/8\" Ball End Mill".to_string(),
        ToolType::EndMillBall,
        3.175,
        45.0,
    );
    tool5.flutes = 2;
    tool5.flute_length = 35.0;
    tool5.material = ToolMaterial::Carbide;
    tool5.coating = Some(ToolCoating::TiAlN);
    tool5.params.rpm = 22000;
    tool5.params.rpm_range = (16000, 28000);
    tool5.params.feed_rate = 1200.0;
    tool5.params.stepover_percent = 20.0;
    library.add_tool(tool5);

    // Precision Fly Cutter
    let mut tool6 = Tool::new(
        ToolId("tool_fly_cutter_50mm".to_string()),
        6,
        "Precision Fly Cutter".to_string(),
        ToolType::Specialty,
        50.0, // 50mm cutting diameter (approx 2 inch)
        60.0, // Length
    );
    tool6.flutes = 1; // Single point cutter
    tool6.shaft_diameter = Some(12.7); // 1/2 inch shank
    tool6.material = ToolMaterial::Carbide; // Holder is steel, bit is carbide
    tool6.manufacturer = Some("Buyohlic".to_string());
    tool6.description = "Precision Fly Cutter with 1/2\" Shank. Ideal for surfacing steel, cast iron, and aluminum.".to_string();
    tool6.params.rpm = 1500; // Slower for fly cutters
    tool6.params.rpm_range = (500, 3000);
    tool6.params.feed_rate = 300.0;
    tool6.params.plunge_rate = 100.0;
    tool6.params.stepover_percent = 70.0; // Large stepover for facing
    tool6.params.depth_per_pass = 0.5;
    library.add_tool(tool6);

    // NITOMAK Surfacing Router Bit
    let mut tool7 = Tool::new(
        ToolId("tool_nitomak_surfacing_2in".to_string()),
        7,
        "NITOMAK Surfacing Router Bit".to_string(),
        ToolType::Specialty,
        50.8, // 2 inch
        60.0, // Assumed overall length
    );
    tool7.flutes = 3;
    tool7.flute_length = 12.7; // 1/2 inch cutting length
    tool7.shaft_diameter = Some(12.7); // 1/2 inch shank
    tool7.material = ToolMaterial::Carbide;
    tool7.manufacturer = Some("NITOMAK".to_string());
    tool7.description = "CNC Spoilboard Surfacing Router Bit, 1/2\" Shank, 2\" Cutting Diameter, 3 Flutes. Teflon coated.".to_string();
    tool7.params.rpm = 12000; // Router bits usually run faster than fly cutters
    tool7.params.rpm_range = (10000, 18000);
    tool7.params.feed_rate = 2000.0;
    tool7.params.stepover_percent = 40.0;
    tool7.params.depth_per_pass = 1.0;
    library.add_tool(tool7);

    library
}
