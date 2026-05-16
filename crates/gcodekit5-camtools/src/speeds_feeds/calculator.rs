//! Speeds and Feeds Calculator - Enhanced Calculation Engine
//!
//! Calculates spindle speeds and feed rates based on material properties,
//! tool geometry, and machine capabilities with advanced formulas and adjustments.
//!
//! # Formulas
//! - RPM = (Surface Speed * 1000) / (π * Diameter) * adjustments
//! - Feed Rate = RPM * Chip Load * Flutes * adjustments
//! - Power = (Specific Cutting Force * Chip Area * Cutting Speed) / 60000
//! - Deflection = (Force * Stick_out³) / (3 * E * I)

use gcodekit5_core::data::materials::{Material, MaterialCategory};
use gcodekit5_core::data::tools::{Tool, ToolCoating, ToolGeometry, ToolMaterial};
use gcodekit5_devicedb::model::DeviceProfile;
use std::f32::consts::PI;

/// Type of machining operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    /// Standard slotting (full width cut)
    Slotting,
    /// Pocketing operation (moderate engagement)
    Pocketing,
    /// Profile/side milling (low engagement)
    Profiling,
    /// Adaptive/high-speed machining
    Adaptive,
    /// Drilling operation
    Drilling,
    /// Plunging/ramping
    Plunging,
}

impl OperationType {
    /// Get the recommended radial engagement percentage for this operation
    pub fn radial_engagement_percent(self) -> f32 {
        match self {
            Self::Slotting => 100.0,
            Self::Pocketing => 50.0,
            Self::Profiling => 30.0,
            Self::Adaptive => 20.0,
            Self::Drilling => 100.0,
            Self::Plunging => 100.0,
        }
    }

    /// Get the axial engagement factor (multiplier for depth of cut)
    pub fn axial_engagement_factor(self) -> f32 {
        match self {
            Self::Slotting => 1.0,
            Self::Pocketing => 1.2,
            Self::Profiling => 1.5,
            Self::Adaptive => 1.8,
            Self::Drilling => 0.5,
            Self::Plunging => 0.3,
        }
    }

    /// Get the feed rate multiplier for this operation
    pub fn feed_rate_multiplier(self) -> f32 {
        match self {
            Self::Slotting => 1.0,
            Self::Pocketing => 1.1,
            Self::Profiling => 1.2,
            Self::Adaptive => 1.5,
            Self::Drilling => 0.8,
            Self::Plunging => 0.5,
        }
    }
}

/// Input parameters for calculation
#[derive(Debug, Clone)]
pub struct CalculationInput {
    /// Selected material
    pub material: Material,
    /// Selected tool
    pub tool: Tool,
    /// Active device/machine
    pub device: DeviceProfile,
    /// Type of operation
    pub operation: OperationType,
    /// Depth of cut in mm
    pub depth_of_cut: f32,
    /// Width of cut/stepover in mm (None = use operation default)
    pub width_of_cut: Option<f32>,
    /// Tool stick-out length in mm
    pub tool_stick_out: f32,
    /// Coolant enabled
    pub coolant_enabled: bool,
    /// Conservative calculation (reduces values by safety factor)
    pub conservative: bool,
}

impl Default for CalculationInput {
    fn default() -> Self {
        Self {
            material: Material::new(
                gcodekit5_core::data::materials::MaterialId("default".to_string()),
                "Default".to_string(),
                MaterialCategory::NonFerrousMetal,
                "Default".to_string(),
            ),
            tool: Tool::new(
                gcodekit5_core::data::tools::ToolId("default".to_string()),
                1,
                "Default".to_string(),
                gcodekit5_core::data::tools::ToolType::EndMillFlat,
                6.0,
                50.0,
            ),
            device: DeviceProfile::default(),
            operation: OperationType::Slotting,
            depth_of_cut: 3.0,
            width_of_cut: None,
            tool_stick_out: 20.0,
            coolant_enabled: true,
            conservative: false,
        }
    }
}

