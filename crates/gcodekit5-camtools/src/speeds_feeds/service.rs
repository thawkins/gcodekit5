//! Speeds and Feeds Calculator - Database Integration Services
//!
//! Provides service layers for integrating the feeds and speeds calculator
//! with the Tools, Materials, and Devices databases.

use gcodekit5_core::data::materials::{Material, MaterialCategory, MaterialId, MaterialLibrary};
use gcodekit5_core::data::tools::{Tool, ToolId, ToolLibrary, ToolType};
use gcodekit5_devicedb::model::DeviceProfile;

/// Service for querying and filtering tools from the Tools Manager
#[derive(Debug)]
pub struct SpeedsFeedsToolService {
    /// Filter: Minimum tool diameter (mm)
    pub min_diameter: Option<f32>,
    /// Filter: Maximum tool diameter (mm)
    pub max_diameter: Option<f32>,
    /// Filter: Specific tool types
    pub tool_types: Vec<ToolType>,
    /// Filter: Tool material compatibility with workpiece
    pub compatible_material: Option<MaterialCategory>,
}

impl Default for SpeedsFeedsToolService {
    fn default() -> Self {
        Self {
            min_diameter: None,
            max_diameter: None,
            tool_types: vec![
                ToolType::EndMillFlat,
                ToolType::EndMillBall,
                ToolType::EndMillCornerRadius,
                ToolType::VBit,
                ToolType::DrillBit,
            ],
            compatible_material: None,
        }
    }
}

impl SpeedsFeedsToolService {
    /// Create a new tool service with default filters
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum diameter filter
    pub fn with_min_diameter(mut self, diameter: f32) -> Self {
        self.min_diameter = Some(diameter);
        self
    }

    /// Set maximum diameter filter
    pub fn with_max_diameter(mut self, diameter: f32) -> Self {
        self.max_diameter = Some(diameter);
        self
    }

    /// Set diameter range filter
    pub fn with_diameter_range(mut self, min: f32, max: f32) -> Self {
        self.min_diameter = Some(min);
        self.max_diameter = Some(max);
        self
    }

    /// Set allowed tool types
    pub fn with_tool_types(mut self, types: Vec<ToolType>) -> Self {
        self.tool_types = types;
        self
    }

    /// Set compatible material filter
    pub fn with_compatible_material(mut self, material: MaterialCategory) -> Self {
        self.compatible_material = Some(material);
        self
    }

