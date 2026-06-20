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
            let _ = fs::write(path, bytes);
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
                let colorized = colorize_icon(temp_bytes, rgba);
                let _ = fs::write(path, colorized);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_pixel_icon(grid: &[&str]) -> Vec<u8> {
        let height = grid.len();
        let first_chars: Vec<char> = grid[0].chars().filter(|c| !c.is_whitespace()).collect();
        let width = first_chars.len();
        let mut img = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::new(width as u32, height as u32);
        for y in 0..height {
            let chars: Vec<char> = grid[y].chars().filter(|c| !c.is_whitespace()).collect();
            for x in 0..width {
                let pixel = if chars[x] == '1' {
                    image::Rgba([255u8, 255u8, 255u8, 255u8])
                } else {
                    image::Rgba([0u8, 0u8, 0u8, 0u8])
                };
                img.put_pixel(x as u32, y as u32, pixel);
            }
        }
        let mut buffer = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buffer, image::ImageFormat::Png).unwrap();
        buffer.into_inner()
    }

    #[test]
    fn generate_icons() {
        let ok_grid = &[
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 1 1 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 1 1 1 0",
            "0 0 0 0 0 0 0 0 0 0 0 1 1 1 0 0",
            "0 0 0 0 0 0 0 0 0 0 1 1 1 0 0 0",
            "0 0 0 0 0 0 0 0 0 1 1 1 0 0 0 0",
            "0 1 1 0 0 0 0 0 1 1 1 0 0 0 0 0",
            "0 1 1 1 0 0 0 1 1 1 0 0 0 0 0 0",
            "0 0 1 1 1 0 1 1 1 0 0 0 0 0 0 0",
            "0 0 0 1 1 1 1 1 0 0 0 0 0 0 0 0",
            "0 0 0 0 1 1 1 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
        ];

        let mouse_grid = &[
            "0 0 0 0 0 1 1 1 1 1 1 0 0 0 0 0",
            "0 0 0 1 1 1 1 0 0 1 1 1 1 0 0 0",
            "0 0 1 1 1 1 1 0 0 1 1 1 1 1 0 0",
            "0 0 1 1 1 1 1 1 1 1 1 1 1 1 0 0",
            "0 0 1 1 1 1 1 0 0 1 1 1 1 1 0 0",
            "0 0 1 1 1 1 1 0 0 1 1 1 1 1 0 0",
            "0 0 1 1 1 1 1 1 1 1 1 1 1 1 0 0",
            "0 0 1 1 1 1 1 1 1 1 1 1 1 1 0 0",
            "0 0 1 1 1 1 1 1 1 1 1 1 1 1 0 0",
            "0 0 1 1 1 1 1 1 1 1 1 1 1 1 0 0",
            "0 0 0 1 1 1 1 1 1 1 1 1 1 0 0 0",
            "0 0 0 1 1 1 1 1 1 1 1 1 1 0 0 0",
            "0 0 0 0 1 1 1 1 1 1 1 1 0 0 0 0",
            "0 0 0 0 0 1 1 1 1 1 1 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
        ];

        let gamepad_grid = &[
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 1 1 1 1 1 1 0 0 0 0 0",
            "0 0 1 1 1 1 1 1 1 1 1 1 1 1 0 0",
            "0 1 1 1 1 1 1 1 1 1 1 1 1 1 1 0",
            "1 1 1 0 1 1 1 1 1 1 1 1 0 1 1 1",
            "1 1 0 0 0 1 1 1 1 1 1 0 0 0 1 1",
            "1 1 1 0 1 1 1 1 1 1 1 1 0 1 1 1",
            "1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1",
            "1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1",
            "0 1 1 1 1 0 0 0 0 0 0 1 1 1 1 0",
            "0 1 1 1 0 0 0 0 0 0 0 0 1 1 1 0",
            "0 0 1 1 0 0 0 0 0 0 0 0 1 1 0 0",
            "0 0 1 1 0 0 0 0 0 0 0 0 1 1 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
        ];

        let buds_grid = &[
            "0 0 0 0 0 1 1 1 1 1 1 0 0 0 0 0",
            "0 0 0 1 1 1 1 1 1 1 1 1 1 0 0 0",
            "0 0 1 1 1 0 0 0 0 0 0 1 1 1 0 0",
            "0 1 1 1 0 0 0 0 0 0 0 0 1 1 1 0",
            "0 1 1 0 0 0 0 0 0 0 0 0 0 1 1 0",
            "0 1 1 0 0 0 0 0 0 0 0 0 0 1 1 0",
            "1 1 1 1 0 0 0 0 0 0 0 0 1 1 1 1",
            "1 1 1 1 0 0 0 0 0 0 0 0 1 1 1 1",
            "1 1 1 1 0 0 0 0 0 0 0 0 1 1 1 1",
            "1 1 1 1 0 0 0 0 0 0 0 0 1 1 1 1",
            "1 1 1 1 0 0 0 0 0 0 0 0 1 1 1 1",
            "0 1 1 1 0 0 0 0 0 0 0 0 1 1 1 0",
            "0 0 1 1 0 0 0 0 0 0 0 0 1 1 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
        ];

        let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("icons");
        std::fs::create_dir_all(&base_dir).unwrap();

        std::fs::write(base_dir.join("ok.png"), generate_pixel_icon(ok_grid)).unwrap();
        std::fs::write(base_dir.join("low_mouse.png"), generate_pixel_icon(mouse_grid)).unwrap();
        std::fs::write(base_dir.join("low_gamepad.png"), generate_pixel_icon(gamepad_grid)).unwrap();
        std::fs::write(base_dir.join("low_buds.png"), generate_pixel_icon(buds_grid)).unwrap();
    }
}
