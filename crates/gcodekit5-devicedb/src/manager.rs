//! gcodekit5-devicedb/src/manager.rs
//! # Device Manager
//!
//! Handles loading, saving, and managing CNC device profiles.
//! Supports JSON-based persistence and provides the main API
//! for device CRUD operations.

use crate::model::DeviceProfile;
use crate::traits::DeviceProfileProvider;
use anyhow::{Context, Result};
use gcodekit5_core::{thread_safe, thread_safe_rw, ThreadSafe, ThreadSafeRw, ThreadSafeRwMap};
use std::fs;
use std::path::PathBuf;

/// Thread-safe store of device profiles persisted to a JSON file.
#[derive(Debug, Clone)]
pub struct DeviceManager {
    profiles: ThreadSafeRwMap<String, DeviceProfile>,
    active_profile_id: ThreadSafeRw<Option<String>>,
    config_path: PathBuf,
    file_lock: ThreadSafe<()>,
    /// Set when the last load could not read part of an existing file; the next save
    /// backs the original up before overwriting it.
    needs_backup: ThreadSafeRw<bool>,
}

impl DeviceManager {
    /// Create a manager backed by the JSON file at `config_path`. Nothing is read until
    /// [`DeviceManager::load`] is called.
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            profiles: thread_safe_rw(std::collections::HashMap::new()),
            active_profile_id: thread_safe_rw(None),
            config_path,
            file_lock: thread_safe(()),
            needs_backup: thread_safe_rw(false),
        }
    }

    /// Load device profiles from the configuration file.
    ///
    /// If the file does not exist a default profile is created, saved and made active.
    /// Individual profile entries that cannot be deserialised are skipped (and logged)
    /// so one bad entry does not hide every other profile. When anything in an existing
    /// file could not be read, the next [`DeviceManager::save`] first copies the original
    /// file to `<name>.bak-<UTC timestamp>` so the unreadable data is never lost.
    ///
    /// # Errors
    /// Returns an error if the file exists but cannot be read or is not valid JSON.
    /// In that case the in-memory profiles are left unchanged and a backup is still
    /// taken before any later save overwrites the file.
    pub fn load(&self) -> Result<()> {
        if !self.config_path.exists() {
            // Create default profile if file doesn't exist
            let default_profile = DeviceProfile::default();
            self.save_profile(default_profile.clone())?;
            self.set_active_profile(&default_profile.id)?;
            return Ok(());
        }

        let content = match fs::read_to_string(&self.config_path) {
            Ok(c) => c,
            Err(e) => {
                self.mark_needs_backup();
                return Err(e).context("Failed to read device profiles file");
            }
        };

        let data: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                self.mark_needs_backup();
                tracing::error!(
                    path = %self.config_path.display(),
                    error = %e,
                    "Device profiles file is not valid JSON; it will be backed up before the next save"
                );
                return Err(e).context("Failed to parse device profiles JSON");
            }
        };

        let mut profiles_map = std::collections::HashMap::new();
        let mut skipped = 0usize;

        match data.get("profiles").and_then(|v| v.as_array()) {
            Some(profiles_array) => {
                for (index, p) in profiles_array.iter().enumerate() {
                    match serde_json::from_value::<DeviceProfile>(p.clone()) {
                        Ok(profile) => {
                            profiles_map.insert(profile.id.clone(), profile);
                        }
                        Err(e) => {
                            skipped += 1;
                            tracing::warn!(
                                path = %self.config_path.display(),
                                index,
                                error = %e,
                                "Skipping unreadable device profile entry"
                            );
                        }
                    }
                }
            }
            None => {
                // An existing file with no usable profiles array would otherwise be
                // silently replaced by the next save.
                skipped += 1;
                tracing::warn!(
                    path = %self.config_path.display(),
                    "Device profiles file has no 'profiles' array"
                );
            }
        }

        if skipped > 0 {
            self.mark_needs_backup();
        }

        let active_id = data
            .get("active_id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .filter(|id| profiles_map.contains_key(id));

        {
            let mut profiles_lock = self.profiles.write();
            *profiles_lock = profiles_map;

            let mut active_lock = self.active_profile_id.write();
            *active_lock = active_id;
        }

        Ok(())
    }

    /// Returns `true` if the last [`DeviceManager::load`] found data it could not read
    /// and the original file has not been backed up yet.
    pub fn has_unreadable_entries(&self) -> bool {
        *self.needs_backup.read()
    }

    fn mark_needs_backup(&self) {
        *self.needs_backup.write() = true;
    }

    /// Path used for a backup of the profiles file taken at the current time.
    fn backup_path(&self) -> PathBuf {
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
        let file_name = self
            .config_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "devices.json".to_string());
        self.config_path
            .with_file_name(format!("{file_name}.bak-{stamp}"))
    }

    /// Copy the current profiles file aside if the last load could not read all of it.
    /// Must be called with `file_lock` held.
    fn backup_if_needed(&self) -> Result<()> {
        let mut needs_backup = self.needs_backup.write();
        if !*needs_backup {
            return Ok(());
        }
        if self.config_path.exists() {
            let backup = self.backup_path();
            fs::copy(&self.config_path, &backup).with_context(|| {
                format!(
                    "Failed to back up device profiles file to {}",
                    backup.display()
                )
            })?;
            tracing::warn!(
                backup = %backup.display(),
                "Backed up device profiles file before overwriting unreadable entries"
            );
        }
        *needs_backup = false;
        Ok(())
    }

    /// Persist all profiles and the active profile id to the configuration file.
    ///
    /// The write is atomic (temp file + rename). If the last load skipped unreadable
    /// data, the original file is first copied to a timestamped `.bak-` file; if that
    /// backup fails the save is aborted so nothing is lost.
    ///
    /// # Errors
    /// Returns an error if the backup, serialisation or file write fails.
    pub fn save(&self) -> Result<()> {
        // Acquire file lock to prevent concurrent writes
        let _file_guard = self.file_lock.lock();

        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        self.backup_if_needed()?;

        let content = {
            let profiles_lock = self.profiles.read();
            let active_lock = self.active_profile_id.read();

            let mut profiles_vec: Vec<&DeviceProfile> = profiles_lock.values().collect();
            profiles_vec.sort_by(|a, b| a.id.cmp(&b.id));

            let data = serde_json::json!({
                "profiles": profiles_vec,
                "active_id": *active_lock
            });
            serde_json::to_string_pretty(&data)?
        };

        // Write to a sibling temp file and rename so a crash mid-write cannot leave a
        // truncated profiles file (which would then fail to load).
        let tmp_path = self.config_path.with_extension("json.tmp");
        fs::write(&tmp_path, content).context("Failed to write device profiles file")?;
        fs::rename(&tmp_path, &self.config_path)
            .context("Failed to replace device profiles file")?;

        Ok(())
    }

    /// Return a clone of the profile with the given id, if present.
    pub fn get_profile(&self, id: &str) -> Option<DeviceProfile> {
        self.profiles.read().get(id).cloned()
    }

    /// Return clones of all loaded profiles (unordered).
    pub fn get_all_profiles(&self) -> Vec<DeviceProfile> {
        self.profiles.read().values().cloned().collect()
    }

    /// Insert or replace a profile and persist the file.
    ///
    /// # Errors
    /// Returns an error if saving fails.
    pub fn save_profile(&self, profile: DeviceProfile) -> Result<()> {
        {
            let mut lock = self.profiles.write();
            lock.insert(profile.id.clone(), profile);
        }
        self.save()
    }

    /// Remove a profile (clearing the active selection if it was active) and persist.
    ///
    /// # Errors
    /// Returns an error if saving fails.
    pub fn delete_profile(&self, id: &str) -> Result<()> {
        {
            let mut lock = self.profiles.write();
            lock.remove(id);
        }

        // If active profile was deleted, clear active selection
        {
            let mut active_lock = self.active_profile_id.write();
            if let Some(active) = &*active_lock {
                if active == id {
                    *active_lock = None;
                }
            }
        }

        self.save()
    }

    /// Mark an existing profile as active and persist.
    ///
    /// # Errors
    /// Returns an error if the profile does not exist or saving fails.
    pub fn set_active_profile(&self, id: &str) -> Result<()> {
        let exists = self.profiles.read().contains_key(id);
        if exists {
            {
                let mut lock = self.active_profile_id.write();
                *lock = Some(id.to_string());
            }
            self.save()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Profile not found"))
        }
    }

    /// Return the active profile, if one is set and still exists.
    pub fn get_active_profile(&self) -> Option<DeviceProfile> {
        let active_id = self.active_profile_id.read().clone()?;
        self.get_profile(&active_id)
    }
}

impl DeviceProfileProvider for DeviceManager {
    fn get_active_profile(&self) -> Option<DeviceProfile> {
        self.get_active_profile()
    }

    fn get_profile(&self, id: &str) -> Option<DeviceProfile> {
        self.get_profile(id)
    }
}
