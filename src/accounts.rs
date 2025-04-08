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
            
            // Save a copy of the account ID
            let account_id = account.id.clone();
            
            // Make this the active account
            self.active_account_id = Some(account_id.clone());
            
            // Save changes
            self.save_accounts()?;
            
            info!("Updated existing account: {}", account.username);
            return Ok(account_id);
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
    
    // Fix the save_accounts method to create a properly formatted AccountsIndex
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
        
        // Create the index (not mutable to fix compiler error)
        let index = AccountsIndex {
            accounts: self.accounts.iter().map(|a| a.id.clone()).collect(),
            active_account: self.active_account_id.clone(),
            last_active: Some(Utc::now()),
        };
        
        // Write the index
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
}

// Fix the continue inside closure issue by refactoring the AccountsPage component
#[component]
fn AccountsPage() -> Element {
    let accounts = get_all_accounts();
    let active_account = get_active_account();
    let show_login_dialog = use_signal(|| false);
    let error_message = use_signal(|| Option::<String>::None);
    
    // Generate account items for the list, skipping active account
    let other_accounts = accounts.iter()
        .filter(|account| !active_account.as_ref().map_or(false, |active| active.id == account.id))
        .collect::<Vec<_>>();
    
    rsx! {
        div { class: "accounts-container",
            h1 { "Account Management" }
            
            // Display error if any
            if let Some(error) = &*error_message.read() {
                div { class: "error-notification",
                    div { class: "error-message", "{error}" }
                    button { 
                        class: "error-close",
                        onclick: move |_| error_message.set(None),
                        "×"
                    }
                }
            }
            
            // Current active account
            div { class: "active-account-section",
                h2 { "Current Account" }
                
                if let Some(account) = active_account {
                    div { class: "active-account-card",
                        img {
                            class: "minecraft-avatar",
                            src: "https://minotar.net/avatar/{account.username}/100.png",
                            alt: "Minecraft Avatar"
                        }
                        
                        div { class: "account-info",
                            h3 { "{account.username}" }
                            
                            if let Some(name) = account.display_name {
                                p { class: "display-name", "{name}" }
                            }
                            
                            p { class: "minecraft-uuid", "UUID: {account.uuid}" }
                            
                            if let Some(last_login) = account.last_login {
                                p { class: "last-login", "Last login: {last_login.format(\"%B %d, %Y\")}" }
                            }
                        }
                        
                        button {
                            class: "sign-out-button",
                            onclick: move |_| {
                                match sign_out() {
                                    Ok(_) => {
                                        // Refresh the page to show updated account status
                                    },
                                    Err(e) => {
                                        error_message.set(Some(e));
                                    }
                                }
                            },
                            "Sign Out"
                        }
                    }
                } else {
                    div { class: "no-account-message",
                        p { "You are not currently signed in to any Microsoft account." }
                        
                        button {
                            class: "sign-in-button",
                            onclick: move |_| {
                                show_login_dialog.set(true);
                            },
                            "Sign In with Microsoft"
                        }
                    }
                }
            }
            
            // Other accounts
            if other_accounts.len() > 0 {
                div { class: "other-accounts-section",
                    h2 { "Other Accounts" }
                    
                    div { class: "accounts-list",
                        for account in other_accounts {
                            div { class: "account-list-item",
                                img {
                                    class: "minecraft-avatar-small",
                                    src: "https://minotar.net/avatar/{account.username}/50.png",
                                    alt: "Minecraft Avatar"
                                }
                                
                                div { class: "account-list-info",
                                    p { class: "account-username", "{account.username}" }
                                    
                                    if let Some(name) = &account.display_name {
                                        p { class: "account-display-name", "{name}" }
                                    }
                                }
                                
                                div { class: "account-actions",
                                    button {
                                        class: "switch-account-button",
                                        onclick: move |_| {
                                            let account_id = account.id.clone();
                                            match switch_account(&account_id) {
                                                Ok(_) => {
                                                    // Refresh the page
                                                },
                                                Err(e) => {
                                                    error_message.set(Some(e));
                                                }
                                            }
                                        },
                                        "Switch"
                                    }
                                    
                                    button {
                                        class: "remove-account-button",
                                        onclick: move |_| {
                                            // Remove account logic
                                        },
                                        "Remove"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Login dialog
            if *show_login_dialog.read() {
                LoginDialog {
                    onclose: move |_| {
                        show_login_dialog.set(false);
                    },
                    onlogin: move |result| {
                        match result {
                            Ok(_) => {
                                show_login_dialog.set(false);
                                // Refresh the page
                            },
                            Err(e) => {
                                error_message.set(Some(e));
                                show_login_dialog.set(false);
                            }
                        }
                    }
                }
            }
        }
    }
}
