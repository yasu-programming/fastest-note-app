use std::process::Command;

fn main() {
    // Set build timestamp
    let timestamp = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
    
    // Get Git commit hash if available
    if let Ok(output) = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let git_hash = String::from_utf8_lossy(&output.stdout);
            let git_hash = git_hash.trim();
            println!("cargo:rustc-env=GIT_COMMIT={}", git_hash);
        } else {
            println!("cargo:rustc-env=GIT_COMMIT=unknown");
        }
    } else {
        println!("cargo:rustc-env=GIT_COMMIT=unknown");
    }
    
    // Get Rust compiler version
    let rustc_version = rustc_version::version().unwrap();
    println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version);
    
    // Rerun build script if Git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
}