use serde::{Deserialize, Serialize};
use log::{debug, warn};
use isahc::http::StatusCode;
use isahc::AsyncReadResponseExt;

/// Entry in the changelog
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ChangelogEntry {
    pub title: String,
    pub contents: String, 
    pub date: Option<String>,
    pub version: Option<String>,
    pub importance: Option<String>,  // "major", "minor", "bugfix"
}

/// Complete changelog structure
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Changelog {
    pub entries: Vec<ChangelogEntry>,
}

/// Load the changelog from the repository
pub async fn fetch_changelog(
    modpack_source: &str, 
    http_client: &crate::CachedHttpClient
) -> Result<Changelog, String> {
    // Point to the root directory instead of src/data
    debug!("Fetching changelog from {}{}/changelog.json", crate::GH_RAW, modpack_source);
    
    let changelog_url = format!("{}{}/changelog.json", crate::GH_RAW, modpack_source);
    
    let mut changelog_resp = match http_client.get_async(changelog_url.clone()).await {
        Ok(val) => val,
        Err(e) => {
            warn!("Failed to fetch changelog: {}", e);
            return Err(format!("Failed to fetch changelog: {}", e));
        }
    };
    
    if changelog_resp.status() != StatusCode::OK {
        warn!("Changelog returned non-200 status: {}", changelog_resp.status());
        return Err(format!("Changelog returned status: {}", changelog_resp.status()));
    }
    
    let changelog_text = match changelog_resp.text().await {
        Ok(text) => text,
        Err(e) => {
            warn!("Failed to read changelog response: {}", e);
            return Err(format!("Failed to read changelog response: {}", e));
        }
    };
    
    match serde_json::from_str::<Changelog>(&changelog_text) {
        Ok(changelog) => {
            debug!("Successfully parsed changelog with {} entries", changelog.entries.len());
            Ok(changelog)
        },
        Err(e) => {
            warn!("Failed to parse changelog: {}", e);
            Err(format!("Failed to parse changelog: {}", e))
        }
    }
}
