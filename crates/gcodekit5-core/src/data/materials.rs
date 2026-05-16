//! Materials Database module
//!
//! This module provides:
//! - Material categories and types
//! - Material properties (physical, mechanical, machining, safety)
//! - Cutting parameter recommendations
//! - Material library management
//! - Custom material support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Material categories for organization.
///
/// Top-level grouping for materials. Each category has different machining
/// characteristics and safety considerations.
///
/// # Example
/// ```
/// use gcodekit5_core::data::materials::MaterialCategory;
///
/// let cat = MaterialCategory::Wood;
/// assert_eq!(format!("{:?}", cat), "Wood");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum MaterialCategory {
    /// Natural wood (hardwoods, softwoods)
    Wood,
    /// Engineered wood products
    EngineeredWood,
    /// Plastic and polymer materials
    Plastic,
    /// Non-ferrous metals (aluminum, brass, copper)
    NonFerrousMetal,
    /// Ferrous metals (steel, stainless)
    FerrousMetal,
    /// Composite materials (carbon fiber, fiberglass)
    Composite,
    /// Stone and ceramic materials
    StoneAndCeramic,
}

impl std::fmt::Display for MaterialCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wood => write!(f, "Wood"),
            Self::EngineeredWood => write!(f, "Engineered Wood"),
            Self::Plastic => write!(f, "Plastic"),
            Self::NonFerrousMetal => write!(f, "Non-Ferrous Metal"),
            Self::FerrousMetal => write!(f, "Ferrous Metal"),
            Self::Composite => write!(f, "Composite"),
            Self::StoneAndCeramic => write!(f, "Stone and Ceramic"),
        }
    }
}

/// Chip formation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChipType {
    /// Continuous chips (most metals, harder plastics)
    Continuous,
    /// Segmented chips (gray cast iron)
    Segmented,
    /// Granular or powdery chips (composites, ceramics)
    Granular,
    /// Very small, breakable chips (some plastics)
    Small,
}

/// Heat sensitivity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeatSensitivity {
    /// Low heat sensitivity (woods, most metals)
    Low,
    /// Moderate heat sensitivity
    Moderate,
    /// High heat sensitivity (thermoplastics, composites)
    High,
}

/// Abrasiveness level (effect on tool wear)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Abrasiveness {
    /// Low wear (aluminum, wood)
    Low,
    /// Moderate wear (mild steel)
    Moderate,
    /// High wear (stainless, composites)
    High,
}

/// Surface finish achievability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceFinishability {
    /// Excellent surface finish possible
    Excellent,
    /// Good surface finish with proper technique
    Good,
    /// Fair surface finish, may need secondary finishing
    Fair,
    /// Rough surface finish expected
    Rough,
}

/// Hazard levels for safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HazardLevel {
    /// No special hazard
    None,
    /// Minimal hazard
    Minimal,
    /// Moderate hazard, PPE recommended
    Moderate,
    /// High hazard, PPE required
    High,
}

/// Personal Protective Equipment requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PPE {
    /// Safety glasses/face shield
    EyeProtection,
    /// Dust mask or respirator
    Respiratory,
    /// Hearing protection
    HearingProtection,
    /// Gloves
    Gloves,
    /// Apron
    Apron,
}

/// Coolant/Lubrication type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoolantType {
    /// No coolant needed
    None,
    /// Air only (dust blowout)
    AirOnly,
    /// Mineral oil based coolant
    MineralOil,
    /// Water soluble coolant
    WaterSoluble,
    /// Synthetic coolant
    Synthetic,
}

