use dioxus::prelude::*;

#[component]
pub fn PerformanceTab(
    memory_allocation: Signal<i32>,
    java_args: Signal<String>,
) -> Element {
    // Recommended memory values (in MB)
    let min_memory = 1024;  // 1GB
    let max_memory = 16384; // 16GB
    let step = 512;         // 512MB increments
    
    // Memory markers
    let markers = [
        ("1GB", 1024),
        ("2GB", 2048),
        ("4GB", 4096),
        ("8GB", 8192),
        ("16GB", 16384),
    ];

    // Java preset options
    let presets = [
        (
            "Optimized G1GC (Recommended)", 
            "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch -XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 -XX:G1HeapRegionSize=8M -XX:G1ReservePercent=20 -XX:G1HeapWastePercent=5 -XX:G1MixedGCCountTarget=4 -XX:InitiatingHeapOccupancyPercent=15 -XX:G1MixedGCLiveThresholdPercent=90 -XX:G1RSetUpdatingPauseTimePercent=5 -XX:SurvivorRatio=32 -XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1"
        ),
        (
            "Shenandoah GC (High-End Systems)",
            "-XX:+UseShenandoahGC -XX:ShenandoahGCHeuristics=compact -XX:+UseNUMA -XX:+AlwaysPreTouch -XX:+DisableExplicitGC"
        ),
        (
            "Default (No Arguments)",
            ""
        ),
    ];

    rsx! {
        div { class: "performance-tab",
            h2 { "Performance Settings" }
            p { "Adjust memory allocation and Java arguments to optimize performance." }
            
            div { class: "performance-section",
                // Memory Allocation
                div { class: "form-group memory-section",
                    h3 { "Memory Allocation" }
                    
                    div { class: "current-memory-display",
                        "Current allocation: "
                        span { class: "memory-value", "{memory_allocation}MB" }
                        
                        // Format as GB if >= 1024MB
                        if *memory_allocation.read() >= 1024 {
                            span { class: "memory-gb", " ({(*memory_allocation.read() as f32 / 1024.0):.1}GB)" }
                        }
                    }
                    
                    // Memory slider
                    input {
                        r#type: "range",
                        min: "{min_memory}",
                        max: "{max_memory}",
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
                            div { 
                                class: "memory-marker",
                                style: "position: relative; left: {(value - min_memory) as f32 / (max_memory - min_memory) as f32 * 100.0}%",
                                "{label}"
                            }
                        }
                    }
                    
                    p { class: "memory-recommendation",
                        "Recommended: Allocate about half of your system's available RAM, but no more than 8GB for most situations."
                    }
                }
                
                // Java Arguments
                div { class: "form-group java-args-section",
                    h3 { "Java Arguments" }
                    p { class: "java-args-description",
                        "These arguments control how Java runs Minecraft. Advanced users can customize these."
                    }
                    
                    textarea {
                        id: "java-args",
                        rows: "4",
                        value: "{java_args}",
                        oninput: move |evt| java_args.set(evt.value().clone()),
                        class: "java-args-input"
                    }
                    
                    // Preset buttons
                    div { class: "java-args-presets",
                        h4 { "Suggested Arguments" }
                        
                        div { class: "java-preset-buttons",
                            for (label, args) in presets {
                                {
                                    let args_clone = args.to_string();
                                    rsx! {
                                        button {
                                            class: "java-preset-button",
                                            onclick: move |_| java_args.set(args_clone.clone()),
                                            "{label}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
