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
    // Create a new installation from a preset
    pub fn new_from_preset(
        name: String,
        preset: &Preset,
        minecraft_version: String,
        loader_type: String,
        loader_version: String,
        launcher_type: String,
        universal_version: String,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        
        info!("Creating new installation '{}' with ID: {}", name, id);
        
        // Generate installation path based on ID
        let installations_dir = get_installations_dir();
        let installation_path = installations_dir.join(&id);
        
        // Use preset's recommended settings or defaults
        let memory_allocation = preset.recommended_memory.unwrap_or(3072); // 3GB default
        let java_args = preset.recommended_java_args.clone().unwrap_or_else(|| 
            "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M".to_string()
        );
        
        Self {
            id,
            name,
            created_at: now,
            last_used: now,
            minecraft_version,
            loader_type,
            loader_version,
            installation_path,
            enabled_features: preset.enabled_features.clone(),
            memory_allocation,
            java_args,
            linked_account_id: None,
            installed: false,
            modified: false,
            update_available: false,
            launcher_type,
            universal_version,
            last_launch: None,
            total_launches: 0,
        }
    }
    
    // Custom installation without using a preset
    pub fn new_custom(
        name: String,
        minecraft_version: String,
        loader_type: String,
        loader_version: String,
        launcher_type: String,
        universal_version: String,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        
        info!("Creating new custom installation '{}' with ID: {}", name, id);
        
        // Generate installation path based on ID
        let installations_dir = get_installations_dir();
        let installation_path = installations_dir.join(&id);
        
        Self {
            id,
            name,
            created_at: now,
            last_used: now,
            minecraft_version,
            loader_type,
            loader_version,
            installation_path,
            enabled_features: vec!["default".to_string()],
            memory_allocation: 3072, // 3GB default
            java_args: "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M".to_string(),
            linked_account_id: None,
            installed: false,
            modified: false,
            update_available: false,
            launcher_type,
            universal_version,
            last_launch: None,
            total_launches: 0,
        }
    }
    
    // Save method implementation
    pub fn save(&self) -> Result<(), String> {
        let installation_dir = self.installation_path.parent().ok_or_else(|| 
            "Invalid installation path".to_string()
        )?;
        
        // Create directory if it doesn't exist
        if !installation_dir.exists() {
            std::fs::create_dir_all(installation_dir)
                .map_err(|e| format!("Failed to create installation directory: {}", e))?;
        }
        
        // Save installation config
        let config_path = self.installation_path.join("installation.json");
        let installation_json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize installation: {}", e))?;
        
        std::fs::write(&config_path, installation_json)
            .map_err(|e| format!("Failed to write installation config: {}", e))?;
        
        debug!("Saved installation '{}' to {}", self.name, config_path.display());
        
        Ok(())
    }
    
    // Install or update the installation - async fix
    pub async fn install_or_update(&mut self, http_client: &CachedHttpClient) -> Result<(), String> {
        if !self.installed || self.update_available || self.modified {
            info!("Installing/updating installation '{}'", self.name);
            
            // Convert to installer profile
            let installer_profile = self.to_installer_profile(http_client);
            
            // Perform installation
            if !self.installed {
                match crate::install(&installer_profile, || {}).await {
                    Ok(_) => {
                        self.installed = true;
                        self.modified = false;
                        self.update_available = false;
                        self.last_used = chrono::Utc::now();
                        info!("Successfully installed '{}'", self.name);
                        Ok(())
                    },
                    Err(e) => {
                        error!("Failed to install '{}': {}", self.name, e);
                        Err(e)
                    }
                }
            } else {
                // Update existing installation
                match crate::update(&installer_profile, || {}).await {
                    Ok(_) => {
                        self.modified = false;
                        self.update_available = false;
                        self.last_used = chrono::Utc::now();
                        info!("Successfully updated '{}'", self.name);
                        Ok(())
                    },
                    Err(e) => {
                        error!("Failed to update '{}': {}", self.name, e);
                        Err(e)
                    }
                }
            }
        } else {
            debug!("No changes needed for installation '{}'", self.name);
            Ok(())
        }
    }
}

// Register installation function for installation.rs
pub fn register_installation(installation: &Installation) -> Result<(), String> {
    let mut index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    // Add to index if not already present
    if !index.installations.contains(&installation.id) {
        index.installations.push(installation.id.clone());
    }
    
    // If this is the first installation, make it active
    if index.active_installation.is_none() {
        index.active_installation = Some(installation.id.clone());
    }
    
    index.last_active = Some(chrono::Utc::now());
    
    save_installations_index(&index)
        .map_err(|e| format!("Failed to save installations index: {}", e))
}