/// Detailed calculation result with all computed values
#[derive(Debug, Clone)]
pub struct CalculationResult {
    /// Calculated Spindle Speed in RPM
    pub rpm: u32,
    /// Calculated Feed Rate in mm/min
    pub feed_rate: f32,
    /// Surface Speed used (m/min)
    pub surface_speed_m_min: f32,
    /// Surface Speed (SFM - Surface Feet per Minute)
    pub surface_speed_sfm: f32,
    /// Chip Load per tooth (mm/tooth)
    pub chip_load_mm: f32,
    /// Chip Load per tooth (inch/tooth)
    pub chip_load_inch: f32,
    /// Material Removal Rate (mm³/min)
    pub material_removal_rate: f32,
    /// Estimated spindle power required (kW)
    pub power_required_kw: f32,
    /// Estimated tool deflection (mm)
    pub estimated_deflection_mm: f32,
    /// Radial engagement percentage
    pub radial_engagement_percent: f32,
    /// Axial engagement (depth of cut)
    pub axial_engagement_mm: f32,
    /// Source/description of calculation data
    pub source: String,
    /// Warnings generated during calculation
    pub warnings: Vec<String>,
    /// Unclamped RPM if clamping occurred
    pub unclamped_rpm: Option<u32>,
    /// Unclamped Feed Rate if clamping occurred
    pub unclamped_feed_rate: Option<f32>,
}

/// Material-specific speed factor calculator
pub struct MaterialSpeedFactors;

impl MaterialSpeedFactors {
    /// Get SFM adjustment factor for tool material vs workpiece material
    pub fn get_tool_material_factor(
        tool_material: ToolMaterial,
        workpiece_category: MaterialCategory,
    ) -> f32 {
        match (tool_material, workpiece_category) {
            // HSS tools - conservative speeds
            (ToolMaterial::HSS, MaterialCategory::Wood) => 1.0,
            (ToolMaterial::HSS, MaterialCategory::Plastic) => 0.9,
            (ToolMaterial::HSS, MaterialCategory::NonFerrousMetal) => 0.6,
            (ToolMaterial::HSS, MaterialCategory::FerrousMetal) => 0.5,
            (ToolMaterial::HSS, MaterialCategory::Composite) => 0.4,
            (ToolMaterial::HSS, MaterialCategory::StoneAndCeramic) => 0.3,
            (ToolMaterial::HSS, MaterialCategory::EngineeredWood) => 0.9,

            // Carbide tools - standard speeds
            (ToolMaterial::Carbide, MaterialCategory::Wood) => 2.5,
            (ToolMaterial::Carbide, MaterialCategory::Plastic) => 2.0,
            (ToolMaterial::Carbide, MaterialCategory::NonFerrousMetal) => 1.7,
            (ToolMaterial::Carbide, MaterialCategory::FerrousMetal) => 1.5,
            (ToolMaterial::Carbide, MaterialCategory::Composite) => 1.2,
            (ToolMaterial::Carbide, MaterialCategory::StoneAndCeramic) => 0.8,
            (ToolMaterial::Carbide, MaterialCategory::EngineeredWood) => 2.3,

            // Coated carbide - slightly higher speeds
            (ToolMaterial::CoatedCarbide, MaterialCategory::Wood) => 3.0,
            (ToolMaterial::CoatedCarbide, MaterialCategory::Plastic) => 2.3,
            (ToolMaterial::CoatedCarbide, MaterialCategory::NonFerrousMetal) => 2.0,
            (ToolMaterial::CoatedCarbide, MaterialCategory::FerrousMetal) => 1.8,
            (ToolMaterial::CoatedCarbide, MaterialCategory::Composite) => 1.5,
            (ToolMaterial::CoatedCarbide, MaterialCategory::StoneAndCeramic) => 1.0,
            (ToolMaterial::CoatedCarbide, MaterialCategory::EngineeredWood) => 2.7,

            // Diamond tools - highest speeds, but limited by material
            (ToolMaterial::Diamond, MaterialCategory::Wood) => 3.5,
            (ToolMaterial::Diamond, MaterialCategory::Plastic) => 3.0,
            (ToolMaterial::Diamond, MaterialCategory::NonFerrousMetal) => 2.5,
            (ToolMaterial::Diamond, MaterialCategory::FerrousMetal) => 2.0,
            (ToolMaterial::Diamond, MaterialCategory::Composite) => 2.5,
            (ToolMaterial::Diamond, MaterialCategory::StoneAndCeramic) => 1.5,
            (ToolMaterial::Diamond, MaterialCategory::EngineeredWood) => 3.2,
        }
    }

