use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use log::{debug, error, info, warn};
use tokio::runtime::Runtime;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

// Get the accounts directory
fn get_accounts_dir() -> PathBuf {
    let app_data = crate::get_app_data();
    app_data.join(".WC_OVHL/accounts")
}

use crate::launcher::microsoft_auth::InnerMicrosoftAuth;
use crate::microsoft_auth_impl::AuthInfo;

lazy_static::lazy_static! {
    static ref ACCOUNT_MANAGER: Mutex<AccountManager> = Mutex::new(AccountManager::new());
}

// Initialize the account manager (call on app startup)
pub fn initialize_accounts() -> Result<(), String> {
    let mut manager = ACCOUNT_MANAGER.lock().unwrap();
    manager.load_accounts()?;
    
    // Mark initialization as complete
    crate::launcher::microsoft_auth::MicrosoftAuth::mark_initialization_complete();
    
    Ok(())
}

// Helper functions for application code to use

// Get the currently active account
pub fn get_active_account() -> Option<StoredAccount> {
    let manager = ACCOUNT_MANAGER.lock().unwrap();
    manager.get_active_account().cloned()
}

// Get all accounts
pub fn get_all_accounts() -> Vec<StoredAccount> {
    let manager = ACCOUNT_MANAGER.lock().unwrap();
    manager.get_all_accounts().to_vec()
}

// Authenticate with Microsoft (wrapper that handles the mutex)
pub async fn authenticate() -> Result<StoredAccount, String> {
    let runtime = Runtime::new().unwrap();
    
    runtime.block_on(async {
        let mut manager = ACCOUNT_MANAGER.lock().unwrap();
        match manager.authenticate().await {
            Ok(account) => Ok(account),
            Err(e) => Err(e.to_string()),
        }
    })
}

// Switch active account
pub fn switch_account(id: &str) -> Result<(), String> {
    let mut manager = ACCOUNT_MANAGER.lock().unwrap();
    manager.set_active_account(id)
}

// Sign out
pub fn sign_out() -> Result<(), String> {
    let mut manager = ACCOUNT_MANAGER.lock().unwrap();
    manager.sign_out()
}

// Check if user is authenticated
pub fn is_authenticated() -> bool {
    let manager = ACCOUNT_MANAGER.lock().unwrap();
    manager.get_active_account().is_some()
}

// Structure for a stored account
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredAccount {
    pub id: String,
    pub username: String,
    pub uuid: String,          // Minecraft UUID
    pub last_used: DateTime<Utc>,
    
    // We store only refresh token, not access token for security
    pub refresh_token: String,
    
    // Optional metadata
    pub display_name: Option<String>,  // User-defined name for the account
    pub avatar_url: Option<String>,    // Avatar URL if available
    pub last_login: Option<DateTime<Utc>>, // Last successful login
}

impl StoredAccount {
    // Create a new stored account from auth info
    pub fn from_auth_info(auth_info: &AuthInfo) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id,
            username: auth_info.username.clone(),
            uuid: auth_info.uuid.clone(),
            last_used: Utc::now(),
            refresh_token: auth_info.refresh_token.clone(),
            display_name: None,
            avatar_url: None,
            last_login: Some(Utc::now()),
        }
    }
    
    // Update from new auth info after refresh
    pub fn update_from_auth_info(&mut self, auth_info: &AuthInfo) {
        self.username = auth_info.username.clone();
        self.uuid = auth_info.uuid.clone();
        self.refresh_token = auth_info.refresh_token.clone();
        self.last_used = Utc::now();
        self.last_login = Some(Utc::now());
    }
    
    // Convert to auth info for use with minecraft launcher
    pub async fn to_auth_info(&self) -> Result<AuthInfo, Box<dyn std::error::Error>> {
        // Try to refresh the token
        InnerMicrosoftAuth::refresh_token(&self.refresh_token).await
    }
}

// Structure for accounts index file
#[derive(Debug, Deserialize, Serialize)]
struct AccountsIndex {
    accounts: Vec<String>,  // List of account IDs
    active_account: Option<String>, // Currently selected account
    last_active: Option<DateTime<Utc>>,
}

impl Default for AccountsIndex {
    fn default() -> Self {
        Self {
            accounts: Vec::new(),
            active_account: None,
            last_active: Some(Utc::now()),
        }
    }
}

// Account Manager
#[derive(Debug)]
pub struct AccountManager {
    accounts_dir: PathBuf,
    accounts: Vec<StoredAccount>,
    active_account_id: Option<String>,
    index: AccountsIndex,
    loaded: bool,
}

impl AccountManager {
    // Create a new account manager
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
    
