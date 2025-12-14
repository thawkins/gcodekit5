//! CNC Tools Manager Backend
//!
//! This module provides backend logic for the CNC Tools Manager UI,
//! including persistence for custom tools.

use gcodekit5_core::data::gtc_import::{GtcImportResult, GtcImporter};
use gcodekit5_core::data::tools::{
    ShankType, Tool, ToolCoating, ToolCuttingParams, ToolId, ToolLibrary, ToolMaterial, ToolType,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedTool {
    pub id: ToolId,
    pub name: String,
    pub description: String,
    pub tool_type: ToolType,

    pub diameter: f32,
    pub shaft_diameter: Option<f32>,
    pub length: f32,
    pub flute_length: f32,
    pub flutes: u32,
    pub corner_radius: Option<f32>,
    pub tip_angle: Option<f32>,

    pub material: ToolMaterial,
    pub coating: Option<ToolCoating>,
    pub shank: ShankType,

    pub params: ToolCuttingParams,

    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
    pub cost: Option<f32>,
    pub notes: String,
    pub custom: bool,
}

impl From<&Tool> for PersistedTool {
    fn from(t: &Tool) -> Self {
        Self {
            id: t.id.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
            tool_type: t.tool_type,
            diameter: t.diameter,
            shaft_diameter: t.shaft_diameter,
            length: t.length,
            flute_length: t.flute_length,
            flutes: t.flutes,
            corner_radius: t.corner_radius,
            tip_angle: t.tip_angle,
            material: t.material,
            coating: t.coating,
            shank: t.shank,
            params: t.params.clone(),
            manufacturer: t.manufacturer.clone(),
            part_number: t.part_number.clone(),
            cost: t.cost,
            notes: t.notes.clone(),
            custom: t.custom,
        }
    }
}

impl From<PersistedTool> for Tool {
    fn from(t: PersistedTool) -> Self {
        // Tool number is intentionally not persisted (it is not a device tool index).
        // Use 0 as a stable placeholder.
        let mut tool = Tool::new(t.id, 0, t.name, t.tool_type, t.diameter, t.length);
        tool.description = t.description;
        tool.shaft_diameter = t.shaft_diameter;
        tool.flute_length = t.flute_length;
        tool.flutes = t.flutes;
        tool.corner_radius = t.corner_radius;
        tool.tip_angle = t.tip_angle;
        tool.material = t.material;
        tool.coating = t.coating;
        tool.shank = t.shank;
        tool.params = t.params;
        tool.manufacturer = t.manufacturer;
        tool.part_number = t.part_number;
        tool.cost = t.cost;
        tool.notes = t.notes;
        tool.custom = t.custom;
        tool
    }
}

pub struct ToolsManagerBackend {
    library: ToolLibrary,
    storage_path: PathBuf,
}

impl ToolsManagerBackend {
    pub fn new() -> Self {
        let storage_path = Self::get_storage_path();
        let mut library = gcodekit5_core::data::tools::init_standard_library();

        // Load custom tools from disk if they exist
        if let Ok(custom_tools) = Self::load_from_file(&storage_path) {
            for tool in custom_tools {
                library.add_tool(tool);
            }
        }

        Self {
            library,
            storage_path,
        }
    }

    fn get_storage_path() -> PathBuf {
        let mut path = dirs::config_dir()
            .or_else(|| dirs::home_dir())
            .unwrap_or_else(|| PathBuf::from("."));
        path.push("gcodekit5");
        std::fs::create_dir_all(&path).ok();
        path.push("custom_tools.json");
        path
    }

    fn load_from_file(path: &PathBuf) -> Result<Vec<Tool>, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let tools: Vec<PersistedTool> = serde_json::from_str(&contents)?;
        Ok(tools.into_iter().map(Into::into).collect())
    }

    fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Only save custom tools (tool numbers are intentionally not persisted)
        let custom_tools: Vec<PersistedTool> = self
            .library
            .get_all_tools()
            .into_iter()
            .filter(|t| t.custom)
            .map(PersistedTool::from)
            .collect();

        let json = serde_json::to_string_pretty(&custom_tools)?;
        std::fs::write(&self.storage_path, json)?;
        Ok(())
    }

    pub fn get_library(&self) -> &ToolLibrary {
        &self.library
    }

    pub fn get_library_mut(&mut self) -> &mut ToolLibrary {
        &mut self.library
    }

    pub fn add_tool(&mut self, tool: Tool) {
        self.library.add_tool(tool);
        // Save custom tools to disk
        if let Err(e) = self.save_to_file() {
            tracing::warn!("Failed to save tools: {}", e);
        }
    }

    pub fn remove_tool(&mut self, id: &ToolId) -> Option<Tool> {
        let result = self.library.remove_tool(id);
        // Save custom tools to disk
        if let Err(e) = self.save_to_file() {
            tracing::warn!("Failed to save tools: {}", e);
        }
        result
    }

    pub fn get_tool(&self, id: &ToolId) -> Option<&Tool> {
        self.library.get_tool(id)
    }

    pub fn search_tools(&self, query: &str) -> Vec<&Tool> {
        if query.is_empty() {
            self.library.get_all_tools()
        } else {
            self.library.search_by_name(query)
        }
    }

    pub fn filter_by_type(&self, tool_type: ToolType) -> Vec<&Tool> {
        self.library.get_tools_by_type(tool_type)
    }

    pub fn filter_by_diameter(&self, diameter: f32, tolerance: f32) -> Vec<&Tool> {
        self.library
            .search_by_diameter(diameter - tolerance, diameter + tolerance)
    }

    pub fn get_all_tools(&self) -> Vec<&Tool> {
        self.library.get_all_tools()
    }

    pub fn export_custom_tools<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let custom_tools: Vec<PersistedTool> = self
            .library
            .get_all_tools()
            .into_iter()
            .filter(|t| t.custom)
            .map(PersistedTool::from)
            .collect();

        let json = serde_json::to_string_pretty(&custom_tools)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn reset_custom_tools(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Remove custom tools from library
        let ids: Vec<ToolId> = self
            .library
            .get_all_tools()
            .into_iter()
            .filter(|t| t.custom)
            .map(|t| t.id.clone())
            .collect();

        for id in ids {
            let _ = self.library.remove_tool(&id);
        }

        // Remove persisted file
        if self.storage_path.exists() {
            let _ = std::fs::remove_file(&self.storage_path);
        }

        Ok(())
    }

    /// Import tools from a GTC package (.zip file)
    pub fn import_gtc_package<P: AsRef<Path>>(
        &mut self,
        zip_path: P,
    ) -> Result<GtcImportResult, Box<dyn std::error::Error>> {
        // Determine next tool number
        let next_number = self
            .library
            .get_all_tools()
            .iter()
            .map(|t| t.number)
            .max()
            .unwrap_or(0)
            + 1;

        let mut importer = GtcImporter::new(next_number);
        let result = importer.import_from_zip(zip_path)?;

        // Add imported tools to library
        for tool in &result.imported_tools {
            self.library.add_tool(tool.clone());
        }

        // Save to disk
        if let Err(e) = self.save_to_file() {
            tracing::warn!("Failed to save tools after GTC import: {}", e);
        }

        Ok(result)
    }

    /// Import tools from a GTC JSON file
    pub fn import_gtc_json<P: AsRef<Path>>(
        &mut self,
        json_path: P,
    ) -> Result<GtcImportResult, Box<dyn std::error::Error>> {
        // Determine next tool number
        let next_number = self
            .library
            .get_all_tools()
            .iter()
            .map(|t| t.number)
            .max()
            .unwrap_or(0)
            + 1;

        let mut importer = GtcImporter::new(next_number);
        let result = importer.import_from_json(json_path)?;

        // Add imported tools to library
        for tool in &result.imported_tools {
            self.library.add_tool(tool.clone());
        }

        // Save to disk
        if let Err(e) = self.save_to_file() {
            tracing::warn!("Failed to save tools after GTC import: {}", e);
        }

        Ok(result)
    }
}