    /// Get coating adjustment factor
    pub fn get_coating_factor(coating: Option<ToolCoating>) -> f32 {
        match coating {
            None => 1.0,
            Some(ToolCoating::TiN) => 1.1,
            Some(ToolCoating::TiAlN) => 1.25,
            Some(ToolCoating::DLC) => 1.3,
            Some(ToolCoating::AlOx) => 1.15,
        }
    }
}

/// Tool geometry adjustment calculator
pub struct GeometryAdjustments;

impl GeometryAdjustments {
    /// Calculate helix angle adjustment factor
    /// Higher helix angles allow slightly higher feeds
    pub fn helix_factor(geometry: &ToolGeometry) -> f32 {
        match geometry.helix_angle {
            Some(angle) if angle > 45.0 => 1.1,
            Some(angle) if angle > 35.0 => 1.05,
            Some(angle) if angle < 20.0 => 0.9,
            _ => 1.0,
        }
    }

    /// Calculate radial engagement adjustment
    /// Lower engagement allows higher chip loads (chip thinning effect)
    pub fn radial_engagement_factor(radial_engagement_percent: f32) -> f32 {
        if radial_engagement_percent <= 25.0 {
            1.3 // Significant chip thinning compensation
        } else if radial_engagement_percent <= 50.0 {
            1.15 // Moderate compensation
        } else if radial_engagement_percent <= 75.0 {
            0.95 // Slight reduction
        } else {
            0.85 // Full slotting is most demanding
        }
    }

    /// Calculate axial engagement adjustment
    pub fn axial_engagement_factor(axial_engagement_percent: f32) -> f32 {
        if axial_engagement_percent <= 0.5 {
            1.1 // Low axial engagement
        } else if axial_engagement_percent <= 1.0 {
            1.0 // Standard
        } else if axial_engagement_percent <= 1.5 {
            0.9 // High engagement
        } else {
            0.8 // Very high engagement
        }
    }
}

/// Enhanced calculator for speeds and feeds
pub struct SpeedsFeedsCalculator;