/// Cutting parameters for a specific material and tool combination.
///
/// Recommended operating ranges that vary by workpiece material. All linear
/// values are in mm, rates in mm/min. These are starting points — adjust
/// based on actual cutting conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuttingParameters {
    /// RPM range (min, max)
    pub rpm_range: (u32, u32),
    /// Feed rate range in mm/min (min, max) for roughing
    pub feed_rate_range: (f32, f32),
    /// Plunge rate as percentage of feed rate (0-100)
    pub plunge_rate_percent: f32,
    /// Maximum depth of cut in mm
    pub max_doc: f32,
    /// Stepover range as percentage of tool diameter (min, max)
    pub stepover_percent: (f32, f32),
    /// Recommended surface speed in m/min (SFM equivalent)
    #[serde(default)]
    pub surface_speed_m_min: Option<f32>,
    /// Recommended chip load in mm/tooth
    #[serde(default)]
    pub chip_load_mm: Option<f32>,
    /// Surface speed range in SFM (Surface Feet per Minute) - min, max
    #[serde(default)]
    pub sfm_range: Option<(f32, f32)>,
    /// Chip load range in mm/tooth - min, max
    #[serde(default)]
    pub chipload_range: Option<(f32, f32)>,
    /// Power factor for cutting force calculations (multiplier)
    #[serde(default)]
    pub power_factor: Option<f32>,
    /// Maximum ramp/plunge angle in degrees
    #[serde(default)]
    pub ramp_angle_max: Option<f32>,
    /// Maximum stepover for adaptive/high-speed machining (% of diameter)
    #[serde(default)]
    pub adaptive_stepover_max: Option<f32>,
    /// Recommended coolant type
    pub coolant_type: CoolantType,
    /// Notes about parameters
    pub notes: String,
}

impl Default for CuttingParameters {
    fn default() -> Self {
        Self {
            rpm_range: (12000, 18000),
            feed_rate_range: (1000.0, 2000.0),
            plunge_rate_percent: 50.0,
            max_doc: 3.0,
            stepover_percent: (40.0, 60.0),
            surface_speed_m_min: None,
            chip_load_mm: None,
            sfm_range: None,
            chipload_range: None,
            power_factor: None,
            ramp_angle_max: None,
            adaptive_stepover_max: None,
            coolant_type: CoolantType::None,
            notes: String::new(),
        }
    }
}

impl CuttingParameters {
    /// Builder method to set RPM range.
    pub fn with_rpm_range(mut self, min: u32, max: u32) -> Self {
        self.rpm_range = (min, max);
        self
    }

    /// Builder method to set feed rate range in mm/min.
    pub fn with_feed_rate_range(mut self, min: f32, max: f32) -> Self {
        self.feed_rate_range = (min, max);
        self
    }

    /// Builder method to set plunge rate as percentage of feed rate.
    pub fn with_plunge_rate_percent(mut self, percent: f32) -> Self {
        self.plunge_rate_percent = percent;
        self
    }

    /// Builder method to set max depth of cut in mm.
    pub fn with_max_doc(mut self, depth: f32) -> Self {
        self.max_doc = depth;
        self
    }

    /// Builder method to set stepover percentage range.
    pub fn with_stepover_percent(mut self, min: f32, max: f32) -> Self {
        self.stepover_percent = (min, max);
        self
    }

    /// Builder method to set surface speed in m/min.
    pub fn with_surface_speed(mut self, speed: f32) -> Self {
        self.surface_speed_m_min = Some(speed);
        self
    }

    /// Builder method to set chip load in mm/tooth.
    pub fn with_chip_load(mut self, load: f32) -> Self {
        self.chip_load_mm = Some(load);
        self
    }

    /// Builder method to set SFM range.
    pub fn with_sfm_range(mut self, min: f32, max: f32) -> Self {
        self.sfm_range = Some((min, max));
        self
    }

    /// Builder method to set chipload range.
    pub fn with_chipload_range(mut self, min: f32, max: f32) -> Self {
        self.chipload_range = Some((min, max));
        self
    }

    /// Builder method to set power factor.
    pub fn with_power_factor(mut self, factor: f32) -> Self {
        self.power_factor = Some(factor);
        self
    }

