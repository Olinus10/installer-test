use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use log::{debug, error};

use isahc::http::StatusCode;
use isahc::AsyncReadResponseExt;

use crate::CachedHttpClient;
use crate::Author;
use crate::preset::{Preset, PresetsContainer};

// Structure for a mod/component in the universal manifest
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ModComponent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source: String,
    pub location: String,
    pub version: String,
    pub path: Option<PathBuf>,
    pub optional: bool,
    pub default_enabled: bool,
    pub authors: Vec<Author>,
    pub category: Option<String>,  // Type of mod (gameplay, visual, etc.)
    pub dependencies: Option<Vec<String>>,  // IDs of required mods
    pub incompatibilities: Option<Vec<String>>,  // IDs of incompatible mods
}

// Complete universal manifest structure
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct UniversalManifest {
    pub manifest_version: i32,
    pub modpack_version: String,
    pub minecraft_version: String,
    pub name: String,
    pub subtitle: String,
    pub description: String,
    pub icon: bool,
    pub uuid: String,
    
    // Loader info
    pub loader: crate::Loader,
    
    // All available components
    pub mods: Vec<ModComponent>,
    pub shaderpacks: Vec<ModComponent>,
    pub resourcepacks: Vec<ModComponent>,
    
    // Metadata
    pub category: Option<String>,
    pub short_description: Option<String>,
    pub version: String,
    
    // Default settings
    pub max_mem: Option<i32>,
    pub min_mem: Option<i32>,
    pub java_args: Option<String>,
}

// Default URL for the universal manifest
const DEFAULT_UNIVERSAL_URL: &str = "https://raw.githubusercontent.com/Olinus10/installer-test/master/src/data/universal.json";
const DEFAULT_PRESETS_URL: &str = "https://raw.githubusercontent.com/Olinus10/installer-test/master/src/data/presets.json";

// Load the universal manifest from a URL
pub async fn load_universal_manifest(http_client: &CachedHttpClient, url: Option<&str>) -> Result<UniversalManifest, String> {
    // Use the correct raw content URL format for GitHub
    let manifest_url = url.unwrap_or("https://raw.githubusercontent.com/Olinus10/installer-test/master/src/data/universal.json");
    debug!("Loading universal manifest from: {}", manifest_url);
    
    // Add retry logic for more reliability
    let mut retries = 0;
    const MAX_RETRIES: usize = 3;
    
    loop {
        match http_client.get_async(manifest_url).await {
            Ok(mut response) => {
                let status = response.status();
                debug!("Got response with status: {}", status);
                
                if status != StatusCode::OK {
                    error!("Failed to fetch universal manifest: HTTP {}", status);
                    
                    if retries < MAX_RETRIES && (status.as_u16() >= 500 || status.as_u16() == 429) {
                        retries += 1;
                        debug!("Retrying request ({}/{})", retries, MAX_RETRIES);
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * retries as u64)).await;
                        continue;
                    }
                    
                    return Err(format!("Failed to fetch universal manifest: HTTP {}", status));
                }
                
                // Get text as String to avoid the unsized str error
                match response.text().await {
                    Ok(manifest_json) => {
                        debug!("Received manifest JSON of length: {}", manifest_json.len());
                        // Parse the universal manifest
                        match serde_json::from_str::<UniversalManifest>(&manifest_json) {
                            Ok(manifest) => {
                                debug!("Successfully loaded universal manifest for {}", manifest.name);
                                debug!("Manifest contains: {} mods, {} shaderpacks, {} resourcepacks", 
                                    manifest.mods.len(), manifest.shaderpacks.len(), manifest.resourcepacks.len());
                                return Ok(manifest);
                            },
                            Err(e) => {
                                error!("Failed to parse universal manifest JSON: {}", e);
                                
                                // Log the start of the JSON for debugging
                                let preview = if manifest_json.len() > 200 { 
                                    format!("{}...", &manifest_json[..200]) 
                                } else { 
                                    manifest_json.clone() 
                                };
                                debug!("Manifest JSON preview: {}", preview);
                                
                                return Err(format!("Failed to parse universal manifest JSON: {}", e));
                            }
                        }
                    },
                    Err(e) => {
                        error!("Failed to read universal manifest response: {}", e);
                        
                        if retries < MAX_RETRIES {
                            retries += 1;
                            debug!("Retrying request ({}/{})", retries, MAX_RETRIES);
                            tokio::time::sleep(tokio::time::Duration::from_millis(500 * retries as u64)).await;
                            continue;
                        }
                        
                        return Err(format!("Failed to read universal manifest response: {}", e));
                    }
                }
            },
            Err(e) => {
                error!("Failed to fetch universal manifest: {}", e);
                
                if retries < MAX_RETRIES {
                    retries += 1;
                    debug!("Retrying request ({}/{})", retries, MAX_RETRIES);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * retries as u64)).await;
                    continue;
                }
                
                return Err(format!("Failed to fetch universal manifest: {}", e));
            }
        }
    }
}

