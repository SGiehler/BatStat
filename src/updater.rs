use serde::Deserialize;
use std::io::{Read, Write};

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

pub fn is_newer(latest_tag: &str, current_version: &str) -> bool {
    let clean_latest = latest_tag.trim_start_matches('v');
    let clean_current = current_version.trim_start_matches('v');
    
    let latest_parts: Vec<&str> = clean_latest.split('.').collect();
    let current_parts: Vec<&str> = clean_current.split('.').collect();
    
    for i in 0..latest_parts.len().max(current_parts.len()) {
        let latest_val = latest_parts.get(i).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let current_val = current_parts.get(i).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        if latest_val > current_val {
            return true;
        } else if latest_val < current_val {
            return false;
        }
    }
    false
}

pub fn check_for_update() -> Result<Option<ReleaseInfo>, String> {
    let url = "https://api.github.com/repos/SGiehler/BatStat/releases/latest";
    let response: ReleaseInfo = ureq::get(url)
        .set("User-Agent", "batstat-updater")
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?
        .into_json()
        .map_err(|e| format!("JSON parsing failed: {}", e))?;
        
    let current_version = env!("CARGO_PKG_VERSION");
    if is_newer(&response.tag_name, current_version) {
        Ok(Some(response))
    } else {
        Ok(None)
    }
}

pub fn download_and_install_update<F>(url: &str, progress_callback: F) -> Result<(), String>
where
    F: Fn(f32) + Send + 'static,
{
    let response = ureq::get(url)
        .set("User-Agent", "batstat-updater")
        .call()
        .map_err(|e| format!("Failed to download update: {}", e))?;

    let total_size = response.header("Content-Length")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    let mut reader = response.into_reader();
    let mut buffer = vec![0; 4096];
    let mut bytes_downloaded = 0;
    
    let temp_dir = std::env::temp_dir();
    let msi_path = temp_dir.join("batstat_upgrade.msi");
    let mut file = std::fs::File::create(&msi_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    loop {
        let n = reader.read(&mut buffer)
            .map_err(|e| format!("Read error: {}", e))?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])
            .map_err(|e| format!("Write error: {}", e))?;
        bytes_downloaded += n;
        if total_size > 0 {
            progress_callback(bytes_downloaded as f32 / total_size as f32);
        }
    }

    // Run the MSI installer in passive mode so it runs a clean upgrade
    std::process::Command::new("msiexec")
        .arg("/i")
        .arg(&msi_path)
        .arg("/passive")
        .spawn()
        .map_err(|e| format!("Failed to launch installer: {}", e))?;

    // Exit the app so the installer can replace the executable
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer("v1.2.0", "0.1.0"));
        assert!(is_newer("1.2.0", "v0.1.0"));
        assert!(is_newer("v1.2.0", "v1.1.9"));
        assert!(!is_newer("v1.2.0", "v1.2.0"));
        assert!(!is_newer("v1.2.0", "v1.3.0"));
        assert!(is_newer("v2.0.0", "v1.9.9"));
    }
}
