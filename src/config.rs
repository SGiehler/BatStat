use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceConfig {
    pub unique_id: String,
    pub name: String,
    pub enabled: bool,
    pub threshold: u8,
    pub low_battery_icon_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub polling_interval_secs: u64,
    pub autostart: bool,
    #[serde(default = "default_true")]
    pub enable_notifications: bool,
    pub devices: Vec<DeviceConfig>,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            polling_interval_secs: 60,
            autostart: false,
            enable_notifications: true,
            devices: Vec::new(),
        }
    }
}

pub fn get_config_path() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|appdata| {
        let mut path = PathBuf::from(appdata);
        path.push("BatStat");
        path.push("config.toml");
        path
    })
}

pub fn load_config() -> AppConfig {
    if let Some(path) = get_config_path() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut config) = toml::from_str::<AppConfig>(&content) {
                    let mut migrated_devices = Vec::new();
                    let mut seen_ids = std::collections::HashSet::new();

                    for mut dev in config.devices {
                        if dev.unique_id.starts_with("pulsar_") {
                            let parts: Vec<String> = dev.unique_id.split('_').map(|s| s.to_string()).collect();
                            if parts.len() > 2 {
                                dev.unique_id = format!("{}_{}", parts[0], parts[1]);
                                dev.name = format!("Pulsar Mouse ({})", parts[1]);
                            }
                        } else if dev.unique_id.starts_with("gamebuds_left") || dev.unique_id.starts_with("gamebuds_right") {
                            dev.unique_id = "gamebuds".to_string();
                            dev.name = "SteelSeries Arctis GameBuds".to_string();
                        }

                        if !seen_ids.contains(&dev.unique_id) {
                            seen_ids.insert(dev.unique_id.clone());
                            migrated_devices.push(dev);
                        }
                    }
                    config.devices = migrated_devices;
                    return config;
                }
            }
        }
    }
    AppConfig::default()
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path().ok_or("Could not locate AppData directory")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    let content = toml::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write config file: {}", e))?;
    Ok(())
}