impl SpeedsFeedsCalculator {
    /// Calculate speeds and feeds with all advanced adjustments
    pub fn calculate(input: &CalculationInput) -> CalculationResult {
        let mut warnings = Vec::new();
        let mut source_parts = Vec::new();

        let tool_type_key = Self::get_tool_type_key(&input.tool);
        let material_params = input.material.get_cutting_params(&tool_type_key);

        // 1. Calculate Surface Speed (m/min)
        let (surface_speed_m_min, surface_speed_source) =
            Self::calculate_surface_speed(input, material_params);
        source_parts.push(surface_speed_source);

        // 2. Calculate Base RPM
        let base_rpm = Self::rpm_from_surface_speed(surface_speed_m_min, input.tool.diameter);

        // 3. Apply RPM Adjustments
        let adjusted_rpm = Self::adjust_rpm_for_conditions(base_rpm, input);

        // 4. Calculate Chip Load
        let (base_chip_load, chip_load_source) =
            Self::calculate_base_chip_load(input, material_params, adjusted_rpm);
        source_parts.push(chip_load_source);

        // 5. Apply Chip Load Adjustments
        let adjusted_chip_load = Self::adjust_chip_load_for_conditions(base_chip_load, input);

        // 6. Calculate Feed Rate
        let base_feed_rate = adjusted_rpm * adjusted_chip_load * input.tool.flutes as f32;
        let feed_rate = base_feed_rate * input.operation.feed_rate_multiplier();

        // 7. Calculate additional outputs
        let radial_engagement = input.width_of_cut.unwrap_or_else(|| {
            input.tool.diameter * input.operation.radial_engagement_percent() / 100.0
        });

        let mrr = Self::calculate_mrr(feed_rate, radial_engagement, input.depth_of_cut);
        let power_kw =
            Self::estimate_power(input, feed_rate, radial_engagement, input.depth_of_cut);
        let deflection = Self::estimate_deflection(input, power_kw);

        // 8. Apply Machine Limits and Generate Warnings
        let (final_rpm, final_feed_rate, clamped_rpm, clamped_feed) =
            Self::apply_machine_limits(adjusted_rpm, feed_rate, input, &mut warnings);

        // 9. Generate Sources String
        let source = source_parts.join(" + ");

        CalculationResult {
            rpm: final_rpm,
            feed_rate: final_feed_rate,
            surface_speed_m_min,
            surface_speed_sfm: surface_speed_m_min * 3.28084,
            chip_load_mm: adjusted_chip_load,
            chip_load_inch: adjusted_chip_load / 25.4,
            material_removal_rate: mrr,
            power_required_kw: power_kw,
            estimated_deflection_mm: deflection,
            radial_engagement_percent: (radial_engagement / input.tool.diameter) * 100.0,
            axial_engagement_mm: input.depth_of_cut,
            source,
            warnings,
            unclamped_rpm: clamped_rpm,
            unclamped_feed_rate: clamped_feed,
        }
    }

    /// Legacy calculate method for backward compatibility
    pub fn calculate_legacy(
        material: &Material,
        tool: &Tool,
        device: &DeviceProfile,
    ) -> CalculationResult {
        let input = CalculationInput {
            material: material.clone(),
            tool: tool.clone(),
            device: device.clone(),
            operation: OperationType::Slotting,
            depth_of_cut: tool.params.depth_per_pass,
            width_of_cut: Some(tool.diameter * 0.5),
            tool_stick_out: tool.limits.stick_out_max.unwrap_or(20.0),
            coolant_enabled: true,
            conservative: false,
        };
        Self::calculate(&input)
    }

    /// Get tool type key string for parameter lookup
    fn get_tool_type_key(tool: &Tool) -> String {
        use gcodekit5_core::data::tools::ToolType;
        match tool.tool_type {
            ToolType::EndMillFlat => "endmill_flat".to_string(),
            ToolType::EndMillBall => "endmill_ball".to_string(),
            ToolType::EndMillCornerRadius => "endmill_corner".to_string(),
            ToolType::VBit => "vbit".to_string(),
            ToolType::DrillBit => "drill".to_string(),
            ToolType::SpotDrill => "spot_drill".to_string(),
            ToolType::EngravingBit => "engraving".to_string(),
            ToolType::ChamferTool => "chamfer".to_string(),
            ToolType::Specialty => "specialty".to_string(),
        }
    }

    /// Calculate surface speed in m/min
    fn calculate_surface_speed(
        input: &CalculationInput,
        material_params: Option<&gcodekit5_core::data::materials::CuttingParameters>,
    ) -> (f32, String) {
        // Priority 1: Direct material surface speed
        if let Some(params) = material_params {
            if let Some(speed) = params.surface_speed_m_min {
                return (speed, "Material Surface Speed".to_string());
            }

            // Priority 2: Estimate from SFM range
            if let Some((min_sfm, max_sfm)) = params.sfm_range {
                let avg_sfm = (min_sfm + max_sfm) / 2.0;
                let speed_m_min = avg_sfm * 0.3048; // Convert SFM to m/min
                return (speed_m_min, "Material SFM Range".to_string());
            }

            // Priority 3: Estimate from RPM range
            let avg_rpm = (params.rpm_range.0 + params.rpm_range.1) as f32 / 2.0;
            let speed = (avg_rpm * PI * input.tool.diameter) / 1000.0;
            return (speed, "Estimated from Material RPM".to_string());
        }

        // Fallback: Calculate from tool defaults
        let rpm = input.tool.params.rpm as f32;
        let speed = (rpm * PI * input.tool.diameter) / 1000.0;
        (speed, "Tool Defaults".to_string())
    }