    /// Builder method to set max ramp angle.
    pub fn with_ramp_angle_max(mut self, angle: f32) -> Self {
        self.ramp_angle_max = Some(angle);
        self
    }

    /// Builder method to set max adaptive stepover.
    pub fn with_adaptive_stepover_max(mut self, stepover: f32) -> Self {
        self.adaptive_stepover_max = Some(stepover);
        self
    }

    /// Builder method to set coolant type.
    pub fn with_coolant_type(mut self, coolant: CoolantType) -> Self {
        self.coolant_type = coolant;
        self
    }

    /// Builder method to set notes.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

/// Material identifier.
///
/// A string-based unique identifier for materials within a library.
/// Convention: `"<category>_<name>"`, e.g. `"wood_red_oak"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct MaterialId(
    /// The unique string identifier for the material.
    pub String,
);

impl std::fmt::Display for MaterialId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Complete material definition.
///
/// Stores physical properties, machining characteristics, safety information,
/// and tool-specific cutting parameters. Used by CAM tools to recommend
/// feeds and speeds.
///
/// # Example
/// ```
/// use gcodekit5_core::data::materials::{Material, MaterialCategory, MaterialId};
///
/// let material = Material::new(
///     MaterialId("wood_pine".to_string()),
///     "Pine".to_string(),
///     MaterialCategory::Wood,
///     "Softwood".to_string(),
/// );
/// assert_eq!(material.machinability_desc(), "Easy");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    /// Unique material identifier
    pub id: MaterialId,
    /// Display name
    pub name: String,
    /// Material category
    pub category: MaterialCategory,
    /// Subcategory (e.g., "Red Oak" for hardwood)
    pub subcategory: String,
    /// Brief description
    pub description: String,

    // Physical properties
    /// Density in kg/m³
    pub density: f32,
    /// Machinability rating (1-10, higher is easier)
    pub machinability_rating: u8,
    /// Tensile strength in MPa (optional)
    pub tensile_strength: Option<f32>,
    /// Melting point or glass transition temperature in °C (optional)
    pub melting_point: Option<f32>,
    /// Brinell hardness (HB) - for SFM adjustment calculations
    #[serde(default)]
    pub brinell_hardness: Option<f32>,
    /// Rockwell hardness C scale (HRC) - for hardened materials
    #[serde(default)]
    pub rockwell_hardness: Option<f32>,
    /// Thermal conductivity in W/(m·K) - for heat dissipation calculations
    #[serde(default)]
    pub thermal_conductivity: Option<f32>,
    /// Specific cutting force (Kc) in N/mm² - for power calculations
    #[serde(default)]
    pub specific_cutting_force: Option<f32>,
    // Machining characteristics
    /// Type of chips formed
    pub chip_type: ChipType,
    /// Heat sensitivity when cutting
    pub heat_sensitivity: HeatSensitivity,
    /// Tool wear factor (abrasiveness)
    pub abrasiveness: Abrasiveness,
    /// Surface finish achievable
    pub surface_finish: SurfaceFinishability,

    // Safety information
    /// Dust hazard level
    pub dust_hazard: HazardLevel,
    /// Fume hazard level
    pub fume_hazard: HazardLevel,
    /// Required PPE
    pub required_ppe: Vec<PPE>,
    /// Is coolant required?
    pub coolant_required: bool,

    // Cutting parameters for different tool types
    /// Cutting parameters (tool type -> parameters)
    pub cutting_params: HashMap<String, CuttingParameters>,

    // Metadata
    /// Whether this is a user-defined custom material
    pub custom: bool,
    /// Notes and tips
    pub notes: String,
}