// Additional index loading/saving helpers for installation.rs
pub fn load_installations_index() -> Result<InstallationsIndex, std::io::Error> {
    let index_path = get_installations_dir().join("index.json");
    
    if !index_path.exists() {
        return Ok(InstallationsIndex::default());
    }
    
    let index_json = std::fs::read_to_string(index_path)?;
    let index: InstallationsIndex = serde_json::from_str(&index_json)
        .unwrap_or_default();
    
    Ok(index)
}

pub fn save_installations_index(index: &InstallationsIndex) -> Result<(), std::io::Error> {
    let installations_dir = get_installations_dir();
    
    // Create directory if it doesn't exist
    if !installations_dir.exists() {
        std::fs::create_dir_all(&installations_dir)?;
    }
    
    let index_path = installations_dir.join("index.json");
    let index_json = serde_json::to_string_pretty(index)?;
    
    std::fs::write(index_path, index_json)
}

// Load an installation by ID
pub fn load_installation(id: &str) -> Result<Installation, String> {
    let installation_dir = get_installations_dir().join(id);
    let config_path = installation_dir.join("installation.json");
    
    if !config_path.exists() {
        return Err(format!("Installation {} not found", id));
    }
    
    let config_json = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read installation config: {}", e))?;
    
    let installation: Installation = serde_json::from_str(&config_json)
        .map_err(|e| format!("Failed to parse installation config: {}", e))?;
    
    Ok(installation)
}

// Implementation for AccountManager struct - add this to accounts.rs
impl AccountManager {
    // Add a new method
    pub fn new() -> Self {
        let accounts_dir = get_accounts_dir();
        Self {
            accounts_dir,
            accounts: Vec::new(),
            active_account_id: None,
            index: AccountsIndex::default(),
            loaded: false,
        }
    }
    
    // Add an authenticate method
    pub async fn authenticate(&mut self) -> Result<StoredAccount, Box<dyn std::error::Error>> {
        // Trigger Microsoft authentication flow
        let auth_info = crate::microsoft_auth_impl::MicrosoftAuth::authenticate().await?;
        
        // Add or update account
        let account_id = self.add_account(&auth_info)?;
        
        // Get and return the account
        match self.get_account(&account_id) {
            Some(account) => Ok(account.clone()),
            None => Err("Account not found after authentication".into()),
        }
    }
    
    // Implement get_active_account
    pub fn get_active_account(&self) -> Option<&StoredAccount> {
        if let Some(id) = &self.active_account_id {
            self.get_account(id)
        } else {
            None
        }
    }
    
    // Implement get_all_accounts
    pub fn get_all_accounts(&self) -> &[StoredAccount] {
        &self.accounts
    }
    
    // Implement set_active_account
    pub fn set_active_account(&mut self, id: &str) -> Result<(), String> {
        if !self.loaded {
            self.load_accounts()?;
        }
        
        // Verify account exists
        if self.get_account(id).is_none() {
            return Err(format!("Account {} not found", id));
        }
        
        self.active_account_id = Some(id.to_string());
        self.save_accounts()?;
        
        info!("Set active account to {}", id);
        Ok(())
    }
    
    // Implement sign_out
    pub fn sign_out(&mut self) -> Result<(), String> {
        if let Some(id) = self.active_account_id.clone() {
            // Clone the ID first to avoid the borrow conflict
            self.remove_account(&id)?;
        }
        
        Ok(())
    }
    
    // Implement load_accounts
    pub fn load_accounts(&mut self) -> Result<(), String> {
        if self.loaded {
            return Ok(());
        }
        
        debug!("Loading accounts from {}", self.accounts_dir.display());
        
        // Create accounts directory if it doesn't exist
        if !self.accounts_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&self.accounts_dir) {
                return Err(format!("Failed to create accounts directory: {}", e));
            }
        }
        
        // Load index
        let index_path = self.accounts_dir.join("index.json");
        self.index = if index_path.exists() {
            match std::fs::read_to_string(&index_path) {
                Ok(json) => match serde_json::from_str(&json) {
                    Ok(index) => index,
                    Err(e) => {
                        warn!("Failed to parse accounts index: {}", e);
                        AccountsIndex::default()
                    }
                },
                Err(e) => {
                    warn!("Failed to read accounts index: {}", e);
                    AccountsIndex::default()
                }
            }
        } else {
            AccountsIndex::default()
        };
        
        // Load each account
        for id in &self.index.accounts {
            let account_path = self.accounts_dir.join(format!("{}.json", id));
            if account_path.exists() {
                match std::fs::read_to_string(&account_path) {
                    Ok(json) => match serde_json::from_str(&json) {
                        Ok(account) => self.accounts.push(account),
                        Err(e) => warn!("Failed to parse account {}: {}", id, e),
                    },
                    Err(e) => warn!("Failed to read account {}: {}", id, e),
                }
            }
        }
        
        // Set active account
        self.active_account_id = self.index.active_account.clone();
        
        debug!("Loaded {} accounts", self.accounts.len());
        self.loaded = true;
        
        Ok(())
    }
}
