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
    #[serde(default)]  // Add default in case this field is missing
    pub shaderpacks: Vec<ModComponent>,
    #[serde(default)]  // Add default in case this field is missing
    pub resourcepacks: Vec<ModComponent>,
    
    // Metadata - make all these optional
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub short_description: Option<String>,
    pub version: String,
    
    // Default settings - all optional
    #[serde(default)]
    pub max_mem: Option<i32>,
    #[serde(default)]
    pub min_mem: Option<i32>,
    #[serde(default)]
    pub java_args: Option<String>,
}

// Structure for a mod/component with more flexible options
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ModComponent {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub source: String,
    pub location: String,
    pub version: String,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default = "default_false")]
    pub optional: bool,
    #[serde(default = "default_false")]
    pub default_enabled: bool,
    #[serde(default)]
    pub authors: Vec<Author>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
    #[serde(default)]
    pub incompatibilities: Option<Vec<String>>,
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Clone)]
pub struct ManifestError {
    pub message: String,
    pub error_type: ManifestErrorType,
    pub file_name: String,
    pub raw_content: Option<String>, // Store the raw content for debugging
}

#[derive(Debug, Clone, PartialEq)]
pub enum ManifestErrorType {
    NetworkError,
    SyntaxError,
    DeserializationError,
    ValidationError,
    UnknownError,
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} in {}: {}", self.error_type, self.file_name, self.message)
    }
}

impl std::fmt::Display for ManifestErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestErrorType::NetworkError => write!(f, "Network Error"),
            ManifestErrorType::SyntaxError => write!(f, "Syntax Error"),
            ManifestErrorType::DeserializationError => write!(f, "Deserialization Error"),
            ManifestErrorType::ValidationError => write!(f, "Validation Error"),
            ManifestErrorType::UnknownError => write!(f, "Unknown Error"),
        }
    }
}

pub async fn validate_universal_json(http_client: &CachedHttpClient, url: &str) -> Result<(), String> {
    debug!("Validating universal.json structure from: {}", url);
    
    match http_client.get_async(url).await {
        Ok(mut response) => {
            let status = response.status();
            
            if status != StatusCode::OK {
                return Err(format!("Failed to fetch universal.json: HTTP {}", status));
            }
            
            match response.text().await {
                Ok(json_text) => {
                    debug!("Received {} bytes of universal.json", json_text.len());
                    
                    // First check if it's valid JSON
                    match serde_json::from_str::<serde_json::Value>(&json_text) {
                        Ok(value) => {
                            debug!("universal.json is valid JSON");
                            
                            // Check if it's an object
                            if let Some(obj) = value.as_object() {
                                debug!("Top-level fields in universal.json:");
                                for (key, val) in obj {
                                    let type_name = match val {
                                        serde_json::Value::Null => "null",
                                        serde_json::Value::Bool(_) => "boolean",
                                        serde_json::Value::Number(_) => "number",
                                        serde_json::Value::String(_) => "string",
                                        serde_json::Value::Array(_) => "array",
                                        serde_json::Value::Object(_) => "object",
                                    };
                                    debug!("  - {}: {}", key, type_name);
                                }
                                
                                // Now check for required fields
                                let required_fields = vec![
                                    "manifest_version", "modpack_version", "minecraft_version",
                                    "name", "subtitle", "description", "icon", "uuid",
                                    "loader", "mods", "version"
                                ];
                                
                                for field in required_fields {
                                    if !obj.contains_key(field) {
                                        return Err(format!("universal.json is missing required field: {}", field));
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            return Err(format!("universal.json is not valid JSON: {}", e));
                        }
                    }
                    
                    // Now try to deserialize into our struct
                    match serde_json::from_str::<UniversalManifest>(&json_text) {
                        Ok(_) => {
                            debug!("universal.json successfully validates against UniversalManifest struct");
                            Ok(())
                        },
                        Err(e) => {
                            Err(format!("universal.json does not match UniversalManifest struct: {}", e))
                        }
                    }
                },
                Err(e) => {
                    Err(format!("Failed to read universal.json: {}", e))
                }
            }
        },
        Err(e) => {
            Err(format!("Failed to fetch universal.json: {}", e))
        }
    }
}

// Default URL for the universal manifest
const DEFAULT_UNIVERSAL_URL: &str = "https://raw.githubusercontent.com/Olinus10/installer-test/master/src/data/universal.json";
const DEFAULT_PRESETS_URL: &str = "https://raw.githubusercontent.com/Olinus10/installer-test/master/src/data/presets.json";

// Load the universal manifest from a URL
pub async fn load_universal_manifest(http_client: &CachedHttpClient, url: Option<&str>) -> Result<UniversalManifest, ManifestError> {
    let manifest_url = url.unwrap_or("https://raw.githubusercontent.com/Olinus10/installer-test/master/universal.json");
    debug!("Loading universal manifest from: {}", manifest_url);
    
    // Add retry logic for more reliability
    let mut retries = 0;
    const MAX_RETRIES: usize = 3;
    
    loop {
        match http_client.get_async(manifest_url).await {
            Ok(mut response) => {
                if response.status() != StatusCode::OK {
                    let status = response.status();
                    error!("Failed to fetch universal manifest: HTTP {}", status);
                    
                    if retries < MAX_RETRIES && (status.as_u16() >= 500 || status.as_u16() == 429) {
                        retries += 1;
                        debug!("Retrying request ({}/{})", retries, MAX_RETRIES);
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * retries as u64)).await;
                        continue;
                    }
                    
                    return Err(ManifestError {
                        message: format!("Failed to fetch universal manifest: HTTP {}", status),
                        error_type: ManifestErrorType::NetworkError,
                        file_name: "universal.json".to_string(),
                        raw_content: None,
                    });
                }
                
                // Get text as String to avoid the unsized str error
                match response.text().await {
                    Ok(manifest_json) => {
                        // Store the raw JSON for debugging
                        let raw_content = Some(manifest_json.clone());
                        
                        // Try parsing as regular JSON first to catch syntax errors
                        if let Err(json_err) = serde_json::from_str::<serde_json::Value>(&manifest_json) {
                            return Err(ManifestError {
                                message: format!("Invalid JSON syntax: {}", json_err),
                                error_type: ManifestErrorType::SyntaxError,
                                file_name: "universal.json".to_string(),
                                raw_content,
                            });
                        }
                        
                        // Parse the universal manifest
                        match serde_json::from_str::(&manifest_json) {
    Ok(manifest) => {
        debug!("Successfully loaded universal manifest for {}", manifest.name);
        return Ok(manifest);
    },
    Err(e) => {
        error!("Failed to parse universal manifest JSON: {}", e);
        
        return Err(ManifestError {
            message: format!("Failed to parse universal manifest: {}", e),
            error_type: ManifestErrorType::DeserializationError,
            file_name: "universal.json".to_string(),
            raw_content,
        });
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
                        
                        Err(ManifestError {
                            message: format!("Failed to read universal manifest: {}", e),
                            error_type: ManifestErrorType::NetworkError,
                            file_name: "universal.json".to_string(),
                            raw_content: None,
                        })
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
                
                Err(ManifestError {
                    message: format!("Failed to fetch universal manifest: {}", e),
                    error_type: ManifestErrorType::NetworkError,
                    file_name: "universal.json".to_string(),
                    raw_content: None,
                })
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