impl Material {
    /// Create a new material with basic properties
    pub fn new(
        id: MaterialId,
        name: String,
        category: MaterialCategory,
        subcategory: String,
    ) -> Self {
        Self {
            id,
            name,
            category,
            subcategory,
            description: String::new(),
            density: 750.0,
            machinability_rating: 7,
            tensile_strength: None,
            melting_point: None,
            brinell_hardness: None,
            rockwell_hardness: None,
            thermal_conductivity: None,
            specific_cutting_force: None,
            chip_type: ChipType::Continuous,
            heat_sensitivity: HeatSensitivity::Low,
            abrasiveness: Abrasiveness::Low,
            surface_finish: SurfaceFinishability::Good,
            dust_hazard: HazardLevel::Minimal,
            fume_hazard: HazardLevel::None,
            required_ppe: vec![PPE::EyeProtection],
            coolant_required: false,
            cutting_params: HashMap::new(),
            custom: false,
            notes: String::new(),
        }
    }

    /// Get cutting parameters for a specific tool type
    pub fn get_cutting_params(&self, tool_type: &str) -> Option<&CuttingParameters> {
        self.cutting_params.get(tool_type)
    }

    /// Set cutting parameters for a tool type
    pub fn set_cutting_params(&mut self, tool_type: String, params: CuttingParameters) {
        self.cutting_params.insert(tool_type, params);
    }

    /// Get machinability description
    pub fn machinability_desc(&self) -> &'static str {
        match self.machinability_rating {
            1..=2 => "Very Difficult",
            3..=4 => "Difficult",
            5..=6 => "Moderate",
            7..=8 => "Easy",
            9..=10 => "Very Easy",
            _ => "Unknown",
        }
    }

    /// Builder method to set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Builder method to set density in kg/m³.
    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// Builder method to set machinability rating (1-10).
    pub fn with_machinability_rating(mut self, rating: u8) -> Self {
        self.machinability_rating = rating;
        self
    }

    /// Builder method to set tensile strength in MPa.
    pub fn with_tensile_strength(mut self, strength: f32) -> Self {
        self.tensile_strength = Some(strength);
        self
    }

    /// Builder method to set melting point in °C.
    pub fn with_melting_point(mut self, temp: f32) -> Self {
        self.melting_point = Some(temp);
        self
    }

    /// Builder method to set Brinell hardness (HB).
    pub fn with_brinell_hardness(mut self, hardness: f32) -> Self {
        self.brinell_hardness = Some(hardness);
        self
    }

    /// Builder method to set Rockwell hardness C (HRC).
    pub fn with_rockwell_hardness(mut self, hardness: f32) -> Self {
        self.rockwell_hardness = Some(hardness);
        self
    }

    /// Builder method to set thermal conductivity in W/(m·K).
    pub fn with_thermal_conductivity(mut self, conductivity: f32) -> Self {
        self.thermal_conductivity = Some(conductivity);
        self
    }

    /// Builder method to set specific cutting force (Kc) in N/mm².
    pub fn with_specific_cutting_force(mut self, kc: f32) -> Self {
        self.specific_cutting_force = Some(kc);
        self
    }
    /// Builder method to set chip type.
    pub fn with_chip_type(mut self, chip_type: ChipType) -> Self {
        self.chip_type = chip_type;
        self
    }

    /// Builder method to set heat sensitivity.
    pub fn with_heat_sensitivity(mut self, sensitivity: HeatSensitivity) -> Self {
        self.heat_sensitivity = sensitivity;
        self
    }

    /// Builder method to set abrasiveness.
    pub fn with_abrasiveness(mut self, abrasiveness: Abrasiveness) -> Self {
        self.abrasiveness = abrasiveness;
        self
    }

    /// Builder method to set surface finishability.
    pub fn with_surface_finish(mut self, finish: SurfaceFinishability) -> Self {
        self.surface_finish = finish;
        self
    }

    /// Builder method to set dust hazard level.
    pub fn with_dust_hazard(mut self, level: HazardLevel) -> Self {
        self.dust_hazard = level;
        self
    }

    /// Builder method to set fume hazard level.
    pub fn with_fume_hazard(mut self, level: HazardLevel) -> Self {
        self.fume_hazard = level;
        self
    }

    /// Builder method to set required PPE.
    pub fn with_required_ppe(mut self, ppe: Vec<PPE>) -> Self {
        self.required_ppe = ppe;
        self
    }

    /// Builder method to set coolant requirement.
    pub fn with_coolant_required(mut self, required: bool) -> Self {
        self.coolant_required = required;
        self
    }

    /// Builder method to set notes.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }

    /// Builder method to mark as custom material.
    pub fn with_custom(mut self, custom: bool) -> Self {
        self.custom = custom;
        self
    }
}

