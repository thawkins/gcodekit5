//! Materials Manager Backend
//!
//! This module provides helper functions for the Materials Manager UI.
//! The actual UI integration happens in main.rs after slint modules are generated.

use gcodekit5_core::data::materials::{
    Abrasiveness, ChipType, HazardLevel, HeatSensitivity, Material, MaterialCategory, MaterialId,
    MaterialLibrary, SurfaceFinishability,
};
use std::path::PathBuf;

pub struct MaterialsManagerBackend {
    library: MaterialLibrary,
    storage_path: PathBuf,
}

impl MaterialsManagerBackend {
    pub fn new() -> Self {
        let storage_path = Self::get_storage_path();
        let mut library = gcodekit5_core::data::materials::init_standard_library();

        // Load custom materials from disk if they exist
        if let Ok(custom_materials) = Self::load_from_file(&storage_path) {
            for material in custom_materials {
                library.add_material(material);
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
        path.push("custom_materials.json");
        path
    }

    fn load_from_file(path: &PathBuf) -> Result<Vec<Material>, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let materials: Vec<Material> = serde_json::from_str(&contents)?;
        Ok(materials)
    }

    fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Only save custom materials
        let custom_materials: Vec<&Material> = self
            .library
            .get_all_materials()
            .into_iter()
            .filter(|m| m.custom)
            .collect();

        let json = serde_json::to_string_pretty(&custom_materials)?;
        std::fs::write(&self.storage_path, json)?;
        Ok(())
    }

    pub fn get_library(&self) -> &MaterialLibrary {
        &self.library
    }

    pub fn get_library_mut(&mut self) -> &mut MaterialLibrary {
        &mut self.library
    }

    pub fn add_material(&mut self, material: Material) {
        self.library.add_material(material);
        // Save custom materials to disk
        if let Err(e) = self.save_to_file() {
            tracing::warn!("Failed to save materials: {}", e);
        }
    }

    pub fn remove_material(&mut self, id: &MaterialId) -> Option<Material> {
        let result = self.library.remove_material(id);
        // Save custom materials to disk
        if let Err(e) = self.save_to_file() {
            tracing::warn!("Failed to save materials: {}", e);
        }
        result
    }

    pub fn get_material(&self, id: &MaterialId) -> Option<&Material> {
        self.library.get_material(id)
    }

    pub fn search_materials(&self, query: &str) -> Vec<&Material> {
        if query.is_empty() {
            self.library.get_all_materials()
        } else {
            self.library.search_by_name(query)
        }
    }

    pub fn filter_by_category(&self, category: MaterialCategory) -> Vec<&Material> {
        self.library.get_materials_by_category(category)
    }

    pub fn get_all_materials(&self) -> Vec<&Material> {
        self.library.get_all_materials()
    }
}

impl Default for MaterialsManagerBackend {
    fn default() -> Self {
        Self::new()
    }
}

pub fn string_to_category(s: &str) -> Option<MaterialCategory> {
    match s {
        "Wood" => Some(MaterialCategory::Wood),
        "Engineered Wood" => Some(MaterialCategory::EngineeredWood),
        "Plastic" => Some(MaterialCategory::Plastic),
        "Non-Ferrous Metal" => Some(MaterialCategory::NonFerrousMetal),
        "Ferrous Metal" => Some(MaterialCategory::FerrousMetal),
        "Composite" => Some(MaterialCategory::Composite),
        "Stone & Ceramic" => Some(MaterialCategory::StoneAndCeramic),
        _ => None,
    }
}

pub fn string_to_chip_type(s: &str) -> ChipType {
    match s {
        "Continuous" => ChipType::Continuous,
        "Segmented" => ChipType::Segmented,
        "Granular" => ChipType::Granular,
        "Small" => ChipType::Small,
        _ => ChipType::Continuous,
    }
}

pub fn string_to_heat_sensitivity(s: &str) -> HeatSensitivity {
    match s {
        "Low" => HeatSensitivity::Low,
        "Moderate" => HeatSensitivity::Moderate,
        "High" => HeatSensitivity::High,
        _ => HeatSensitivity::Low,
    }
}

pub fn string_to_abrasiveness(s: &str) -> Abrasiveness {
    match s {
        "Low" => Abrasiveness::Low,
        "Moderate" => Abrasiveness::Moderate,
        "High" => Abrasiveness::High,
        _ => Abrasiveness::Low,
    }
}

pub fn string_to_surface_finish(s: &str) -> SurfaceFinishability {
    match s {
        "Excellent" => SurfaceFinishability::Excellent,
        "Good" => SurfaceFinishability::Good,
        "Fair" => SurfaceFinishability::Fair,
        "Rough" => SurfaceFinishability::Rough,
        _ => SurfaceFinishability::Good,
    }
}

pub fn string_to_hazard_level(s: &str) -> HazardLevel {
    match s {
        "None" => HazardLevel::None,
        "Minimal" => HazardLevel::Minimal,
        "Moderate" => HazardLevel::Moderate,
        "High" => HazardLevel::High,
        _ => HazardLevel::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = MaterialsManagerBackend::new();
        assert!(!backend.get_all_materials().is_empty());
    }

    #[test]
    fn test_category_conversion() {
        assert_eq!(string_to_category("Wood"), Some(MaterialCategory::Wood));
        assert_eq!(
            string_to_category("Non-Ferrous Metal"),
            Some(MaterialCategory::NonFerrousMetal)
        );
    }

    #[test]
    fn test_search_materials() {
        let backend = MaterialsManagerBackend::new();
        let results = backend.search_materials("oak");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_persistence() {

        // Create a test material
        let test_material = Material::new(
            MaterialId("test_persist".to_string()),
            "Test Persist Material".to_string(),
            MaterialCategory::Wood,
            "Test".to_string(),
        );

        // Add and save
        {
            let mut backend = MaterialsManagerBackend::new();
            let mut material = test_material.clone();
            material.custom = true;
            backend.add_material(material);
        }

        // Create new backend and verify material was loaded
        {
            let backend = MaterialsManagerBackend::new();
            let loaded = backend.get_material(&MaterialId("test_persist".to_string()));
            assert!(loaded.is_some());
            assert_eq!(loaded.unwrap().name, "Test Persist Material");
        }

        // Cleanup
        {
            let mut backend = MaterialsManagerBackend::new();
            backend.remove_material(&MaterialId("test_persist".to_string()));
        }
    }
}