    /// Find tools matching current filters
    pub fn find_tools<'a>(&self, library: &'a ToolLibrary) -> Vec<&'a Tool> {
        library
            .get_all_tools()
            .into_iter()
            .filter(|tool| {
                // Filter by diameter
                if let Some(min) = self.min_diameter {
                    if tool.diameter < min {
                        return false;
                    }
                }
                if let Some(max) = self.max_diameter {
                    if tool.diameter > max {
                        return false;
                    }
                }

                // Filter by tool type
                if !self.tool_types.is_empty() && !self.tool_types.contains(&tool.tool_type) {
                    return false;
                }

                // Filter by material compatibility (if specified)
                if let Some(ref material) = self.compatible_material {
                    if !self.is_compatible(tool, material) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Get a specific tool by ID
    pub fn get_tool<'a>(&self, library: &'a ToolLibrary, id: &ToolId) -> Option<&'a Tool> {
        library.get_tool(id)
    }

    /// Check if a tool is compatible with a material
    fn is_compatible(&self, tool: &Tool, material: &MaterialCategory) -> bool {
        // Most tools work with most materials, but there are exceptions
        match (tool.tool_type, material) {
            (ToolType::VBit, MaterialCategory::FerrousMetal) => {
                // V-bits are typically for softer materials
                false
            }
            (ToolType::VBit, MaterialCategory::StoneAndCeramic) => false,
            (ToolType::DrillBit, MaterialCategory::StoneAndCeramic) => {
                // Standard drills don't work on stone/ceramic
                false
            }
            _ => true,
        }
    }

    /// Get recommended tools for a specific material and operation
    pub fn get_recommended_tools<'a>(
        &self,
        library: &'a ToolLibrary,
        material: &MaterialCategory,
        roughing: bool,
    ) -> Vec<&'a Tool> {
        let mut tools = self.find_tools(library);

        // Sort by appropriateness
        tools.sort_by(|a, b| {
            let score_a = self.tool_score(a, material, roughing);
            let score_b = self.tool_score(b, material, roughing);
            score_b.partial_cmp(&score_a).unwrap() // Higher score first
        });

        tools
    }

    /// Calculate a score for how appropriate a tool is
    fn tool_score(&self, tool: &Tool, material: &MaterialCategory, roughing: bool) -> f32 {
        let mut score = 100.0;

        // Material compatibility
        match (tool.material, material) {
            (gcodekit5_core::data::tools::ToolMaterial::HSS, MaterialCategory::StoneAndCeramic) => {
                score -= 50.0;
            }
            (gcodekit5_core::data::tools::ToolMaterial::Carbide, _) => {
                score += 20.0;
            }
            (gcodekit5_core::data::tools::ToolMaterial::CoatedCarbide, _) => {
                score += 30.0;
            }
            _ => {}
        }

        // Tool type appropriateness for operation
        match (tool.tool_type, roughing) {
            (ToolType::EndMillFlat, true) => score += 20.0, // Good for roughing
            (ToolType::EndMillFlat, false) => score += 15.0, // Good for finishing too
            (ToolType::EndMillBall, false) => score += 10.0, // Good for finishing
            (ToolType::EndMillCornerRadius, _) => score += 25.0, // Versatile
            (ToolType::EndMillBall, true) => score -= 5.0,  // Not ideal for heavy roughing
            _ => {}
        }

        // Flute count for operation
        if roughing {
            // More flutes better for roughing
            if tool.flutes >= 3 {
                score += 10.0;
            }
        } else {
            // Fewer flutes better for chip evacuation in finishing
            if tool.flutes <= 2 {
                score += 10.0;
            }
        }

        // Coating bonus
        if tool.coating.is_some() {
            score += 10.0;
        }

        score
    }

    /// Validate a tool for the given parameters
    pub fn validate_tool(
        &self,
        tool: &Tool,
        material: &MaterialCategory,
        depth: f32,
    ) -> Result<(), String> {
        let mut errors = Vec::new();

        // Check depth of cut limit
        let max_doc = tool.diameter * tool.params.max_doc_percent / 100.0;
        if depth > max_doc {
            errors.push(format!(
                "Depth of cut {:.2}mm exceeds maximum {:.2}mm for this tool",
                depth, max_doc
            ));
        }

        // Check stick-out (if limits available)
        if let Some(max_stick) = tool.limits.stick_out_max {
            // This would need actual stick-out value from caller
            // For now, just a placeholder
            let _ = max_stick;
        }

        // Check material compatibility
        if !self.is_compatible(tool, material) {
            errors.push(format!(
                "Tool type {:?} not recommended for material {:?}",
                tool.tool_type, material
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}

/// Service for querying and filtering materials from the Materials Database
#[derive(Debug)]
pub struct SpeedsFeedsMaterialService {
    /// Filter: Specific material categories
    pub categories: Vec<MaterialCategory>,
    /// Filter: Minimum machinability rating
    pub min_machinability: Option<u8>,
    /// Filter: Maximum machinability rating
    pub max_machinability: Option<u8>,
    /// Filter: Materials with cutting parameters for specific tool type
    pub has_cutting_params: Option<String>,
}

impl Default for SpeedsFeedsMaterialService {
    fn default() -> Self {
        Self {
            categories: vec![
                MaterialCategory::Wood,
                MaterialCategory::EngineeredWood,
                MaterialCategory::Plastic,
                MaterialCategory::NonFerrousMetal,
                MaterialCategory::FerrousMetal,
                MaterialCategory::Composite,
            ],
            min_machinability: None,
            max_machinability: None,
            has_cutting_params: None,
        }
    }
}

impl SpeedsFeedsMaterialService {
    /// Create a new material service with default filters
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by specific categories
    pub fn with_categories(mut self, categories: Vec<MaterialCategory>) -> Self {
        self.categories = categories;
        self
    }

    /// Filter by machinability range
    pub fn with_machinability_range(mut self, min: u8, max: u8) -> Self {
        self.min_machinability = Some(min);
        self.max_machinability = Some(max);
        self
    }

    /// Only show materials with cutting parameters for a tool type
    pub fn with_cutting_params_for(mut self, tool_type: &str) -> Self {
        self.has_cutting_params = Some(tool_type.to_string());
        self
    }

    /// Find materials matching current filters
    pub fn find_materials<'a>(&self, library: &'a MaterialLibrary) -> Vec<&'a Material> {
        library
            .get_all_materials()
            .into_iter()
            .filter(|material| {
                // Filter by category
                if !self.categories.is_empty() && !self.categories.contains(&material.category) {
                    return false;
                }

                // Filter by machinability
                if let Some(min) = self.min_machinability {
                    if material.machinability_rating < min {
                        return false;
                    }
                }
                if let Some(max) = self.max_machinability {
                    if material.machinability_rating > max {
                        return false;
                    }
                }

                // Filter by cutting params availability
                if let Some(ref tool_type) = self.has_cutting_params {
                    if material.get_cutting_params(tool_type).is_none() {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Get a specific material by ID
    pub fn get_material<'a>(
        &self,
        library: &'a MaterialLibrary,
        id: &MaterialId,
    ) -> Option<&'a Material> {
        library.get_material(id)
    }

    /// Search materials by name (case-insensitive)
    pub fn search_by_name<'a>(
        &self,
        library: &'a MaterialLibrary,
        query: &str,
    ) -> Vec<&'a Material> {
        let query_lower = query.to_lowercase();
        library
            .get_all_materials()
            .into_iter()
            .filter(|material| {
                material.name.to_lowercase().contains(&query_lower)
                    || material.subcategory.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Get cutting parameters for a specific tool type
    pub fn get_cutting_params<'a>(
        &self,
        material: &'a Material,
        tool_type: &str,
    ) -> Option<&'a gcodekit5_core::data::materials::CuttingParameters> {
        material.get_cutting_params(tool_type)
    }

    /// Check if material requires coolant
    pub fn requires_coolant(&self, material: &Material) -> bool {
        material.coolant_required
    }

    /// Get recommended coolant type
    pub fn get_recommended_coolant(&self, material: &Material, tool_type: &str) -> String {
        material
            .get_cutting_params(tool_type)
            .map(|p| format!("{:?}", p.coolant_type))
            .unwrap_or_else(|| {
                // Default based on material category
                match material.category {
                    MaterialCategory::FerrousMetal => "WaterSoluble".to_string(),
                    MaterialCategory::NonFerrousMetal => "WaterSoluble".to_string(),
                    MaterialCategory::Plastic => "AirOnly".to_string(),
                    _ => "None".to_string(),
                }
            })
    }
}

/// Service for querying device/machine capabilities
#[derive(Debug)]
pub struct SpeedsFeedsDeviceService {
    device: DeviceProfile,
}

impl SpeedsFeedsDeviceService {
    /// Create a new device service
    pub fn new(device: DeviceProfile) -> Self {
        Self { device }
    }

    /// Get device limits relevant to feeds and speeds
    pub fn get_limits(&self) -> DeviceLimits {
        DeviceLimits {
            max_rpm: self.device.max_spindle_speed_rpm,
            max_feed_rate: self.device.max_feed_rate as f32,
            max_power_kw: self.device.cnc_spindle_watts as f32 / 1000.0,
            num_axes: self.device.num_axes,
            has_coolant: self.device.has_coolant,
        }
    }

    /// Validate RPM against device limits
    pub fn validate_rpm(&self, rpm: u32) -> Result<u32, String> {
        if rpm > self.device.max_spindle_speed_rpm {
            Err(format!(
                "RPM {} exceeds device maximum {}",
                rpm, self.device.max_spindle_speed_rpm
            ))
        } else if rpm < 1000 {
            Err(format!("RPM {} below minimum 1000", rpm))
        } else {
            Ok(rpm)
        }
    }

    /// Validate feed rate against device limits
    pub fn validate_feed_rate(&self, feed_rate: f32) -> Result<f32, String> {
        if feed_rate > self.device.max_feed_rate as f32 {
            Err(format!(
                "Feed rate {:.1} exceeds device maximum {:.1}",
                feed_rate, self.device.max_feed_rate
            ))
        } else if feed_rate <= 0.0 {
            Err("Feed rate must be positive".to_string())
        } else {
            Ok(feed_rate)
        }
    }

    /// Check if device can handle the calculated power requirement
    pub fn can_handle_power(&self, required_power_kw: f32) -> Result<(), String> {
        let available_power = self.device.cnc_spindle_watts as f32 / 1000.0;
        if required_power_kw > available_power {
            Err(format!(
                "Required power {:.2}kW exceeds available power {:.2}kW",
                required_power_kw, available_power
            ))
        } else {
            Ok(())
        }
    }

    /// Get recommended spindle RPM range
    pub fn recommended_rpm_range(&self, tool_diameter: f32) -> (u32, u32) {
        let min_rpm = 1000u32;
        let max_rpm = self.device.max_spindle_speed_rpm;

        // Adjust based on tool diameter (larger tools need lower RPM)
        let diameter_factor = (6.0 / tool_diameter).clamp(0.5, 2.0);
        let adjusted_max = ((max_rpm as f32 * diameter_factor).min(max_rpm as f32)) as u32;

        (min_rpm, adjusted_max)
    }

    /// Check if coolant is available
    pub fn has_coolant(&self) -> bool {
        self.device.has_coolant
    }

    /// Get a copy of the device profile
    pub fn device(&self) -> &DeviceProfile {
        &self.device
    }

    /// Get device-specific recommendations
    pub fn get_recommendations(&self) -> DeviceRecommendations {
        DeviceRecommendations {
            max_rigorous_doc_percent: if self.device.device_type
                == gcodekit5_devicedb::model::DeviceType::CncMill
            {
                100.0
            } else {
                50.0
            },
            max_adaptive_engagement: if self.device.num_axes >= 3 {
                20.0
            } else {
                15.0
            },
            recommended_stepover_percent: match self.device.device_type {
                gcodekit5_devicedb::model::DeviceType::CncMill => 50.0,
                gcodekit5_devicedb::model::DeviceType::CncLathe => 80.0,
                _ => 40.0,
            },
        }
    }
}

/// Device limits for feeds and speeds
#[derive(Debug, Clone, Copy)]
pub struct DeviceLimits {
    /// Maximum spindle RPM
    pub max_rpm: u32,
    /// Maximum feed rate in mm/min
    pub max_feed_rate: f32,
    /// Maximum spindle power in kW
    pub max_power_kw: f32,
    /// Number of axes
    pub num_axes: u8,
    /// Has coolant system
    pub has_coolant: bool,
}

/// Device-specific recommendations
#[derive(Debug, Clone, Copy)]
pub struct DeviceRecommendations {
    /// Maximum depth of cut as percentage of diameter
    pub max_rigorous_doc_percent: f32,
    /// Maximum radial engagement for adaptive (percentage)
    pub max_adaptive_engagement: f32,
    /// Recommended stepover percentage
    pub recommended_stepover_percent: f32,
}

/// Unified data context combining all services
#[derive(Debug)]
pub struct SpeedsFeedsDataContext {
    /// Tool service for querying tools
    pub tool_service: SpeedsFeedsToolService,
    /// Material service for querying materials
    pub material_service: SpeedsFeedsMaterialService,
    /// Device service for machine capabilities
    pub device_service: Option<SpeedsFeedsDeviceService>,
}

impl Default for SpeedsFeedsDataContext {
    fn default() -> Self {
        Self {
            tool_service: SpeedsFeedsToolService::new(),
            material_service: SpeedsFeedsMaterialService::new(),
            device_service: None,
        }
    }
}

impl SpeedsFeedsDataContext {
    /// Create a new data context with all services
    pub fn new(device: Option<DeviceProfile>) -> Self {
        Self {
            tool_service: SpeedsFeedsToolService::new(),
            material_service: SpeedsFeedsMaterialService::new(),
            device_service: device.map(SpeedsFeedsDeviceService::new),
        }
    }

    /// Set the active device
    pub fn with_device(mut self, device: DeviceProfile) -> Self {
        self.device_service = Some(SpeedsFeedsDeviceService::new(device));
        self
    }

    /// Get device limits (or defaults if no device set)
    pub fn get_device_limits(&self) -> DeviceLimits {
        self.device_service
            .as_ref()
            .map(|s| s.get_limits())
            .unwrap_or_else(|| DeviceLimits {
                max_rpm: 12000,
                max_feed_rate: 1000.0,
                max_power_kw: 0.5,
                num_axes: 3,
                has_coolant: false,
            })
    }

    /// Check if all services are ready
    pub fn is_ready(&self) -> bool {
        // At minimum, we need tool and material services configured
        // Device is optional but recommended
        true
    }

    /// Get validation warnings if context is incomplete
    pub fn get_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.device_service.is_none() {
            warnings.push("No device profile selected. Using default limits.".to_string());
        }

        warnings
    }
}

/// Pre-calculated lookup table for common material/tool combinations
pub struct FeedsSpeedsLookupTable {
    /// Cached results by (tool_id, material_id, operation)
    cache: std::collections::HashMap<(String, String, String), super::CalculationResult>,
}

impl FeedsSpeedsLookupTable {
    /// Create a new empty lookup table
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }

    /// Get a cached result
    pub fn get(
        &self,
        tool_id: &str,
        material_id: &str,
        operation: &str,
    ) -> Option<&super::CalculationResult> {
        self.cache.get(&(
            tool_id.to_string(),
            material_id.to_string(),
            operation.to_string(),
        ))
    }

    /// Store a result in the cache
    pub fn cache(
        &mut self,
        tool_id: &str,
        material_id: &str,
        operation: &str,
        result: super::CalculationResult,
    ) {
        self.cache.insert(
            (
                tool_id.to_string(),
                material_id.to_string(),
                operation.to_string(),
            ),
            result,
        );
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for FeedsSpeedsLookupTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_service_filters() {
        let service = SpeedsFeedsToolService::new()
            .with_diameter_range(3.0, 12.0)
            .with_tool_types(vec![ToolType::EndMillFlat, ToolType::EndMillBall]);

        assert_eq!(service.min_diameter, Some(3.0));
        assert_eq!(service.max_diameter, Some(12.0));
        assert_eq!(service.tool_types.len(), 2);
    }

    #[test]
    fn test_material_service_filters() {
        let service = SpeedsFeedsMaterialService::new()
            .with_machinability_range(5, 8)
            .with_cutting_params_for("endmill_flat");

        assert_eq!(service.min_machinability, Some(5));
        assert_eq!(service.max_machinability, Some(8));
        assert_eq!(service.has_cutting_params, Some("endmill_flat".to_string()));
    }

    #[test]
    fn test_device_service_limits() {
        let device = DeviceProfile::default();
        let service = SpeedsFeedsDeviceService::new(device);
        let limits = service.get_limits();

        assert!(limits.max_rpm > 0);
        assert!(limits.max_feed_rate > 0.0);
    }

    #[test]
    fn test_lookup_table() {
        let mut table = FeedsSpeedsLookupTable::new();
        assert!(table.is_empty());

        // Create a dummy result - need to create minimal CalculationResult
        // We can't easily create one here without importing, so just test the basic operations
        assert_eq!(table.len(), 0);
        table.clear();
        assert!(table.is_empty());
    }
}
