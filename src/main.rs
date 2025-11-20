use anyhow::{Context, Result};
use notify_rust::Notification;
use std::collections::HashMap;
use std::process::{Child, Command};
use std::time::Duration;
use thirtyfour::prelude::*;
use tokio::time::sleep;

// ============================================================================
// EMBEDDED CONFIGURATION - Loaded at compile time from .env file
// ============================================================================
// These values are read from .env during compilation (via build.rs) and
// embedded directly into the binary. The .env file is in .gitignore,
// so credentials are never committed to GitHub.
const ROUTER_IP: &str = env!("EMBEDDED_ROUTER_IP");
const ROUTER_PASSWORD: &str = env!("EMBEDDED_ROUTER_PASSWORD");
const PPPOE_CREDENTIALS: &str = env!("EMBEDDED_PPPOE_CREDENTIALS");
// ============================================================================

/// Start ChromeDriver as a subprocess
///
/// # Returns
/// * A Child process handle for ChromeDriver
fn start_chromedriver() -> Result<Child> {
    println!("Starting ChromeDriver...");
    
    // Use different executable name based on platform
    #[cfg(target_os = "windows")]
    let chromedriver_cmd = "chromedriver.exe";
    
    #[cfg(not(target_os = "windows"))]
    let chromedriver_cmd = "chromedriver";
    
    let child = Command::new(chromedriver_cmd)
        .arg("--port=9515")
        .spawn()
        .context("Failed to start ChromeDriver. Make sure it's installed.")?;
    
    // Give ChromeDriver a moment to start up
    std::thread::sleep(Duration::from_secs(2));
    println!("ChromeDriver started successfully on port 9515");
    
    Ok(child)
}

/// Stop ChromeDriver subprocess
///
/// # Arguments
/// * `child` - The ChromeDriver process handle
fn stop_chromedriver(mut child: Child) {
    println!("Stopping ChromeDriver...");
    let _ = child.kill();
    let _ = child.wait();
    println!("ChromeDriver stopped");
}

/// Send a desktop notification
///
/// # Arguments
/// * `title` - The notification title
/// * `message` - The notification message
fn send_notification(title: &str, message: &str) {
    // Convert to owned strings before spawning thread
    let title = title.to_string();
    let message = message.to_string();
    
    // Try to send notification in a separate thread to prevent blocking on Windows
    std::thread::spawn(move || {
        let _ = Notification::new()
            .summary(&title)
            .body(&message)
            .appname("Auto WiFi Manager")
            .timeout(5000) // 5 seconds
            .show();
    });
    
    // Give the notification thread a moment to start (prevents race condition)
    std::thread::sleep(Duration::from_millis(100));
}

/// Log in to the portal and retrieve the Total Use value.
///
/// # Arguments
/// * `username` - The username for login
/// * `password` - The password for login
///
/// # Returns
/// * The total use value as an integer (e.g., 3577 for "3577 Minute")
async fn get_total_use(username: &str, password: &str) -> Result<i32> {
    // Configure Chrome to run in headless mode
    let mut caps = DesiredCapabilities::chrome();
    caps.add_arg("--headless=new")?;
    caps.add_arg("--no-sandbox")?;
    caps.add_arg("--disable-dev-shm-usage")?;

    let driver = WebDriver::new("http://localhost:9515", caps)
        .await
        .context("Failed to connect to ChromeDriver. Is it running on port 9515?")?;

    // Navigate to login page
    driver
        .goto("http://10.220.20.12/index.php/home/login")
        .await?;

    // Find and fill in login fields
    let username_field = driver
        .query(By::Name("username"))
        .first()
        .await
        .context("Username field not found")?;

    let password_field = driver
        .query(By::Name("password"))
        .first()
        .await
        .context("Password field not found")?;

    username_field.send_keys(username).await?;
    password_field.send_keys(password).await?;

    // Try to find and click the sign-in button
    let sign_in_result = driver
        .query(By::Css("button[type='submit'], input[type='submit']"))
        .first()
        .await;

    match sign_in_result {
        Ok(button) => {
            if let Err(_) = button.click().await {
                // If click fails, submit via ENTER
                password_field.send_keys(Key::Enter).await?;
            }
        }
        Err(_) => {
            // No submit button found, use ENTER
            password_field.send_keys(Key::Enter).await?;
        }
    }

    // Wait for the post-login page to load
    sleep(Duration::from_secs(2)).await;

    // Find the "Total Use:" row and extract the value
    let total_use_cell = driver
        .query(By::XPath(
            "//td[contains(text(), 'Total Use:')]/following-sibling::td[1]",
        ))
        .first()
        .await
        .context("Total Use cell not found")?;

    let total_use_value = total_use_cell.text().await?;

    // Parse the numeric value from the string (e.g., "3577 Minute" -> 3577)
    let parts: Vec<&str> = total_use_value.split_whitespace().collect();
    if parts.is_empty() {
        anyhow::bail!("Could not parse Total Use value: {}", total_use_value);
    }

    let amount_str = parts[0].replace(',', "");
    let amount = amount_str
        .parse::<i32>()
        .context(format!("Failed to parse amount: {}", amount_str))?;

    // Close the browser
    driver.quit().await?;

    Ok(amount)
}