// Convert UniversalManifest to crate::Manifest format for compatibility
pub fn universal_to_manifest(universal: &UniversalManifest, enabled_features: Vec<String>) -> crate::Manifest {
    // Create features from components
    let mut features = vec![
        crate::Feature {
            id: "default".to_string(),
            name: "Core Components".to_string(),
            default: true,
            hidden: false,
            description: Some("Essential components for the modpack to function".to_string()),
        }
    ];
    
    // Add mods as features if they're optional
    for component in &universal.mods {
        if component.optional {
            features.push(crate::Feature {
                id: component.id.clone(),
                name: component.name.clone(),
                default: component.default_enabled,
                hidden: false,
                description: component.description.clone(),
            });
        }
    }
    
    // Similar for shaderpacks and resourcepacks
    for component in &universal.shaderpacks {
        if component.optional {
            features.push(crate::Feature {
                id: component.id.clone(),
                name: component.name.clone(),
                default: component.default_enabled,
                hidden: false,
                description: component.description.clone(),
            });
        }
    }
    
    for component in &universal.resourcepacks {
        if component.optional {
            features.push(crate::Feature {
                id: component.id.clone(),
                name: component.name.clone(),
                default: component.default_enabled,
                hidden: false,
                description: component.description.clone(),
            });
        }
    }
    
    // Convert mods from universal format to original format
    let mods = universal.mods.iter().map(|component| {
        crate::Mod {
            name: component.name.clone(),
            source: component.source.clone(),
            location: component.location.clone(),
            version: component.version.clone(),
            path: component.path.clone(),
            id: component.id.clone(),
            authors: component.authors.clone(),
        }
    }).collect();
    
    // Convert shaderpacks and resourcepacks similarly
    let shaderpacks = universal.shaderpacks.iter().map(|component| {
        crate::Shaderpack {
            name: component.name.clone(),
            source: component.source.clone(),
            location: component.location.clone(),
            version: component.version.clone(),
            path: component.path.clone(),
            id: component.id.clone(),
            authors: component.authors.clone(),
        }
    }).collect();
    
    let resourcepacks = universal.resourcepacks.iter().map(|component| {
        crate::Resourcepack {
            name: component.name.clone(),
            source: component.source.clone(),
            location: component.location.clone(),
            version: component.version.clone(),
            path: component.path.clone(),
            id: component.id.clone(),
            authors: component.authors.clone(),
        }
    }).collect();
    
    // Build the manifest
    crate::Manifest {
        manifest_version: universal.manifest_version,
        modpack_version: universal.modpack_version.clone(),
        name: universal.name.clone(),
        subtitle: universal.subtitle.clone(),
        tab_group: None,
        tab_title: None,
        tab_color: None,
        tab_background: None,
        tab_primary_font: None,
        tab_secondary_font: None,
        settings_background: None,
        popup_title: None,
        popup_contents: None,
        description: universal.description.clone(),
        icon: universal.icon,
        uuid: universal.uuid.clone(),
        loader: universal.loader.clone(),
        mods,
        shaderpacks,
        resourcepacks,
        remote_include: None,
        include: Vec::new(),
        features,
        trend: None,
        enabled_features,
        included_files: None,
        source: None,
        installer_path: None,
        max_mem: universal.max_mem,
        min_mem: universal.min_mem,
        java_args: universal.java_args.clone(),
        category: universal.category.clone(),
        is_new: None,
        short_description: universal.short_description.clone(),
    }
}
