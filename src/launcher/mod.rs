// Existing imports and exports
pub mod config;
mod process;

// Correct imports
pub use config::{update_jvm_args, get_jvm_args};
pub use process::launch_modpack;

// New component modules
mod integrated_features;
mod features_tab;
mod performance_tab;
mod settings_tab;

// Export components
pub use integrated_features::IntegratedFeatures;
pub use features_tab::FeaturesTab;
pub use performance_tab::PerformanceTab;
pub use settings_tab::SettingsTab;

// Import log macros
use log::info;
