use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use log::{debug, error};
use isahc::StatusCode;

use crate::CachedHttpClient;

// Structure defining a single preset configuration
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub icon: Option<String>,
    
    // Enabled component IDs
    pub enabled_features: Vec<String>,
    
    // Recommended settings
    pub recommended_memory: Option<i32>,
    pub recommended_java_args: Option<String>,
    
    // Whether this preset is featured/trending
    pub trending: Option<bool>,
    
    // Category for organization purposes
    pub category: Option<String>,
    
    // Visual customization
    pub background: Option<String>,
    pub color: Option<String>,
}

// Container for all presets to parse from JSON
#[derive(Debug, Deserialize, Serialize)]
pub struct PresetsContainer {
    pub version: String,
    pub last_updated: String,
    pub presets: Vec<Preset>,
}

// Default URL for presets
const DEFAULT_PRESETS_URL: &str = "https://raw.githubusercontent.com/Olinus10/installer-test/master/src/data/presets.json";

impl Preset {
    // Apply this preset to an installation, returning the list of enabled features
    pub fn apply_to_installation(&self, installation: &mut crate::installation::Installation) {
        debug!("Applying preset '{}' to installation '{}'", self.name, installation.name);
        
        // Update enabled features
        installation.enabled_features = self.enabled_features.clone();
        
        // Optionally update recommended settings
        if let Some(memory) = self.recommended_memory {
            installation.memory_allocation = memory;
            debug!("Updated memory allocation to {}", memory);
        }
        
        if let Some(java_args) = &self.recommended_java_args {
            installation.java_args = java_args.clone();
            debug!("Updated Java args to '{}'", java_args);
        }
        
        // Mark as modified and requiring update
        installation.modified = true;
    }
}

// Function to load presets from a URL
pub async fn load_presets(http_client: &CachedHttpClient, url: Option<&str>) -> Result<Vec<Preset>, String> {
    let presets_url = url.unwrap_or(DEFAULT_PRESETS_URL);
    debug!("Loading presets from: {}", presets_url);
    
    let response = match http_client.get_async(presets_url).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to fetch presets: {}", e);
            return Err(format!("Failed to fetch presets: {}", e));
        }
    };
    
    if response.status() != StatusCode::OK {
        error!("Failed to fetch presets: HTTP {}", response.status());
        return Err(format!("Failed to fetch presets: HTTP {}", response.status()));
    }
    
    let presets_json = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            error!("Failed to read presets response: {}", e);
            return Err(format!("Failed to read presets response: {}", e));
        }
    };
    
    // Parse the outer structure
    match serde_json::from_str::<PresetsContainer>(&presets_json) {
        Ok(container) => {
            debug!("Successfully loaded {} presets", container.presets.len());
            Ok(container.presets)
        },
        Err(e) => {
            error!("Failed to parse presets JSON: {}", e);
            Err(format!("Failed to parse presets JSON: {}", e))
        }
    }
}

// Find a preset by ID
pub fn find_preset_by_id(presets: &[Preset], id: &str) -> Option<Preset> {
    presets.iter()
        .find(|preset| preset.id == id)
        .cloned()
}
