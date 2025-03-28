use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::reqwest::async_http_client;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use std::error::Error;
use std::path::PathBuf;
use log::{debug, error, info, warn};
use url::Url;
use chrono::{DateTime, Duration, Utc};
use std::fs;
use std::process::Command;

// Microsoft OAuth2 configuration
const MS_CLIENT_ID: &str = "389b1b32-b5d5-43b2-bddc-84ce938d6737"; // Minecraft Launcher client ID
const MS_AUTH_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const MS_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const REDIRECT_URL: &str = "http://localhost:8000/callback";

// XBox Live endpoints
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XBOX_XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";

// Minecraft service endpoints
const MC_AUTH_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthInfo {
    pub access_token: String,
    pub refresh_token: String,
    pub mc_token: String,
    pub uuid: String,
    pub username: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XboxAuthRequest {
    properties: XboxAuthProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XboxAuthProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: String,
    #[serde(rename = "SiteName")]
    site_name: String,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XboxAuthResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DisplayClaims {
    #[serde(rename = "xui")]
    xui: Vec<Xui>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Xui {
    #[serde(rename = "uhs")]
    uhs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XstsAuthRequest {
    #[serde(rename = "Properties")]
    properties: XstsProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XstsProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: String,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MinecraftAuthRequest {
    #[serde(rename = "identityToken")]
    identity_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MinecraftAuthResponse {
    #[serde(rename = "access_token")]
    access_token: String,
    #[serde(rename = "token_type")]
    token_type: String,
    #[serde(rename = "expires_in")]
    expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
}

pub struct MicrosoftAuth;

impl MicrosoftAuth {
    // Initialize the OAuth2 client for Microsoft
    fn create_client() -> BasicClient {
        BasicClient::new(
            ClientId::new(MS_CLIENT_ID.to_string()),
            None, // No client secret for public clients
            AuthUrl::new(MS_AUTH_URL.to_string()).unwrap(),
            Some(TokenUrl::new(MS_TOKEN_URL.to_string()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new(REDIRECT_URL.to_string()).unwrap())
    }

    // Handle the full authentication flow
    pub async fn authenticate() -> Result<AuthInfo, Box<dyn Error>> {
        debug!("Starting Microsoft authentication flow");
        
        // Check if we already have valid tokens
        if let Some(auth_info) = Self::load_auth_info() {
            if auth_info.expires_at > Utc::now() {
                debug!("Using cached auth tokens for {}", auth_info.username);
                return Ok(auth_info);
            }
            
            debug!("Cached tokens expired, attempting refresh");
            match Self::refresh_token(&auth_info.refresh_token).await {
                Ok(refreshed_auth) => {
                    debug!("Successfully refreshed token for {}", refreshed_auth.username);
                    return Ok(refreshed_auth);
                },
                Err(e) => {
                    debug!("Token refresh failed: {}, starting new auth flow", e);
                }
            }
        }
        
        // Set up the PKCE challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        
        // Generate the authorization URL
        let client = Self::create_client();
        let (auth_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("XboxLive.signin".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();
        
        debug!("Opening browser for Microsoft authentication: {}", auth_url);
        // Open the browser for the user to log in
        if let Err(e) = open::that(auth_url.to_string()) {
            error!("Failed to open browser: {}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to open browser for authentication: {}", e),
            )));
        }
        
        // Start a local server to handle the redirect
        debug!("Waiting for authentication response...");
        let code = Self::wait_for_redirect(csrf_state).await?;
        
        // Exchange the authorization code for tokens
        debug!("Exchanging authorization code for tokens");
        let token_response = client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await?;
        
        debug!("Received Microsoft access token");
        
        // Get Microsoft token details
        let ms_token = token_response.access_token().secret().to_string();
        let ms_refresh_token = token_response.refresh_token()
            .ok_or("No refresh token provided")?
            .secret()
            .to_string();
        
        // Complete the authentication chain
        let auth_info = Self::complete_auth_chain(ms_token, ms_refresh_token).await?;
        
        // Save the auth info
        Self::save_auth_info(&auth_info)?;
        
        debug!("Authentication completed successfully for user: {}", auth_info.username);
        Ok(auth_info)
    }

    // Complete the authentication chain from MS token to Minecraft
    async fn complete_auth_chain(ms_token: String, ms_refresh_token: String) -> Result<AuthInfo, Box<dyn Error>> {
        // Authenticate with Xbox Live
        debug!("Authenticating with Xbox Live");
        let xbox_response = Self::authenticate_with_xbox(&ms_token).await?;
        
        // Get XSTS token
        debug!("Getting XSTS token");
        let xsts_response = Self::get_xsts_token(&xbox_response.token).await?;
        
        // Get Minecraft token
        debug!("Getting Minecraft token");
        let uhs = &xsts_response.display_claims.xui[0].uhs;
        let xsts_token = &xsts_response.token;
        let mc_response = Self::authenticate_with_minecraft(uhs, xsts_token).await?;
        
        // Get Minecraft profile
        debug!("Getting Minecraft profile");
        let profile = Self::get_minecraft_profile(&mc_response.access_token).await?;
        
        // Create auth info
        let expires_at = Utc::now() + Duration::seconds(mc_response.expires_in);
        let auth_info = AuthInfo {
            access_token: ms_token,
            refresh_token: ms_refresh_token,
            mc_token: mc_response.access_token,
            uuid: profile.id,
            username: profile.name,
            expires_at,
        };
        
        Ok(auth_info)
    }
    
    // Wait for the redirect after user logs in
    async fn wait_for_redirect(csrf_state: CsrfToken) -> Result<String, Box<dyn Error>> {
    let expected_state = csrf_state.secret().to_string();
        
        // Setup a TCP listener on localhost:8000
        let listener = TcpListener::bind("127.0.0.1:8000").await?;
        debug!("Listening for redirect on {}", REDIRECT_URL);
        
        // Create a channel to send the auth code when received
        let (code_tx, code_rx) = tokio::sync::oneshot::channel();
        let code_tx = Arc::new(Mutex::new(Some(code_tx)));
        
        // Accept connections in a loop
        let handle = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let code_tx = Arc::clone(&code_tx);
                
                tokio::spawn(async move {
                    let mut buffer = [0; 1024];
                    
                    // Read the HTTP request
                    if let Ok(n) = stream.read(&mut buffer).await {
                        let request = String::from_utf8_lossy(&buffer[..n]);
                        
                        // Check if it's the redirect we're waiting for
                        if request.starts_with("GET /callback") {
                            let params = request.lines().next().unwrap_or("").split_whitespace().nth(1).unwrap_or("");
                            let url = Url::parse(&format!("http://localhost{}", params)).ok();
                            
                            if let Some(url) = url {
                                let pairs: Vec<(String, String)> = url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();
                                
                                // Check if state matches
                                let state_param = pairs.iter().find(|(k, _)| k == "state").map(|(_, v)| v.as_str());
                                if state_param != Some(&expected_state) {
                                    debug!("State mismatch in redirect: got {:?}, expected {}", state_param, expected_state);
                                    
                                    // Send error page
                                    let error_response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Authentication Error</h1><p>Invalid state parameter.</p></body></html>";
                                    let _ = stream.write_all(error_response.as_bytes()).await;
                                    return;
                                }
                                
                                // Extract code
                                if let Some((_, code)) = pairs.iter().find(|(k, _)| k == "code") {
                                    // Send success page
                                    let success_response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Authentication Successful</h1><p>You can close this window and return to the application.</p><script>window.close();</script></body></html>";
                                    let _ = stream.write_all(success_response.as_bytes()).await;
                                    
                                    // Send the code through the channel
                                    if let Some(tx) = code_tx.lock().unwrap().take() {
                                        let _ = tx.send(code.clone());
                                    }
                                }
                            }
                        }
                    }
                });
            }
        });
        
        // Wait for the code
        let code = tokio::select! {
            code = code_rx => {
                code.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)))?
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(300)) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Authentication timed out after 5 minutes"
                )));
            }
        };
        
        // Cancel the listener task
        handle.abort();
        
        debug!("Received authorization code");
        Ok(code)
    }

    // Authenticate with Xbox Live
    async fn authenticate_with_xbox(ms_token: &str) -> Result<XboxAuthResponse, Box<dyn Error>> {
        let client = Client::new();
        
        let request_body = XboxAuthRequest {
            properties: XboxAuthProperties {
                auth_method: "RPS".to_string(),
                site_name: "user.auth.xboxlive.com".to_string(),
                rps_ticket: format!("d={}", ms_token),
            },
            relying_party: "http://auth.xboxlive.com".to_string(),
            token_type: "JWT".to_string(),
        };
        
        let response = client
            .post(XBOX_AUTH_URL)
            .json(&request_body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Xbox authentication failed: {}", error_text),
            )));
        }
        
        let xbox_response: XboxAuthResponse = response.json().await?;
        debug!("Successfully authenticated with Xbox Live");
        Ok(xbox_response)
    }

    // Get XSTS token
    async fn get_xsts_token(xbox_token: &str) -> Result<XboxAuthResponse, Box<dyn Error>> {
        let client = Client::new();
        
        let request_body = XstsAuthRequest {
            properties: XstsProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec![xbox_token.to_string()],
            },
            relying_party: "rp://api.minecraftservices.com/".to_string(),
            token_type: "JWT".to_string(),
        };
        
        let response = client
            .post(XBOX_XSTS_URL)
            .json(&request_body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            // Special handling for common XSTS errors
            if response.status().as_u16() == 401 {
                let error_text = response.text().await?;
                if error_text.contains("2148916233") {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "This account does not have an Xbox profile. Please create one at xbox.com first.",
                    )));
                } else if error_text.contains("2148916238") {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Child accounts must be added to a family by an adult before they can use Minecraft.",
                    )));
                }
                
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("XSTS authentication failed: {}", error_text),
                )));
            }
            
            let error_text = response.text().await?;
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("XSTS authentication failed: {}", error_text),
            )));
        }
        
        let xsts_response: XboxAuthResponse = response.json().await?;
        debug!("Successfully got XSTS token");
        Ok(xsts_response)
    }

    // Authenticate with Minecraft
    async fn authenticate_with_minecraft(uhs: &str, xsts_token: &str) -> Result<MinecraftAuthResponse, Box<dyn Error>> {
        let client = Client::new();
        
        let request_body = MinecraftAuthRequest {
            identity_token: format!("XBL3.0 x={};{}", uhs, xsts_token),
        };
        
        let response = client
            .post(MC_AUTH_URL)
            .json(&request_body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Minecraft authentication failed: {}", error_text),
            )));
        }
        
        let mc_response: MinecraftAuthResponse = response.json().await?;
        debug!("Successfully authenticated with Minecraft");
        Ok(mc_response)
    }

    // Get Minecraft profile
    async fn get_minecraft_profile(mc_token: &str) -> Result<MinecraftProfileResponse, Box<dyn Error>> {
        let client = Client::new();
        
        let response = client
            .get(MC_PROFILE_URL)
            .header("Authorization", format!("Bearer {}", mc_token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            // Check for common errors
            if response.status().as_u16() == 404 {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "You don't own Minecraft. Please purchase the game first.",
                )));
            }
            
            let error_text = response.text().await?;
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get Minecraft profile: {}", error_text),
            )));
        }
        
        let profile: MinecraftProfileResponse = response.json().await?;
        debug!("Successfully got Minecraft profile for: {}", profile.name);
        Ok(profile)
    }

    // Refresh an existing token
    pub async fn refresh_token(refresh_token: &str) -> Result<AuthInfo, Box<dyn Error>> {
        debug!("Attempting to refresh Microsoft token");
        
        let client = Self::create_client();
        
        // Exchange refresh token for new tokens
        let token_response = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
            .request_async(async_http_client)
            .await?;
        
        debug!("Received new Microsoft access token");
        
        // Get Microsoft token details
        let ms_token = token_response.access_token().secret().to_string();
        let ms_refresh_token = token_response.refresh_token()
            .ok_or("No refresh token provided")?
            .secret()
            .to_string();
        
        // Complete the authentication chain
        let auth_info = Self::complete_auth_chain(ms_token, ms_refresh_token).await?;
        
        Self::save_auth_info(&auth_info)?;
        
        debug!("Token refresh completed successfully for user: {}", auth_info.username);
        Ok(auth_info)
    }

    // Save authentication info
    fn save_auth_info(auth_info: &AuthInfo) -> Result<(), Box<dyn Error>> {
        let auth_dir = Self::get_auth_dir()?;
        let auth_file = auth_dir.join("auth.json");
        
        let json_data = serde_json::to_string(auth_info)?;
        fs::write(auth_file, json_data)?;
        
        debug!("Saved authentication info to file");
        Ok(())
    }

    // Load saved authentication info
    pub fn load_auth_info() -> Option<AuthInfo> {
        if let Ok(auth_dir) = Self::get_auth_dir() {
            let auth_file = auth_dir.join("auth.json");
            if auth_file.exists() {
                if let Ok(file_content) = fs::read_to_string(auth_file) {
                    if let Ok(auth_info) = serde_json::from_str::<AuthInfo>(&file_content) {
                        debug!("Loaded authentication info from file");
                        return Some(auth_info);
                    }
                }
            }
        }
        
        debug!("No saved authentication info found");
        None
    }

    // Get directory for storing auth data
    fn get_auth_dir() -> Result<PathBuf, Box<dyn Error>> {
        let app_name = ".WC_OVHL";
        let base_dir = crate::get_app_data();
        
        let auth_dir = base_dir.join(app_name).join("auth");
        fs::create_dir_all(&auth_dir)?;
        
        Ok(auth_dir)
    }

    // Clear saved auth data (for logout)
    pub fn logout() -> Result<(), Box<dyn Error>> {
        debug!("Logging out - removing stored authentication data");
        
        if let Ok(auth_dir) = Self::get_auth_dir() {
            let auth_file = auth_dir.join("auth.json");
            if auth_file.exists() {
                fs::remove_file(auth_file)?;
            }
        }
        
        debug!("Logout successful");
        Ok(())
    }

    // Get version ID from profile
    fn get_profile_version(profile_id: &str, minecraft_dir: &PathBuf) -> Result<String, Box<dyn Error>> {
        let manifest_path = minecraft_dir.join(format!(".WC_OVHL/{}/manifest.json", profile_id));
        
        if !manifest_path.exists() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Manifest not found for profile {}", profile_id),
            )));
        }
        
        let manifest_content = fs::read_to_string(manifest_path)?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;
        
        // Get Minecraft version and loader information
        let minecraft_version = manifest["loader"]["minecraft_version"]
            .as_str()
            .ok_or("Missing minecraft_version in manifest")?;
        
        let loader_type = manifest["loader"]["type"]
            .as_str()
            .ok_or("Missing loader type in manifest")?;
        
        let loader_version = manifest["loader"]["version"]
            .as_str()
            .ok_or("Missing loader version in manifest")?;
        
        match loader_type {
            "fabric" => Ok(format!("fabric-loader-{}-{}", loader_version, minecraft_version)),
            "quilt" => Ok(format!("quilt-loader-{}-{}", loader_version, minecraft_version)),
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("Unsupported loader type: {}", loader_type)
            ))),
        }
    }

    // Launch Minecraft with authentication
    pub async fn launch_minecraft(profile_id: &str) -> Result<(), Box<dyn Error>> {
        debug!("Preparing to launch Minecraft with profile: {}", profile_id);
        
        // Get auth info (authenticate if needed)
        let auth_info = Self::authenticate().await?;
        
        // Build paths
        let minecraft_dir = crate::launcher::config::get_minecraft_dir();
        let game_dir = minecraft_dir.join(format!(".WC_OVHL/{}", profile_id));
        
        // Verify game directory exists
        if !game_dir.exists() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Game directory not found: {:?}", game_dir),
            )));
        }
        
        debug!("Game directory: {:?}", game_dir);
        
        // Get version ID from profile
        let version_id = Self::get_profile_version(profile_id, &minecraft_dir)?;
        debug!("Using version: {}", version_id);
        
        // Get custom JVM args if any
        let jvm_args = crate::launcher::get_jvm_args(profile_id)
            .unwrap_or_else(|_| String::from("-Xmx2G -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M"));
        
        debug!("Using JVM args: {}", jvm_args);
        
        // Check if we're on Windows
        if cfg!(target_os = "windows") {
            // Find Java executable
            let java_exec = Self::find_java_executable()?;
            debug!("Using Java executable: {}", java_exec);
            
            // Create a temporary batch file
            let script_path = std::env::temp_dir().join(format!("launch_mc_{}.bat", profile_id));
            
            // Determine assets index
            let assets_index = Self::get_assets_index(minecraft_dir.join("versions").join(&version_id).join(format!("{}.json", version_id)))?;
            debug!("Using assets index: {}", assets_index);
            
            // Write the batch file
            let batch_content = format!(
                "@echo off\r\n\
                 echo Launching Minecraft {version_id}...\r\n\
                 echo Username: {username}\r\n\
                 echo Game Directory: {game_dir}\r\n\
                 \r\n\
                 \"{java_exec}\" {jvm_args} -Djava.library.path=\"{minecraft_dir}\\versions\\{version_id}\\{version_id}-natives\" -cp \"{minecraft_dir}\\libraries\\*;{minecraft_dir}\\versions\\{version_id}\\{version_id}.jar\" net.minecraft.client.main.Main --username \"{username}\" --version \"{version_id}\" --gameDir \"{game_dir}\" --assetsDir \"{minecraft_dir}\\assets\" --assetIndex {assets_index} --uuid {uuid} --accessToken {token} --clientId {client_id} --userType msa --versionType release\r\n",
                username = auth_info.username,
                uuid = auth_info.uuid,
                token = auth_info.mc_token,
                version_id = version_id,
                game_dir = game_dir.display(),
                minecraft_dir = minecraft_dir.display(),
                java_exec = java_exec,
                jvm_args = jvm_args,
                assets_index = assets_index,
                client_id = MS_CLIENT_ID
            );
            
            // Write the batch file
            fs::write(&script_path, batch_content)?;
            debug!("Created launch script at {:?}", script_path);
            
            // Execute the batch file
            debug!("Executing launch script");
            Command::new("cmd.exe")
                .arg("/C")
                .arg("start")
                .arg("Minecraft")
                .arg("/B")
                .arg(script_path)
                .spawn()?;
            
            debug!("Minecraft launch command executed");
            Ok(())
        } else if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            // Find Java executable for Unix-like systems
            let java_exec = Self::find_java_executable()?;
            debug!("Using Java executable: {}", java_exec);
            
            // Create a temporary shell script
            let script_path = std::env::temp_dir().join(format!("launch_mc_{}.sh", profile_id));
            
            // Determine assets index
            let assets_index = Self::get_assets_index(minecraft_dir.join("versions").join(&version_id).join(format!("{}.json", version_id)))?;
            debug!("Using assets index: {}", assets_index);
            
            // Class path separator
            let cp_separator = if cfg!(target_os = "windows") { ";" } else { ":" };
            
            // Write the shell script
            let shell_content = format!(
                "#!/bin/bash\n\
                echo \"Launching Minecraft {version_id}...\"\n\
                echo \"Username: {username}\"\n\
                echo \"Game Directory: {game_dir}\"\n\
                \n\
                \"{java_exec}\" {jvm_args} \\\n\
                -Djava.library.path=\"{minecraft_dir}/versions/{version_id}/{version_id}-natives\" \\\n\
                -cp \"{minecraft_dir}/libraries/*{cp_separator}{minecraft_dir}/versions/{version_id}/{version_id}.jar\" \\\n\
                net.minecraft.client.main.Main \\\n\
                --username \"{username}\" \\\n\
                --version \"{version_id}\" \\\n\
                --gameDir \"{game_dir}\" \\\n\
                --assetsDir \"{minecraft_dir}/assets\" \\\n\
                --assetIndex {assets_index} \\\n\
                --uuid {uuid} \\\n\
                --accessToken {token} \\\n\
                --clientId {client_id} \\\n\
                --userType msa \\\n\
                --versionType release\n",
                username = auth_info.username,
                uuid = auth_info.uuid,
                token = auth_info.mc_token,
                version_id = version_id,
                game_dir = game_dir.display(),
                minecraft_dir = minecraft_dir.display(),
                java_exec = java_exec,
                jvm_args = jvm_args,
                assets_index = assets_index,
                client_id = MS_CLIENT_ID,
                cp_separator = cp_separator
            );
            
            // Write and make the shell script executable
            fs::write(&script_path, shell_content)?;
            
            // Set executable permission on Unix-like systems
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&script_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&script_path, perms)?;
            }
            
            debug!("Created launch script at {:?}", script_path);
            
            // Execute the shell script
            debug!("Executing launch script");
            if cfg!(target_os = "macos") {
                Command::new("open")
                    .arg("-a")
                    .arg("Terminal")
                    .arg(script_path)
                    .spawn()?;
            } else {
                // Linux
                Command::new("sh")
                    .arg("-c")
                    .arg(format!("x-terminal-emulator -e '{}'", script_path.display()))
                    .spawn()?;
            }
            
            debug!("Minecraft launch command executed");
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unsupported operating system",
            )))
        }
    }
    
    // Find a suitable Java executable
    fn find_java_executable() -> Result<String, Box<dyn Error>> {
        debug!("Looking for suitable Java executable");
        
        // First try using JAVA_HOME if set
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let java_exe = if cfg!(windows) {
                PathBuf::from(java_home).join("bin").join("javaw.exe")
            } else {
                PathBuf::from(java_home).join("bin").join("java")
            };
            
            if java_exe.exists() {
                debug!("Using Java from JAVA_HOME: {}", java_exe.display());
                return Ok(java_exe.to_string_lossy().to_string());
            }
        }
        
        // Try common Java paths based on OS
        if cfg!(windows) {
            // Check for bundled Minecraft runtime
            let minecraft_dir = crate::launcher::config::get_minecraft_dir();
            let potential_paths = [
                minecraft_dir.join("runtime").join("java-runtime-gamma").join("bin").join("javaw.exe"),
                minecraft_dir.join("runtime").join("jre-x64").join("bin").join("javaw.exe"),
                PathBuf::from("C:\\Program Files\\Java\\jre-1.8\\bin\\javaw.exe"),
                PathBuf::from("C:\\Program Files (x86)\\Java\\jre-1.8\\bin\\javaw.exe"),
                PathBuf::from("C:\\Program Files\\Java\\jre1.8.0_301\\bin\\javaw.exe"),
                PathBuf::from("C:\\Program Files (x86)\\Java\\jre1.8.0_301\\bin\\javaw.exe"),
                PathBuf::from("C:\\Program Files\\Java\\jre-latest\\bin\\javaw.exe"),
                PathBuf::from("C:\\Program Files (x86)\\Java\\jre-latest\\bin\\javaw.exe"),
            ];
            
            for path in potential_paths {
                if path.exists() {
                    debug!("Found Java at: {}", path.display());
                    return Ok(path.to_string_lossy().to_string());
                }
            }
            
            // Try to get from system PATH
            let output = Command::new("where")
                .arg("javaw.exe")
                .output()?;
            
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                let first_line = path.lines().next().unwrap_or("");
                if !first_line.is_empty() {
                    debug!("Found Java in PATH: {}", first_line);
                    return Ok(first_line.to_string());
                }
            }
        } else {
            // macOS and Linux
            let potential_paths = [
                "/usr/bin/java",
                "/usr/local/bin/java",
                "/opt/java/bin/java",
                "/opt/homebrew/bin/java",
            ];
            
            for path in potential_paths {
                if PathBuf::from(path).exists() {
                    debug!("Found Java at: {}", path);
                    return Ok(path.to_string());
                }
            }
            
            // Try to get from system PATH
            let output = Command::new("which")
                .arg("java")
                .output()?;
            
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                let first_line = path.lines().next().unwrap_or("");
                if !first_line.is_empty() {
                    debug!("Found Java in PATH: {}", first_line);
                    return Ok(first_line.to_string());
                }
            }
        }
        
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find a suitable Java executable. Please make sure Java is installed.",
        )))
    }
    
    // Get the assets index from the version JSON
    fn get_assets_index(version_json_path: PathBuf) -> Result<String, Box<dyn Error>> {
        debug!("Getting assets index from: {}", version_json_path.display());
        
        if !version_json_path.exists() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Version JSON file not found: {}", version_json_path.display()),
            )));
        }
        
        let version_json = fs::read_to_string(version_json_path)?;
        let version_data: serde_json::Value = serde_json::from_str(&version_json)?;
        
        let assets_index = version_data["assetIndex"]["id"]
            .as_str()
            .ok_or("Missing assetIndex.id in version JSON")?;
        
        debug!("Found assets index: {}", assets_index);
        Ok(assets_index.to_string())
    }
}