    // Load accounts from disk
    pub fn load_accounts(&mut self) -> Result<(), String> {
        if self.loaded {
            return Ok(());
        }
        
        debug!("Loading accounts from {}", self.accounts_dir.display());
        
        // Create accounts directory if it doesn't exist
        if !self.accounts_dir.exists() {
            if let Err(e) = fs::create_dir_all(&self.accounts_dir) {
                return Err(format!("Failed to create accounts directory: {}", e));
            }
        }
        
        // Load index
        let index_path = self.accounts_dir.join("index.json");
        self.index = if index_path.exists() {
            match fs::read_to_string(&index_path) {
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
                match fs::read_to_string(&account_path) {
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
    
    // Save accounts to disk
    pub fn save_accounts(&self) -> Result<(), String> {
        if !self.loaded {
            return Err("Account manager not initialized".to_string());
        }
        
        // Create accounts directory if it doesn't exist
        if !self.accounts_dir.exists() {
            if let Err(e) = fs::create_dir_all(&self.accounts_dir) {
                return Err(format!("Failed to create accounts directory: {}", e));
            }
        }
        
        // Save each account
        for account in &self.accounts {
            let account_path = self.accounts_dir.join(format!("{}.json", account.id));
            let json = match serde_json::to_string_pretty(account) {
                Ok(json) => json,
                Err(e) => return Err(format!("Failed to serialize account {}: {}", account.id, e)),
            };
            
            if let Err(e) = fs::write(&account_path, json) {
                return Err(format!("Failed to write account {}: {}", account.id, e));
            }
        }
        
        // Update and save index
        let mut index = AccountsIndex {
            accounts: self.accounts.iter().map(|a| a.id.clone()).collect(),
            active_account: self.active_account_id.clone(),
            last_active: Some(Utc::now()),
        };
        
        let index_path = self.accounts_dir.join("index.json");
        let json = match serde_json::to_string_pretty(&index) {
            Ok(json) => json,
            Err(e) => return Err(format!("Failed to serialize accounts index: {}", e)),
        };
        
        if let Err(e) = fs::write(&index_path, json) {
            return Err(format!("Failed to write accounts index: {}", e));
        }
        
        debug!("Saved {} accounts", self.accounts.len());
        
        Ok(())
    }
    
    // Add a new account
    pub fn add_account(&mut self, auth_info: &AuthInfo) -> Result<String, String> {
        if !self.loaded {
            self.load_accounts()?;
        }
        
        let existing_account_index = self.accounts.iter().position(|a| a.username == auth_info.username);
        
        if let Some(index) = existing_account_index {
            // Update existing account
            let account = &mut self.accounts[index];
            account.update_from_auth_info(auth_info);
            account.last_used = Utc::now();
            
            // Make this the active account
            self.active_account_id = Some(account.id.clone());
            
            // Save changes
            self.save_accounts()?;
            
            info!("Updated existing account: {}", account.username);
            return Ok(account.id.clone());
        }
        
        // Create a new account
        let account = StoredAccount::from_auth_info(auth_info);
        let account_id = account.id.clone();
        
        // Add to accounts list
        self.accounts.push(account);
        
        // Make this the active account
        self.active_account_id = Some(account_id.clone());
        
        // Save changes
        self.save_accounts()?;
        
        info!("Added new account: {}", auth_info.username);
        Ok(account_id)
    }
    
    // Get account by ID
    pub fn get_account(&self, id: &str) -> Option<&StoredAccount> {
        self.accounts.iter().find(|a| a.id == id)
    }
    
    // Get account by username
    pub fn get_account_by_username(&self, username: &str) -> Option<&StoredAccount> {
        self.accounts.iter().find(|a| a.username == username)
    }
    
    // Get active account
    pub fn get_active_account(&self) -> Option<&StoredAccount> {
        if let Some(id) = &self.active_account_id {
            self.get_account(id)
        } else {
            None
        }
    }
    
    // Set active account
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
    
    // Remove an account
    pub fn remove_account(&mut self, id: &str) -> Result<(), String> {
        if !self.loaded {
            self.load_accounts()?;
        }
        
        // Remove from accounts list
        let initial_count = self.accounts.len();
        self.accounts.retain(|a| a.id != id);
        
        if self.accounts.len() == initial_count {
            return Err(format!("Account {} not found", id));
        }
        
        // If this was the active account, clear that
        if let Some(active_id) = &self.active_account_id {
            if active_id == id {
                self.active_account_id = self.accounts.first().map(|a| a.id.clone());
            }
        }
        
        // Delete account file
        let account_path = self.accounts_dir.join(format!("{}.json", id));
        if account_path.exists() {
            if let Err(e) = fs::remove_file(&account_path) {
                warn!("Failed to remove account file {}: {}", id, e);
                // Continue anyway
            }
        }
        
        // Save changes
        self.save_accounts()?;
        
        info!("Removed account {}", id);
        Ok(())
    }
    
    // Get all accounts
    pub fn get_all_accounts(&self) -> &[StoredAccount] {
        &self.accounts
    }
    
    // Authenticate with Microsoft
    pub async fn authenticate(&mut self) -> Result<StoredAccount, Box<dyn std::error::Error>> {
        // Trigger Microsoft authentication flow
        let auth_info = InnerMicrosoftAuth::authenticate().await?;
        
        // Add or update account
        let account_id = self.add_account(&auth_info)?;
        
        // Get and return the account
        match self.get_account(&account_id) {
            Some(account) => Ok(account.clone()),
            None => Err("Account not found after authentication".into()),
        }
    }
    
    // Refresh active account token
    pub async fn refresh_active_account(&mut self) -> Result<AuthInfo, Box<dyn std::error::Error>> {
        if let Some(account) = self.get_active_account() {
            // Clone refresh token to avoid borrow checker issues
            let refresh_token = account.refresh_token.clone();
            
            // Refresh the token
            let auth_info = InnerMicrosoftAuth::refresh_token(&refresh_token).await?;
            
            // Update the account with new token info
            self.add_account(&auth_info)?;
            
            Ok(auth_info)
        } else {
            Err("No active account to refresh".into())
        }
    }
    
    // Sign out the active account
    pub fn sign_out(&mut self) -> Result<(), String> {
        if let Some(id) = self.active_account_id.clone() {
            // Clone the ID first to avoid the borrow conflict
            self.remove_account(&id)?;
        }
        
        Ok(())
    }
}
