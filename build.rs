use std::fs;
use std::path::Path;

fn main() {
    // Read .env file at compile time
    let env_path = Path::new(".env");
    
    if !env_path.exists() {
        panic!(
            "\n\n❌ ERROR: .env file not found!\n\
             Please create a .env file in the project root with your credentials.\n\
             You can copy .env.example to .env and fill in your values.\n\n"
        );
    }

    // Read the .env file
    let env_content = fs::read_to_string(env_path)
        .expect("Failed to read .env file");

    // Parse the .env file and set environment variables for compilation
    let mut router_ip = None;
    let mut router_password = None;
    let mut pppoe_credentials = None;

    for line in env_content.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE pairs
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "ROUTER_IP" => router_ip = Some(value.to_string()),
                "ROUTER_PASSWORD" => router_password = Some(value.to_string()),
                "PPPOE_CREDENTIALS" => pppoe_credentials = Some(value.to_string()),
                _ => {} // Ignore unknown keys
            }
        }
    }

    // Validate that all required variables are present
    let router_ip = router_ip.expect("ROUTER_IP not found in .env file");
    let router_password = router_password.expect("ROUTER_PASSWORD not found in .env file");
    let pppoe_credentials = pppoe_credentials.expect("PPPOE_CREDENTIALS not found in .env file");

    // Set environment variables for the compilation
    // These will be available via env!() macro in the source code
    println!("cargo:rustc-env=EMBEDDED_ROUTER_IP={}", router_ip);
    println!("cargo:rustc-env=EMBEDDED_ROUTER_PASSWORD={}", router_password);
    println!("cargo:rustc-env=EMBEDDED_PPPOE_CREDENTIALS={}", pppoe_credentials);

    // Tell Cargo to rerun this build script if .env changes
    println!("cargo:rerun-if-changed=.env");
    
    println!("cargo:warning=✓ Credentials loaded from .env and embedded into binary");
}