/// Change the PPPoE password on the router.
///
/// # Arguments
/// * `router_ip` - The IP address of the router
/// * `router_password` - The admin password for the router
/// * `pppoe_id_name` - The PPPoE ID username
/// * `pppoe_id_password` - The new PPPoE ID password
///
/// # Returns
/// * `true` if the password change was successful, `false` otherwise
async fn password_change_router(
    router_ip: &str,
    router_password: &str,
    pppoe_id_name: &str,
    pppoe_id_password: &str,
) -> Result<bool> {
    // Configure Chrome to run in headless mode
    let mut caps = DesiredCapabilities::chrome();
    caps.add_arg("--headless=new")?;
    caps.add_arg("--no-sandbox")?;
    caps.add_arg("--disable-dev-shm-usage")?;

    let driver = WebDriver::new("http://localhost:9515", caps)
        .await
        .context("Failed to connect to ChromeDriver")?;

    // Navigate to router login page
    driver
        .goto(&format!("http://{}/info/Login.html", router_ip))
        .await?;

    // Login to router
    let password_field = driver
        .query(By::Id("admin_Password"))
        .first()
        .await
        .context("Router password field not found")?;

    password_field.send_keys(router_password).await?;

    let login_button = driver
        .query(By::Id("logIn_btn"))
        .first()
        .await
        .context("Login button not found")?;

    login_button.click().await?;

    // Wait for login to complete
    sleep(Duration::from_secs(2)).await;

    // Navigate to PPPoE settings page
    driver
        .goto(&format!("http://{}/Internet.html", router_ip))
        .await?;

    // Find and fill in the PPPoE ID and password fields
    let pppoe_id_field = driver
        .query(By::Name("userName_PPPoE"))
        .first()
        .await
        .context("PPPoE username field not found")?;

    sleep(Duration::from_secs(2)).await;

    let pppoe_password_field = driver
        .query(By::Name("password_PPPoE"))
        .first()
        .await
        .context("PPPoE password field not found")?;

    pppoe_id_field.clear().await?;
    pppoe_id_field.send_keys(pppoe_id_name).await?;

    sleep(Duration::from_secs(2)).await;

    pppoe_password_field.clear().await?;
    println!("done_first");
    pppoe_password_field.send_keys(pppoe_id_password).await?;

    // Submit the changes
    let submit_button = driver
        .query(By::Id("Save_btn"))
        .first()
        .await
        .context("Submit button not found")?;

    submit_button.click().await?;

    // Wait for router to apply changes and reconnect
    sleep(Duration::from_secs(35)).await;

    // Note: Not closing driver here to match Python behavior
    // driver.quit().await?;

    Ok(true)
}