    /// Calculate RPM from surface speed
    fn rpm_from_surface_speed(surface_speed_m_min: f32, diameter_mm: f32) -> f32 {
        (surface_speed_m_min * 1000.0) / (PI * diameter_mm)
    }

    /// Apply RPM adjustments for various conditions
    fn adjust_rpm_for_conditions(base_rpm: f32, input: &CalculationInput) -> f32 {
        let mut adjusted = base_rpm;

        // Tool material factor
        let tool_factor = MaterialSpeedFactors::get_tool_material_factor(
            input.tool.material,
            input.material.category,
        );
        adjusted *= tool_factor;

        // Coating factor
        let coating_factor = MaterialSpeedFactors::get_coating_factor(input.tool.coating);
        adjusted *= coating_factor;

        // Conservative mode
        if input.conservative {
            adjusted *= 0.8;
        }

        adjusted
    }

    /// Calculate base chip load
    fn calculate_base_chip_load(
        input: &CalculationInput,
        material_params: Option<&gcodekit5_core::data::materials::CuttingParameters>,
        rpm: f32,
    ) -> (f32, String) {
        if let Some(params) = material_params {
            // Priority 1: Direct chip load
            if let Some(load) = params.chip_load_mm {
                return (load, "Material Chip Load".to_string());
            }

            // Priority 2: From chipload range
            if let Some((min_load, max_load)) = params.chipload_range {
                let avg_load = (min_load + max_load) / 2.0;
                return (avg_load, "Material Chipload Range".to_string());
            }

            // Priority 3: From feed rate range
            let avg_feed = (params.feed_rate_range.0 + params.feed_rate_range.1) / 2.0;
            let load = avg_feed / (rpm * input.tool.flutes as f32);
            return (load, "Estimated from Feed Range".to_string());
        }

        // Fallback: From tool defaults
        let load =
            input.tool.params.feed_rate / (input.tool.params.rpm as f32 * input.tool.flutes as f32);
        (load.max(0.01), "Tool Defaults".to_string())
    }

    /// Adjust chip load for various conditions
    fn adjust_chip_load_for_conditions(base_load: f32, input: &CalculationInput) -> f32 {
        let mut adjusted = base_load;

        // Geometry adjustments
        adjusted *= GeometryAdjustments::helix_factor(&input.tool.geometry);

        // Radial engagement adjustment (chip thinning)
        let radial_pct = input.operation.radial_engagement_percent();
        adjusted *= GeometryAdjustments::radial_engagement_factor(radial_pct);

        // Axial engagement adjustment
        let axial_pct = input.depth_of_cut / input.tool.diameter;
        adjusted *= GeometryAdjustments::axial_engagement_factor(axial_pct);

        // Conservative mode
        if input.conservative {
            adjusted *= 0.85;
        }

        // Coolant adjustment
        if !input.coolant_enabled {
            adjusted *= 0.9;
        }

        // Check against tool limits
        if let Some(max_load) = input.tool.limits.max_chip_load {
            adjusted = adjusted.min(max_load);
        }

        adjusted
    }

    /// Calculate Material Removal Rate (mm³/min)
    fn calculate_mrr(feed_rate: f32, width_of_cut: f32, depth_of_cut: f32) -> f32 {
        feed_rate * width_of_cut * depth_of_cut
    }

