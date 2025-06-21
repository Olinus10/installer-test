use dioxus::prelude::*;
use tokio::time::sleep;
use std::time::Duration;
use log::{error, debug};

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
    
    // State for showing success message
    let mut show_apply_success = use_signal(|| false);
    
    // State for recommended memory
    let mut recommended_memory = use_signal(|| 4096); // Default 4GB recommendation
    
    // Try to detect system memory on component load
    use_effect(move || {
        if detected_memory.read().is_none() {
            if let Some(mem) = get_system_memory() {
                detected_memory.set(Some(mem));
                
                // Calculate recommended memory (30% of system, max 4GB)
                let thirty_percent = (mem * 30) / 100;
                let recommended = std::cmp::min(4096, thirty_percent);
                recommended_memory.set(recommended);
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
    
    // Store original value for comparison to detect changes
    let mut original_memory = use_signal(|| *memory_allocation.read());
    
    // Update original memory when component first loads
    use_effect({
        let memory_allocation = memory_allocation.clone();
        let mut original_memory = original_memory.clone();
        
        move || {
            // Set initial value only once
            if *original_memory.read() == 0 {
                original_memory.set(*memory_allocation.read());
            }
        }
    });
    
    // Calculate if memory has been changed
    let memory_changed = {
        let current = *memory_allocation.read();
        let original = *original_memory.read();
        debug!("Memory comparison: current={}, original={}, changed={}", current, original, current != original);
        current != original && original != 0  // Don't show as changed if original is uninitialized
    };
    
    // Apply memory function
    let apply_memory = {
        let installation_id = installation_id.clone();
        let memory_allocation = memory_allocation.clone();
        let java_args = java_args.clone();
        let mut show_apply_success = show_apply_success.clone();
        let mut original_memory = original_memory.clone();
        
        move |_| {
            let current_memory = *memory_allocation.read();
            let installation_id_for_update = installation_id.clone();
            
            debug!("Applying memory change: {}", current_memory);
            
            spawn(async move {
                // Load the installation
                match crate::installation::load_installation(&installation_id_for_update) {
                    Ok(mut installation) => {
                        debug!("Loaded installation for memory update: {}", installation.name);
                        
                        // Update the memory allocation
                        installation.memory_allocation = current_memory;
                        
                        // Update Java args to include the new memory setting
                        let current_args = installation.java_args.clone();
                        let mut parts: Vec<String> = current_args.split_whitespace()
                            .map(|s| s.to_string())
                            .collect();
                        
                        // Remove any existing memory arguments
                        parts.retain(|part| !part.starts_with("-Xmx") && !part.starts_with("-Xms"));
                        
                        // Add the new memory parameter
                        let memory_param = if current_memory >= 1024 {
                            format!("-Xmx{}G", current_memory / 1024)
                        } else {
                            format!("-Xmx{}M", current_memory)
                        };
                        
                        parts.push(memory_param);
                        installation.java_args = parts.join(" ");
                        
                        debug!("Updated Java args: {}", installation.java_args);
                        
                        // Save the installation
                        match installation.save() {
                            Ok(_) => {
                                debug!("Successfully saved memory changes");
                                
                                // Update the signals to reflect the change
                                java_args.set(installation.java_args.clone());
                                original_memory.set(current_memory); // Update original to new value
                                
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
    
    // Get system memory display
    let system_memory_display = match *detected_memory.read() {
        Some(mem) => format_memory_display(mem),
        None => "Unknown".to_string(),
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
    
    // Create memory markers
    let markers = vec![
        ("1 GB", 1024),
        ("2 GB", 2048),
        ("4 GB", 4096),
        ("8 GB", 8192),
    ];
    
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
                        span { class: "memory-percentage", " ({percentage:.1}% of system memory)" }
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
                        for (label, value) in markers {
                            if value <= *max_memory.read() {
                                div { 
                                    class: "memory-marker",
                                    style: {
                                        let percentage = ((value - min_memory) as f32 / (*max_memory.read() - min_memory) as f32) * 100.0;
                                        format!("left: {percentage}%;")
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
                
                // Apply button for memory changes
                div { class: "memory-apply-container",
                    button {
                        class: if memory_changed { 
                            "memory-apply-button changed" 
                        } else { 
                            "memory-apply-button" 
                        },
                        disabled: !memory_changed,
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