/// Materials library — manages a collection of materials.
///
/// Provides add, remove, and lookup operations for materials.
/// Standard materials are loaded via [`init_standard_library`].
///
/// # Example
/// ```
/// use gcodekit5_core::data::materials::{MaterialLibrary, Material, MaterialId, MaterialCategory};
///
/// let mut lib = MaterialLibrary::new();
/// let mat = Material::new(
///     MaterialId("wood_oak".to_string()), "Oak".to_string(),
///     MaterialCategory::Wood, "Hardwood".to_string(),
/// );
/// lib.add_material(mat);
/// assert_eq!(lib.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialLibrary {
    /// Collection of materials by ID
    materials: HashMap<MaterialId, Material>,
}

impl MaterialLibrary {
    /// Create a new empty library
    pub fn new() -> Self {
        Self {
            materials: HashMap::new(),
        }
    }

    /// Add a material to the library
    pub fn add_material(&mut self, material: Material) {
        self.materials.insert(material.id.clone(), material);
    }

    /// Get a material by ID
    pub fn get_material(&self, id: &MaterialId) -> Option<&Material> {
        self.materials.get(id)
    }

    /// Get a mutable reference to a material
    pub fn get_material_mut(&mut self, id: &MaterialId) -> Option<&mut Material> {
        self.materials.get_mut(id)
    }

    /// Remove a material from the library
    pub fn remove_material(&mut self, id: &MaterialId) -> Option<Material> {
        self.materials.remove(id)
    }

    /// Get all materials
    pub fn get_all_materials(&self) -> Vec<&Material> {
        self.materials.values().collect()
    }

    /// Get all materials in a specific category
    pub fn get_materials_by_category(&self, category: MaterialCategory) -> Vec<&Material> {
        self.materials
            .values()
            .filter(|m| m.category == category)
            .collect()
    }

