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

pub fn get_icons_dir_path() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|appdata| {
        let mut path = PathBuf::from(appdata);
        path.push("BatStat");
        path.push("icons");
        path
    })
}

pub fn get_icon_list() -> Vec<String> {
    let mut list = Vec::new();
    if let Some(path) = get_icons_dir_path() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if name.ends_with(".png") || name.ends_with(".ico") {
                            // Don't show ok.png in the low battery selection dropdown
                            if name != "ok.png" {
                                list.push(name);
                            }
                        }
                    }
                }
            }
        }
    }
    list.sort();
    list
}

fn colorize_icon(default_bytes: &[u8], color: image::Rgba<u8>) -> Vec<u8> {
    let mut img = image::load_from_memory(default_bytes).unwrap().into_rgba8();
    for pixel in img.pixels_mut() {
        if pixel[3] > 0 {
            pixel[0] = color[0];
            pixel[1] = color[1];
            pixel[2] = color[2];
        }
    }
    let mut buffer = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buffer, image::ImageFormat::Png).unwrap();
    buffer.into_inner()
}

pub fn setup_icons_folder() {
    if let Some(dir) = get_icons_dir_path() {
        let _ = fs::create_dir_all(&dir);
        
        let defaults = vec![
            ("ok.png", include_bytes!("icons/ok.png").to_vec()),
            ("low_mouse.png", include_bytes!("icons/low_mouse.png").to_vec()),
            ("low_gamepad.png", include_bytes!("icons/low_gamepad.png").to_vec()),
            ("low_buds.png", include_bytes!("icons/low_buds.png").to_vec()),
        ];
        
        for (name, bytes) in defaults {
            let path = dir.join(name);
            if !path.exists() {
                let _ = fs::write(path, bytes);
            }
        }
        
        let color_variations = vec![
            ("red", image::Rgba([218, 30, 40, 255])),     // #da1e28
            ("orange", image::Rgba([247, 127, 0, 255])),  // Orange
            ("yellow", image::Rgba([252, 191, 73, 255])), // Yellow
            ("blue", image::Rgba([76, 201, 240, 255])),   // Electric Blue
        ];
        
        let templates = vec![
            ("mouse", include_bytes!("icons/low_mouse.png").to_vec()),
            ("gamepad", include_bytes!("icons/low_gamepad.png").to_vec()),
            ("buds", include_bytes!("icons/low_buds.png").to_vec()),
        ];
        
        for (color_name, rgba) in color_variations {
            for (temp_name, temp_bytes) in &templates {
                let filename = format!("low_{}_{}.png", temp_name, color_name);
                let path = dir.join(&filename);
                if !path.exists() {
                    let colorized = colorize_icon(temp_bytes, rgba);
                    let _ = fs::write(path, colorized);
                }
            }
        }
    }
}
