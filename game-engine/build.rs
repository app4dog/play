// Build script to generate TypeScript bindings from Rust types using specta
// For now, we'll keep it simple and manually create the TypeScript types
// TODO: Integrate specta type export in a future iteration

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/events.rs");
    println!("cargo:warning=TypeScript types should be manually synced for now");
    
    // Generate build timestamp for cache-busting detection
    let timestamp = std::env::var("BUILD_TIMESTAMP").unwrap_or_else(|_| {
        // Get current timestamp in RFC3339 format
        match Command::new("date").arg("+%Y-%m-%dT%H:%M:%SZ").output() {
            Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
            Err(_) => {
                // Fallback: use build time from env or current time approximation
                format!("build-{}", std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "unknown".to_string()).len())
            }
        }
    });
    
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
}