    /// Estimate power requirement (kW)
    fn estimate_power(
        input: &CalculationInput,
        feed_rate: f32,
        width_of_cut: f32,
        depth_of_cut: f32,
    ) -> f32 {
        // Standard formula: P = (Fc × V) / 60000 (kW)
        // Where:
        //   Fc = cutting force (Newtons)
        //   V = cutting speed (m/min)

        // Get specific cutting force (Kc) from material - typical values:
        // Wood: 10-30 MPa, Aluminum: 100-300 MPa, Steel: 1000-3000 MPa
        let kc = input.material.specific_cutting_force.unwrap_or({
            match input.material.category {
                MaterialCategory::Wood => 20.0,
                MaterialCategory::Plastic => 30.0,
                MaterialCategory::NonFerrousMetal => 200.0, // Lowered from 700 for aluminum
                MaterialCategory::FerrousMetal => 1500.0,
                MaterialCategory::Composite => 500.0,
                MaterialCategory::StoneAndCeramic => 2000.0,
                MaterialCategory::EngineeredWood => 25.0,
            }
        });

        // Chip cross-sectional area (mm²)
        // feed_per_tooth (mm) = feed_rate (mm/min) / (rpm × flutes)
        // chip_area = depth_of_cut × feed_per_tooth
        let feed_per_tooth =
            feed_rate / (input.tool.params.rpm.max(1) as f32 * input.tool.flutes as f32);
        let chip_area = depth_of_cut * feed_per_tooth;

        // Cutting force (Newtons) = Kc (MPa = N/mm²) × Area (mm²)
        let cutting_force = kc * chip_area * (width_of_cut / input.tool.diameter).max(0.1);

        // Cutting speed (m/min) = π × diameter(mm) × rpm / 1000
        let cutting_speed = (PI * input.tool.diameter * input.tool.params.rpm as f32) / 1000.0;

        // Power at tool tip (kW) = (Force(N) × Speed(m/min)) / 60000
        let power_at_tool = (cutting_force * cutting_speed) / 60000.0;

        // Apply efficiency factor (typically 0.7-0.9)
        power_at_tool / 0.85 // Account for spindle efficiency
    }
    /// Estimate tool deflection (mm)
    fn estimate_deflection(input: &CalculationInput, power_kw: f32) -> f32 {
        // Deflection formula: δ = (F * L³) / (3 * E * I)
        // Returns deflection in millimeters (mm)
        //
        // Where:
        //   F = cutting force (Newtons)
        //   L = stick-out length (meters)
        //   E = modulus of elasticity (Pascals = N/m²)
        //   I = area moment of inertia (m⁴)

        // Estimate cutting force from power and RPM
        // Power (W) = Torque (Nm) * Angular Velocity (rad/s)
        // Torque = Power / (2π * RPM / 60)
        let rpm = input.tool.params.rpm.max(1) as f32;
        let torque_nm = (power_kw * 1000.0) / (2.0 * PI * rpm / 60.0);

        // Force at tool tip = Torque / Radius
        let radius_m = input.tool.diameter / 2000.0; // diameter/2 in meters
        let force_n = if radius_m > 0.0 {
            torque_nm / radius_m
        } else {
            0.0
        };

        // Modulus of elasticity for tool material (in Pascals, not MPa!)
        // 1 MPa = 1,000,000 Pa
        let e_modulus_pa = match input.tool.material {
            ToolMaterial::HSS => 210_000.0 * 1_000_000.0, // 210 GPa = 210,000 MPa
            ToolMaterial::Carbide => 650_000.0 * 1_000_000.0, // 650 GPa = 650,000 MPa
            ToolMaterial::CoatedCarbide => 650_000.0 * 1_000_000.0,
            ToolMaterial::Diamond => 1_050_000.0 * 1_000_000.0, // 1050 GPa
        };

        // Moment of inertia for circular cross-section: I = (π * d⁴) / 64
        // d must be in meters
        let diameter_m = input.tool.diameter / 1000.0;
        let i_m4 = (PI * diameter_m.powi(4)) / 64.0;

        // Stick-out in meters
        let stick_out_m = input.tool_stick_out / 1000.0;

        // Deflection calculation in meters
        let deflection_m = if i_m4 > 0.0 && e_modulus_pa > 0.0 {
            (force_n * stick_out_m.powi(3)) / (3.0 * e_modulus_pa * i_m4)
        } else {
            0.0
        };

        // Convert to millimeters
        let deflection_mm = deflection_m * 1000.0;

        // Sanity check - if result is unreasonable, return 0
        if deflection_mm.is_infinite() || deflection_mm.is_nan() || deflection_mm < 0.0 {
            0.0
        } else {
            deflection_mm
        }
    }

