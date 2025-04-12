// Existing imports and exports
pub mod config;
mod process;

// Correct imports
pub use config::{update_jvm_args, get_jvm_args};
pub use process::launch_modpack;

// New component modules
mod feature_card;
mod feature_category;
mod feature_filter;

// Export components
pub use feature_card::{FeatureCard, FeatureCardProps};
pub use feature_category::{FeatureCategory, FeatureCategoryProps};
pub use feature_filter::{FeatureFilter, FeatureFilterProps};

// Import log macros
use log::info;
