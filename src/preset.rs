use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use log::{debug, error};

use isahc::http::StatusCode;
use isahc::AsyncReadResponseExt;
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
    // Use the correct raw content URL format for GitHub
    let presets_url = url.unwrap_or("https://raw.githubusercontent.com/Olinus10/installer-test/master/src/data/presets.json");
    debug!("Loading presets from: {}", presets_url);
    
    // Add retry logic for more reliability
    let mut retries = 0;
    const MAX_RETRIES: usize = 3;
    
    loop {
        match http_client.get_async(presets_url).await {
            Ok(mut response) => {
                let status = response.status();
                debug!("Got response with status: {}", status);
                
                if status != StatusCode::OK {
                    error!("Failed to fetch presets: HTTP {}", status);
                    
                    if retries < MAX_RETRIES && (status.as_u16() >= 500 || status.as_u16() == 429) {
                        retries += 1;
                        debug!("Retrying request ({}/{})", retries, MAX_RETRIES);
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * retries as u64)).await;
                        continue;
                    }
                    
                    return Err(format!("Failed to fetch presets: HTTP {}", status));
                }
                
                // Get text as String
                match response.text().await {
                    Ok(presets_json) => {
                        debug!("Received presets JSON of length: {}", presets_json.len());
                        // Parse the presets container
                        match serde_json::from_str::<PresetsContainer>(&presets_json) {
                            Ok(container) => {
                                debug!("Successfully loaded {} presets (version: {})", 
                                      container.presets.len(), container.version);
                                
                                // Log preset information for debugging
                                for preset in &container.presets {
                                    debug!("Loaded preset: {} (ID: {})", preset.name, preset.id);
                                }
                                
                                return Ok(container.presets);
                            },
                            Err(e) => {
                                error!("Failed to parse presets JSON: {}", e);
                                
                                // Log the start of the JSON for debugging
                                let preview = if presets_json.len() > 200 { 
                                    format!("{}...", &presets_json[..200]) 
                                } else { 
                                    presets_json.clone() 
                                };
                                debug!("Presets JSON preview: {}", preview);
                                
                                return Err(format!("Failed to parse presets JSON: {}", e));
                            }
                        }
                    },
                    Err(e) => {
                        error!("Failed to read presets response: {}", e);
                        
                        if retries < MAX_RETRIES {
                            retries += 1;
                            debug!("Retrying request ({}/{})", retries, MAX_RETRIES);
                            tokio::time::sleep(tokio::time::Duration::from_millis(500 * retries as u64)).await;
                            continue;
                        }
                        
                        return Err(format!("Failed to read presets response: {}", e));
                    }
                }
            },
            Err(e) => {
                error!("Failed to fetch presets: {}", e);
                
                if retries < MAX_RETRIES {
                    retries += 1;
                    debug!("Retrying request ({}/{})", retries, MAX_RETRIES);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * retries as u64)).await;
                    continue;
                }
                
                return Err(format!("Failed to fetch presets: {}", e));
            }
        }
    }
}


// Find a preset by ID
pub fn find_preset_by_id(presets: &[Preset], id: &str) -> Option<Preset> {
    debug!("Looking for preset with ID: {}", id);
    
    let result = presets.iter()
        .find(|preset| preset.id == id)
        .cloned();
        
    match &result {
        Some(preset) => debug!("Found preset: {}", preset.name),
        None => debug!("No preset found with ID: {}", id),
    }
    
    result
}
