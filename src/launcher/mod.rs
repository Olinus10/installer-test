pub mod config;
mod process;

// Correct imports
pub use config::{update_jvm_args, get_jvm_args, update_memory_allocation, extract_memory_from_args};
pub use process::launch_modpack;

// Component modules
mod integrated_features;
mod features_tab;
mod performance_tab;
mod settings_tab;

// Export components (removing unused exports)
pub use features_tab::FeaturesTab;
pub use performance_tab::PerformanceTab;
pub use settings_tab::SettingsTab;

// Define public feature types needed by other modules
pub struct FeatureCard;
pub struct FeatureCategory;
pub struct FeatureFilter;
