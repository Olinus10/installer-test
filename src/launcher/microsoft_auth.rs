use tokio::runtime::Runtime;
use log::{debug, error, info}; // Remove unused 'warn'
use std::error::Error;

// Re-export the main authentication module from root
pub use crate::microsoft_auth_impl::MicrosoftAuth as InnerMicrosoftAuth;

// Wrapper struct with user-friendly methods
pub struct MicrosoftAuth;

impl MicrosoftAuth {
    // Launch Minecraft with Microsoft authentication
    pub fn launch_minecraft(profile_id: &str) -> Result<(), Box<dyn Error>> {
        // Create a runtime for the async code
        let rt = Runtime::new()?;
        
        // Execute the async authentication and launch
        rt.block_on(async {
            InnerMicrosoftAuth::launch_minecraft(profile_id).await
        })
    }
    
    // Manually trigger authentication
    pub fn authenticate() -> Result<(), Box<dyn Error>> {
        // Create a runtime for the async code
        let rt = Runtime::new()?;
        
        // Execute the async authentication
        rt.block_on(async {
            match InnerMicrosoftAuth::authenticate().await {
                Ok(auth_info) => {
                    info!("Successfully authenticated as {}", auth_info.username);
                    Ok(())
                },
                Err(e) => Err(e)
            }
        })
    }
    
    // Check if the user is already authenticated
    pub fn is_authenticated() -> bool {
        // Create a runtime for the async code
        if let Ok(rt) = Runtime::new() {
            rt.block_on(async {
                if let Some(auth_info) = InnerMicrosoftAuth::load_auth_info() {
                    // Check if token is still valid 
                    if auth_info.expires_at > chrono::Utc::now() {
                        debug!("User is already authenticated as {}", auth_info.username);
                        return true;
                    }
                    
                    // Try refreshing the token
                    match InnerMicrosoftAuth::refresh_token(&auth_info.refresh_token).await {
                        Ok(_) => {
                            debug!("Successfully refreshed authentication token");
                            return true;
                        }
                        Err(e) => {
                            debug!("Failed to refresh token: {}", e);
                            return false;
                        }
                    }
                }
                
                debug!("User is not authenticated");
                false
            })
        } else {
            error!("Failed to create Tokio runtime for authentication check");
            false
        }
    }
    
    // Get the currently authenticated username (if any)
    pub fn get_username() -> Option<String> {
        // Create a runtime for the async code
        if let Ok(rt) = Runtime::new() {
            rt.block_on(async {
                if let Some(auth_info) = InnerMicrosoftAuth::load_auth_info() {
                    // Check if token is still valid
                    if auth_info.expires_at > chrono::Utc::now() {
                        return Some(auth_info.username);
                    }
                    
                    // Try refreshing the token
                    match InnerMicrosoftAuth::refresh_token(&auth_info.refresh_token).await {
                        Ok(refreshed) => Some(refreshed.username),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            })
        } else {
            error!("Failed to create Tokio runtime for username check");
            None
        }
    }
}
