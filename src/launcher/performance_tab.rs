use dioxus::prelude::*;
use tokio::time::sleep;
use std::time::Duration;
use log::error;

// Helper function to get system memory
fn get_system_memory() -> Option<i32> {
    #[cfg(target_os = "windows")]
    {
        // Use wmic command on Windows
        if let Ok(output) = std::process::Command::new("wmic")
            .args(&["computersystem", "get", "TotalPhysicalMemory"])
            .output() 
        {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                // Parse the output to get total memory in bytes
                if let Some(mem_str) = output_str.lines().nth(1) {
                    if let Ok(mem_bytes) = mem_str.trim().parse::<u64>() {
                        // Convert bytes to MB
                        return Some((mem_bytes / (1024 * 1024)) as i32);
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Use /proc/meminfo on Linux
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(mem_kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(mem_kb) = mem_kb_str.parse::<u64>() {
                            // Convert KB to MB
                            return Some((mem_kb / 1024) as i32);
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // Use sysctl on macOS
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(&["-n", "hw.memsize"])
            .output() 
        {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                if let Ok(mem_bytes) = output_str.trim().parse::<u64>() {
                    // Convert bytes to MB
                    return Some((mem_bytes / (1024 * 1024)) as i32);
                }
            }
        }
    }
    
    // Default fallback
    None
}

// Format memory value for display
fn format_memory_display(memory_mb: i32) -> String {
    if memory_mb >= 1024 {
        format!("{:.1} GB", memory_mb as f32 / 1024.0)
    } else {
        format!("{} MB", memory_mb)
    }
}

#[component]
pub fn PerformanceTab(
    memory_allocation: Signal<i32>,
    java_args: Signal<String>,
    installation_id: String,
) -> Element {
    // State for system memory
    let mut detected_memory = use_signal(|| None::<i32>);
    
    // Try to detect system memory on component load
    use_effect(move || {
        if detected_memory.read().is_none() {
            if let Some(mem) = get_system_memory() {
                detected_memory.set(Some(mem));
            }
        }
    });
    
    // Default memory boundaries
    let min_memory = 1024;  // Minimum 1GB
    let max_memory = use_signal(|| 8 * 1024); // Default 8GB max
    
    // Update max memory when system memory is available
    use_effect({
        let detected_memory = detected_memory.clone();
        let mut max_memory = max_memory.clone();
        
        move || {
            if let Some(mem) = *detected_memory.read() {
                // Cap at 8GB or 70% of system memory, whichever is less
                let max_allowed = 8 * 1024; // 8GB in MB
                let seventy_percent = (mem * 70) / 100;
                max_memory.set(std::cmp::min(max_allowed, seventy_percent));
            }
        }
    });
    
    let step = 512; // 512MB steps
    
    // Get the installation from the prop that should be passed
    // This needs to be passed from the parent component
    
    // Store original value for comparison to detect changes
    let original_memory = use_memo(move || *memory_allocation.read());
    
    // State for displaying success message after apply
    let mut show_apply_success = use_signal(|| false);
    
    // Memory markers
    let markers = [
        ("1GB", 1024),
        ("2GB", 2048),
        ("4GB", 4096),
        ("8GB", 8192),
    ];
    
    // Recommended memory based on system memory
    let recommended_memory = use_signal(|| 4096); // Default 4GB
    
    // Update recommended memory when system memory is available
    use_effect({
        let detected_memory = detected_memory.clone();
        let mut recommended_memory = recommended_memory.clone();
        
        move || {
            if let Some(mem) = *detected_memory.read() {
                // Always recommend at most 4GB (4096MB)
                // Calculate half of system memory
                let half_memory = mem / 2;
                
                // Cap at 4GB max as requested
                recommended_memory.set(std::cmp::min(4 * 1024, half_memory));
            }
        }
    });
    
    // Format system memory for display
    let system_memory_display = match *detected_memory.read() {
        Some(mem) => format_memory_display(mem),
        None => "Unknown".to_string(),
    };
    
    // Handler for applying memory changes
let apply_memory = {
    let installation_id = installation_id.clone();
    let memory_allocation = memory_allocation.clone();
    let mut java_args = java_args.clone();
    let mut show_apply_success = show_apply_success.clone();
    
    move |_| {
        let current_memory = *memory_allocation.read();
        let installation_id_for_update = installation_id.clone();
        
        // Create a proper async operation
        spawn(async move {
            // Load the installation
            match crate::installation::load_installation(&installation_id_for_update) {
                Ok(mut installation) => {
                    // Update the memory allocation
                    installation.memory_allocation = current_memory;
                    
                    // Update Java args to include the new memory setting
                    let current_args = installation.java_args.clone();
                    let mut parts: Vec<&str> = current_args.split_whitespace().collect();
                    
                    // Remove any existing memory arguments
                    parts.retain(|part| !part.starts_with("-Xmx") && !part.starts_with("-Xms"));
                    
                    // Add the new memory parameter
                    let memory_param = if current_memory >= 1024 {
                        format!("-Xmx{}G", current_memory / 1024)
                    } else {
                        format!("-Xmx{}M", current_memory)
                    };
                    
                    parts.push(&memory_param);
                    installation.java_args = parts.join(" ");
                    
                    // Save the installation
                    match installation.save() {
                        Ok(_) => {
                            // Update the signal to reflect the change
                            java_args.set(installation.java_args.clone());
                            
                            // Show success message
                            show_apply_success.set(true);
                            
                            // Hide success message after 3 seconds
                            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                            show_apply_success.set(false);
                        },
                        Err(e) => {
                            error!("Failed to save installation: {}", e);
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to load installation: {}", e);
                }
            }
        });
    }
};
    
    // Calculate if memory has been changed from original
let memory_changed = {
    let current_memory = *memory_allocation.read();
    let original_memory = *original_memory.read();
    current_memory != original_memory
};
    
    // Calculate percentages safely
    let memory_percentage = match *detected_memory.read() {
        Some(sys_mem) if sys_mem > 0 => {
            Some((*memory_allocation.read() as f32 / sys_mem as f32) * 100.0)
        },
        _ => None
    };
    
    let recommended_percentage = match *detected_memory.read() {
        Some(sys_mem) if sys_mem > 0 => {
            Some((*recommended_memory.read() as f32 / sys_mem as f32) * 100.0)
        },
        _ => None
    };
    
    // Create memory markers elements
    let memory_marker_elements = markers.iter().enumerate().map(|(index, (label, value))| {
        if *value <= *max_memory.read() {
            // Calculate percentage position
            let percentage = ((*value - min_memory) as f32 / (*max_memory.read() - min_memory) as f32) * 100.0;
            
            // Apply specific adjustments based on marker position
            let margin_adjustment = match index {
                0 => "margin-left: 0%",         // First marker
                3 => "margin-left: -40px",      // Last marker (8GB)
                _ => "",                        // Middle markers
            };
            
            rsx! {
                div { 
                    key: "{label}",
                    class: "memory-marker",
                    style: "left: {percentage}%; {margin_adjustment}",
                    "{label}"
                }
            }
        } else {
            rsx! { Fragment {} }
        }
    });
    
    rsx! {
        div { class: "performance-tab",
            h2 { "Performance Settings" }
            p { "Adjust memory allocation for Minecraft to optimize performance." }
            
            div { class: "performance-section memory-section",
                h3 { "Memory Allocation" }
                
                // System memory info
                div { class: "system-memory-info",
                    "Your System Memory: ",
                    span { class: "system-memory-value", "{system_memory_display}" }
                }
                
                // Current memory display with percentage indicator
                div { class: "current-memory-display",
                    "Current allocation: ",
                    span { class: "memory-value", "{format_memory_display(*memory_allocation.read())}" }
                    
                    // Show percentage of system memory if available
                    if let Some(percentage) = memory_percentage {
                        {
                            rsx! {
                                span { class: "memory-percentage", " ({percentage:.1}% of system memory)" }
                            }
                        }
                    }
                }
                
                // Memory slider with improved design
                div { class: "memory-slider-container",
                    input {
                        r#type: "range",
                        min: "{min_memory}",
                        max: "{*max_memory.read()}",
                        step: "{step}",
                        value: "{*memory_allocation.read()}",
                        oninput: move |evt| {
                            if let Ok(value) = evt.value().parse::<i32>() {
                                memory_allocation.set(value);
                            }
                        },
                        class: "memory-slider"
                    }
                    
                    // Memory markers below slider 
                    div { class: "memory-markers",
                        span { class: "memory-marker", "1 GB" }
                        span { class: "memory-marker", "{format_memory_display(*max_memory.read())}" }
                    }
                }
                
                // Apply button for memory changes - FIXED: Enable when memory has changed
                div { class: "memory-apply-container",
                    button {
                        class: if memory_changed { 
                            "memory-apply-button changed" 
                        } else { 
                            "memory-apply-button" 
                        },
                        disabled: !memory_changed, // Only disable if no changes made
                        onclick: apply_memory,
                        "Apply Memory Changes"
                    }
                    
                    // Success message
                    if *show_apply_success.read() {
                        div { class: "apply-success-message", "Memory settings applied successfully!" }
                    }
                }
                
                p { class: "memory-recommendation",
                    {
                        if detected_memory.read().is_some() {
                            let rec_text = format!("Recommended: {}", format_memory_display(*recommended_memory.read()));
                            
                            if let Some(percentage) = recommended_percentage {
                                format!("{} (~{}% of your system memory)", rec_text, percentage as i32)
                            } else {
                                format!("{} (max 4GB)", rec_text)
                            }
                        } else {
                            "Recommended: Up to 4GB for optimal performance".to_string()
                        }
                    }
                }
            }
        }
    }
}