/// Check which PPPoE ID is currently running on the router.
///
/// # Arguments
/// * `router_ip` - The IP address of the router
/// * `router_password` - The admin password for the router
///
/// # Returns
/// * The PPPoE ID currently in use as a string
async fn which_pppoe_id_running(router_ip: &str, router_password: &str) -> Result<String> {
    // Configure Chrome to run in headless mode
    let mut caps = DesiredCapabilities::chrome();
    caps.add_arg("--headless=new")?;
    caps.add_arg("--no-sandbox")?;
    caps.add_arg("--disable-dev-shm-usage")?;

    let driver = WebDriver::new("http://localhost:9515", caps)
        .await
        .context("Failed to connect to ChromeDriver")?;

    // Navigate to router login page
    driver
        .goto(&format!("http://{}/info/Login.html", router_ip))
        .await?;

    // Login to router
    let password_field = driver
        .query(By::Id("admin_Password"))
        .first()
        .await
        .context("Router password field not found")?;

    password_field.send_keys(router_password).await?;

    let login_button = driver
        .query(By::Id("logIn_btn"))
        .first()
        .await
        .context("Login button not found")?;

    login_button.click().await?;

    // Wait for login to complete
    sleep(Duration::from_secs(2)).await;

    // Navigate to status page
    driver
        .goto(&format!("http://{}/Internet.html", router_ip))
        .await?;

    // Wait for page to fully load
    sleep(Duration::from_secs(2)).await;

    // Find the PPPoE ID field and get its value
    let pppoe_id_field = driver
        .query(By::Name("userName_PPPoE"))
        .first()
        .await
        .context("PPPoE username field not found")?;

    // Get the current PPPoE ID value
    let current_pppoe_id = pppoe_id_field
        .value()
        .await?
        .unwrap_or_default();

    // Close the browser
    driver.quit().await?;

    Ok(current_pppoe_id.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Configuration is embedded at compile time from .env file via build.rs
    // No need to load .env at runtime
    
    // Start ChromeDriver
    let chromedriver_process = start_chromedriver()?;
    
    // Ensure ChromeDriver is stopped when the program exits
    let result = run_automation().await;
    
    // Stop ChromeDriver
    stop_chromedriver(chromedriver_process);
    
    result
}

/// Main automation logic
async fn run_automation() -> Result<()> {
    // Use embedded configuration (compiled into binary from .env file)
    let router_ip = ROUTER_IP;
    let router_password = ROUTER_PASSWORD;
    
    // Use embedded PPPoE credentials
    let pppoe_credentials_str = PPPOE_CREDENTIALS;
    
    // Parse PPPoE credentials (format: "id1:pass1,id2:pass2,...")
    let mut pppoe_id_pass: HashMap<String, String> = HashMap::new();
    for pair in pppoe_credentials_str.split(',') {
        let parts: Vec<&str> = pair.trim().split(':').collect();
        if parts.len() == 2 {
            pppoe_id_pass.insert(parts[0].to_string(), parts[1].to_string());
        } else {
            anyhow::bail!("Invalid PPPOE_CREDENTIALS format in .env file. Expected 'id1:pass1,id2:pass2,...'");
        }
    }

    // Convert to vector to enable cycling through IDs
    let pppoe_ids: Vec<(String, String)> = pppoe_id_pass
        .into_iter()
        .map(|(k, v)| (k, v))
        .collect();

    // Check which PPPoE ID is currently running
    let current_running_id = which_pppoe_id_running(&router_ip, &router_password).await?;
    println!(
        "Currently running PPPoE ID from router: '{}'",
        current_running_id
    );

    // Find the currently running ID and check its usage
    for (index, (pppoe_id_name, pppoe_id_password)) in pppoe_ids.iter().enumerate() {
        println!(
            "Checking if '{}' == '{}'",
            current_running_id, pppoe_id_name
        );

        if current_running_id == *pppoe_id_name {
            println!("✓ PPPoE ID '{}' is currently running.", pppoe_id_name);

            let current_usage = get_total_use(pppoe_id_name, pppoe_id_password).await?;
            println!("Current usage: {} minutes", current_usage);

            // Thresholds
            const SWITCH_THRESHOLD: i32 = 10000;  // Start looking for alternatives at 9000
            const AVAILABLE_THRESHOLD: i32 = 10000;  // Consider IDs with ≤8000 as available
            const DISABLE_THRESHOLD: i32 = 11000;  // Disable connection at 11000

            if current_usage > SWITCH_THRESHOLD {
                println!(
                    "Total use exceeded for '{}' ({} > {} minutes). Looking for next available ID...",
                    pppoe_id_name, current_usage, SWITCH_THRESHOLD
                );

                // Find the next PPPoE ID with usage <= AVAILABLE_THRESHOLD
                let mut found_available_id = false;
                let mut checked_count = 0;
                let mut next_pppoe_id_name = String::new();
                let mut next_pppoe_id_password = String::new();

                // Check up to all remaining IDs in the list
                while checked_count < pppoe_ids.len() - 1 {
                    let next_index = (index + 1 + checked_count) % pppoe_ids.len();
                    let (next_id, next_pass) = &pppoe_ids[next_index];

                    println!("Checking '{}'...", next_id);

                    match get_total_use(next_id, next_pass).await {
                        Ok(next_usage) => {
                            println!("  Usage for '{}': {} minutes", next_id, next_usage);

                            if next_usage <= AVAILABLE_THRESHOLD {
                                println!(
                                    "  ✓ '{}' is available (usage: {} minutes ≤ {})",
                                    next_id, next_usage, AVAILABLE_THRESHOLD
                                );
                                found_available_id = true;
                                next_pppoe_id_name = next_id.clone();
                                next_pppoe_id_password = next_pass.clone();
                                break;
                            } else {
                                println!(
                                    "  ✗ '{}' also exceeded limit ({} minutes)",
                                    next_id, next_usage
                                );
                                checked_count += 1;
                            }
                        }
                        Err(e) => {
                            println!("  Error checking '{}': {}", next_id, e);
                            checked_count += 1;
                        }
                    }
                }

                if found_available_id {
                    println!(
                        "\nSwitching from '{}' to '{}'...",
                        pppoe_id_name, next_pppoe_id_name
                    );

                    match password_change_router(
                        &router_ip,
                        &router_password,
                        &next_pppoe_id_name,
                        &next_pppoe_id_password,
                    )
                    .await
                    {
                        Ok(true) => {
                            println!("✓ Successfully switched to '{}'.", next_pppoe_id_name);
                            send_notification(
                                "WiFi ID Switched ✓",
                                &format!(
                                    "Successfully switched from '{}' to '{}'\nOld usage: {} minutes",
                                    pppoe_id_name, next_pppoe_id_name, current_usage
                                ),
                            );
                        }
                        Ok(false) => {
                            println!("✗ Failed to switch to '{}'.", next_pppoe_id_name);
                            send_notification(
                                "WiFi Switch Failed ✗",
                                &format!(
                                    "Failed to switch from '{}' to '{}'",
                                    pppoe_id_name, next_pppoe_id_name
                                ),
                            );
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                            send_notification(
                                "WiFi Switch Error",
                                &format!("Error switching WiFi ID: {}", e),
                            );
                        }
                    }
                } else {
                    println!("\n⚠ All PPPoE IDs have exceeded the {} minute limit!", AVAILABLE_THRESHOLD);
                    
                    // If current ID has exceeded DISABLE_THRESHOLD minutes, disable PPPoE by setting dummy password
                    if current_usage > DISABLE_THRESHOLD {
                        println!("⚠ Current ID '{}' has {} minutes (>{}). Disabling PPPoE connection...", pppoe_id_name, current_usage, DISABLE_THRESHOLD);
                        
                        match password_change_router(
                            &router_ip,
                            &router_password,
                            pppoe_id_name,
                            "DISABLED_EXCEEDED_LIMIT", // Dummy password to prevent connection
                        )
                        .await
                        {
                            Ok(true) => {
                                println!("✓ PPPoE connection disabled to prevent further usage.");
                                send_notification(
                                    "PPPoE Connection Disabled 🛑",
                                    &format!(
                                        "All IDs exceeded {} min limit.\nCurrent ID '{}' has {} minutes (>{}).\nConnection disabled to prevent charges.",
                                        AVAILABLE_THRESHOLD, pppoe_id_name, current_usage, DISABLE_THRESHOLD
                                    ),
                                );
                            }
                            Ok(false) | Err(_) => {
                                println!("✗ Failed to disable PPPoE connection.");
                                send_notification(
                                    "Failed to Disable PPPoE ✗",
                                    &format!(
                                        "All IDs exceeded limit but couldn't disable connection.\nCurrent usage: {} minutes",
                                        current_usage
                                    ),
                                );
                            }
                        }
                    } else {
                        send_notification(
                            "No WiFi IDs Available ⚠",
                            &format!(
                                "All PPPoE IDs have exceeded the {} minute limit!\nCurrent ID: '{}' - {} minutes (≤{} to avoid disconnect)",
                                AVAILABLE_THRESHOLD, pppoe_id_name, current_usage, DISABLE_THRESHOLD
                            ),
                        );
                    }
                }
            } else {
                println!(
                    "✓ Total use within limit for '{}'. No action taken.",
                    pppoe_id_name
                );
                send_notification(
                    "WiFi Status OK ✓",
                    &format!(
                        "Current ID: '{}'\nUsage: {} minutes (within limit)",
                        pppoe_id_name, current_usage
                    ),
                );
            }

            break; // Exit loop once we find the currently running ID
        }
    }

    Ok(())
}