    /// Search materials by name (partial match, case-insensitive)
    pub fn search_by_name(&self, query: &str) -> Vec<&Material> {
        let query_lower = query.to_lowercase();
        self.materials
            .values()
            .filter(|m| {
                m.name.to_lowercase().contains(&query_lower)
                    || m.subcategory.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Get the number of materials in the library
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Check if library is empty
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

impl Default for MaterialLibrary {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the standard materials library with common materials
pub fn init_standard_library() -> MaterialLibrary {
    let mut library = MaterialLibrary::new();

    // Red Oak (hardwood)
    let mut red_oak = Material::new(
        MaterialId("wood_oak_red".to_string()),
        "Red Oak".to_string(),
        MaterialCategory::Wood,
        "Hardwood".to_string(),
    );
    red_oak.description = "Dense American hardwood, good for general CNC work".to_string();
    red_oak.density = 705.0;
    red_oak.machinability_rating = 8;
    red_oak.tensile_strength = Some(104.0);
    red_oak.surface_finish = SurfaceFinishability::Good;
    red_oak.notes = "Good grain structure, moderate dulling of tools\nDensity (12% MC): ~705 kg/m³ https://amesweb.info/Materials/Density-of-Wood.aspx\nTensile strength (parallel to grain): ~104 MPa (US FPL Wood Handbook, ch5) https://www.fpl.fs.usda.gov/documnts/fplgtr/fplgtr190/chapter_05.pdf".to_string();

    let oak_params = CuttingParameters {
        rpm_range: (16000, 20000),
        feed_rate_range: (1200.0, 2000.0),
        max_doc: 6.0,
        stepover_percent: (40.0, 60.0),
        ..Default::default()
    };
    red_oak.set_cutting_params("endmill_flat".to_string(), oak_params);

    library.add_material(red_oak);

    // Acrylic
    let mut acrylic = Material::new(
        MaterialId("plastic_acrylic".to_string()),
        "Acrylic".to_string(),
        MaterialCategory::Plastic,
        "PMMA".to_string(),
    );
    acrylic.description = "Clear plastic, good for engraving and cutting".to_string();
    acrylic.density = 1190.0;
    acrylic.machinability_rating = 9;
    acrylic.tensile_strength = Some(70.0);
    acrylic.melting_point = Some(105.0);
    acrylic.surface_finish = SurfaceFinishability::Excellent;
    acrylic.heat_sensitivity = HeatSensitivity::High;
    acrylic.notes = "Keep tool speed high and feed moderate to avoid heat buildup.\nPMMA Tg (used for melting_point field): ~105 °C; tensile strength: ~70 MPa. Sources: https://polymers.netzsch.com/Materials/Details/29 ; https://matmake.com/materials-data/polymethyl-methacrylate-properties.html".to_string();

    let acrylic_params = CuttingParameters {
        rpm_range: (18000, 24000),
        feed_rate_range: (1000.0, 1800.0),
        max_doc: 3.0,
        coolant_type: CoolantType::AirOnly,
        ..Default::default()
    };
    acrylic.set_cutting_params("endmill_flat".to_string(), acrylic_params);

    library.add_material(acrylic);

    // Aluminum 6061
    let mut aluminum_6061 = Material::new(
        MaterialId("metal_al_6061".to_string()),
        "Aluminum 6061".to_string(),
        MaterialCategory::NonFerrousMetal,
        "Aluminum alloy".to_string(),
    );
    aluminum_6061.description = "General-purpose aluminum alloy (6061-T6)".to_string();
    aluminum_6061.density = 2700.0;
    aluminum_6061.machinability_rating = 8;
    aluminum_6061.tensile_strength = Some(310.0);
    aluminum_6061.melting_point = Some(582.0);
    aluminum_6061.coolant_required = true;
    aluminum_6061.required_ppe = vec![PPE::EyeProtection, PPE::HearingProtection];
    aluminum_6061.notes = "6061-T6: density ~2700 kg/m³; UTS ~310 MPa; melting range ~582–652 °C (solidus–liquidus). Source: https://asm.matweb.com/search/specificmaterial.asp?bassnum=ma6061t6".to_string();

    let aluminum_params = CuttingParameters {
        rpm_range: (8000, 12000),
        feed_rate_range: (900.0, 2200.0),
        plunge_rate_percent: 40.0,
        max_doc: 3.0,
        stepover_percent: (35.0, 65.0),
        surface_speed_m_min: Some(300.0),
        chip_load_mm: Some(0.05),
        sfm_range: None,
        chipload_range: None,
        power_factor: None,
        ramp_angle_max: None,
        adaptive_stepover_max: None,
        coolant_type: CoolantType::WaterSoluble,
        notes: "12k spindle baseline (assumes ~6mm, 2-flute carbide endmill); adjust by tool diameter using surface speed + chip load. Sources: https://www.machiningdoctor.com/mds/?matId=3850 ; https://www.harveytool.com/resources/general-machining-guidelines".to_string(),
    };
    aluminum_6061.set_cutting_params("endmill_flat".to_string(), aluminum_params);

    library.add_material(aluminum_6061);

    // MPI-derived materials (static)
    for m in crate::data::materials_mpi_static::load_mpi_derived_materials() {
        library.add_material(m);
    }

    library
}