    /// Apply machine limits and generate warnings
    fn apply_machine_limits(
        rpm: f32,
        feed_rate: f32,
        input: &CalculationInput,
        warnings: &mut Vec<String>,
    ) -> (u32, f32, Option<u32>, Option<f32>) {
        let mut final_rpm = rpm as u32;
        let mut final_feed = feed_rate;
        let mut unclamped_rpm = None;
        let mut unclamped_feed = None;

        // RPM limits
        let min_rpm = 1000u32;
        let max_rpm = input.device.max_spindle_speed_rpm;

        if final_rpm > max_rpm {
            warnings.push(format!(
                "RPM {} exceeds machine maximum {}, clamped",
                final_rpm, max_rpm
            ));
            unclamped_rpm = Some(final_rpm);
            final_rpm = max_rpm;
        }

        if final_rpm < min_rpm {
            warnings.push(format!(
                "RPM {} below minimum {}, raised to {}",
                final_rpm, min_rpm, min_rpm
            ));
            final_rpm = min_rpm;
        }

        // Tool RPM limit
        if let Some(tool_max_rpm) = input.tool.limits.max_rpm_rating {
            if final_rpm > tool_max_rpm {
                warnings.push(format!(
                    "RPM {} exceeds tool rating {}, clamped",
                    final_rpm, tool_max_rpm
                ));
                if unclamped_rpm.is_none() {
                    unclamped_rpm = Some(final_rpm);
                }
                final_rpm = tool_max_rpm;
            }
        }

        // Feed rate limits
        let max_feed = input.device.max_feed_rate as f32;
        if final_feed > max_feed {
            warnings.push(format!(
                "Feed rate {:.1} exceeds machine max {:.1}, clamped",
                final_feed, max_feed
            ));
            unclamped_feed = Some(final_feed);
            final_feed = max_feed;
        }

        if final_feed < 50.0 {
            warnings.push(format!(
                "Feed rate {:.1} very low, may cause rubbing",
                final_feed
            ));
        }

        (final_rpm, final_feed, unclamped_rpm, unclamped_feed)
    }
}

/// Convenience function for quick calculation
pub fn calculate_speeds_feeds(
    material: &Material,
    tool: &Tool,
    device: &DeviceProfile,
    operation: OperationType,
) -> CalculationResult {
    let input = CalculationInput {
        material: material.clone(),
        tool: tool.clone(),
        device: device.clone(),
        operation,
        depth_of_cut: tool.params.depth_per_pass,
        width_of_cut: None,
        tool_stick_out: tool.limits.stick_out_max.unwrap_or(20.0),
        coolant_enabled: true,
        conservative: false,
    };
    SpeedsFeedsCalculator::calculate(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_type_multipliers() {
        assert!(
            OperationType::Adaptive.feed_rate_multiplier()
                > OperationType::Slotting.feed_rate_multiplier()
        );
        assert_eq!(OperationType::Slotting.radial_engagement_percent(), 100.0);
        assert_eq!(OperationType::Profiling.radial_engagement_percent(), 30.0);
    }

    #[test]
    fn test_geometry_adjustments() {
        let high_helix = ToolGeometry {
            helix_angle: Some(50.0),
            ..Default::default()
        };
        assert!(GeometryAdjustments::helix_factor(&high_helix) > 1.0);

        let low_engagement = GeometryAdjustments::radial_engagement_factor(20.0);
        let high_engagement = GeometryAdjustments::radial_engagement_factor(100.0);
        assert!(low_engagement > high_engagement);
    }

    #[test]
    fn test_rpm_calculation() {
        let rpm = SpeedsFeedsCalculator::rpm_from_surface_speed(200.0, 6.0);
        // RPM = (200 * 1000) / (π * 6) ≈ 10,610
        assert!(rpm > 10000.0);
        assert!(rpm < 11000.0);
    }
}