impl Default for ToolsManagerBackend {
    fn default() -> Self {
        Self::new()
    }
}

// Helper conversion functions for UI
pub fn string_to_tool_type(s: &str) -> Option<ToolType> {
    ToolType::all().iter().find(|t| t.to_string() == s).cloned()
}

pub fn string_to_tool_material(s: &str) -> Option<ToolMaterial> {
    match s {
        "HSS" => Some(ToolMaterial::HSS),
        "Carbide" => Some(ToolMaterial::Carbide),
        "Coated Carbide" => Some(ToolMaterial::CoatedCarbide),
        "Diamond" => Some(ToolMaterial::Diamond),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = ToolsManagerBackend::new();
        assert!(!backend.get_all_tools().is_empty());
    }

    #[test]
    fn test_tool_type_conversion() {
        assert_eq!(
            string_to_tool_type("Flat End Mill"),
            Some(ToolType::EndMillFlat)
        );
        assert_eq!(string_to_tool_type("Drill Bit"), Some(ToolType::DrillBit));
    }

    #[test]
    fn test_search_tools() {
        let backend = ToolsManagerBackend::new();
        let results = backend.search_tools("end");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_persistence() {
        // Create a test tool
        let test_tool = Tool::new(
            ToolId("test_persist_tool".to_string()),
            999, // tool number
            "Test Persist Tool".to_string(),
            ToolType::EndMillFlat,
            6.35, // diameter
            38.0, // length
        );

        // Add and save
        {
            let mut backend = ToolsManagerBackend::new();
            let mut tool = test_tool.clone();
            tool.custom = true;
            backend.add_tool(tool);
        }

        // Create new backend and verify tool was loaded
        {
            let backend = ToolsManagerBackend::new();
            let loaded = backend.get_tool(&ToolId("test_persist_tool".to_string()));
            assert!(loaded.is_some());
            assert_eq!(loaded.unwrap().name, "Test Persist Tool");
        }

        // Cleanup
        {
            let mut backend = ToolsManagerBackend::new();
            backend.remove_tool(&ToolId("test_persist_tool".to_string()));
        }
    }
}
