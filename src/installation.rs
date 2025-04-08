use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use log::{debug, error, info};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{CachedHttpClient, InstallerProfile, launcher};
use crate::preset::Preset;

// Structure for managing an installation
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Installation {
    // Core identity properties
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    
    // Minecraft configuration
    pub minecraft_version: String,
    pub loader_type: String,      // "fabric", "quilt", etc.
    pub loader_version: String,
    
    // Path information
    pub installation_path: PathBuf,
    
    // Mod configuration
    pub enabled_features: Vec<String>,
    
    // Performance settings
    pub memory_allocation: i32,   // in MB
    pub java_args: String,
    
    // Account linking
    pub linked_account_id: Option<String>,
    
    // Installation status tracking
    pub installed: bool,
    pub modified: bool,           // True if changes were made but not yet applied
    pub update_available: bool,   // True if modpack has updates
    
    // Launcher and versioning information
    pub launcher_type: String,    // "vanilla", "multimc", etc.
    pub universal_version: String, // Which version of the universal modpack this uses
    
    // Last launch info for statistics
    pub last_launch: Option<DateTime<Utc>>,
    pub total_launches: u32,
}

// Implement to_installer_profile method
impl Installation {
    // Convert to an InstallerProfile for the actual installation process
    pub fn to_installer_profile(&self, http_client: &CachedHttpClient) -> InstallerProfile {
        // This is a stub implementation that will need to be completed
        // based on your actual InstallerProfile structure
        
        // Load the universal manifest
        let universal_manifest = match crate::universal::load_universal_manifest(http_client, None) {
            Ok(manifest) => manifest,
            Err(_) => panic!("Failed to load universal manifest"),
        };
        
        // Convert enabled_features to the format expected by InstallerProfile
        let enabled_features = self.enabled_features.clone();
        
        // Create a launcher instance based on the launcher_type
        let launcher = match self.launcher_type.as_str() {
            "vanilla" => Some(crate::Launcher::Vanilla(crate::get_app_data())),
            "multimc" => crate::get_multimc_folder("MultiMC").ok().map(crate::Launcher::MultiMC),
            "prism" => crate::get_multimc_folder("PrismLauncher").ok().map(crate::Launcher::MultiMC),
            custom if custom.starts_with("custom-") => {
                let path = PathBuf::from(custom.trim_start_matches("custom-"));
                Some(crate::Launcher::MultiMC(path))
            },
            _ => None,
        };
        
        // Return the InstallerProfile
        InstallerProfile {
            manifest: crate::universal::universal_to_manifest(&universal_manifest, enabled_features.clone()),
            http_client: http_client.clone(),
            installed: self.installed,
            update_available: self.update_available,
            modpack_source: "Wynncraft-Overhaul/majestic-overhaul/".to_string(),
            modpack_branch: "main".to_string(), // Default branch - may need to be configurable
            enabled_features,
            launcher,
            local_manifest: None, // This would need to be populated from disk if exists
            changelog: None, // This would need to be loaded separately
        }
    }
    
    // Launch the installation
    pub fn launch(&mut self) -> Result<(), String> {
        info!("Launching installation '{}'", self.name);
        
        // Use the Microsoft auth launch method
        match crate::launcher::microsoft_auth::MicrosoftAuth::launch_minecraft(&self.id) {
            Ok(_) => {
                // Update statistics
                self.last_launch = Some(Utc::now());
                self.total_launches += 1;
                self.last_used = Utc::now();
                self.save()?;
                
                info!("Successfully launched installation '{}'", self.name);
                Ok(())
            },
            Err(e) => {
                error!("Failed to launch installation '{}': {}", self.name, e);
                Err(format!("Failed to launch: {}", e))
            }
        }
    }
}
