use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
    #[serde(default)]
    pub enable_debug_logging: bool,
    #[serde(default)]
    pub tray_battery_channel: Option<String>,
    #[serde(default)]
    pub custom_icons_dir: Option<String>,
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
            enable_debug_logging: false,
            tray_battery_channel: None,
            custom_icons_dir: None,
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

pub fn resolve_icons_dir(config: &AppConfig) -> Option<PathBuf> {
    if let Some(ref custom) = config.custom_icons_dir {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    get_icons_dir_path()
}

pub fn get_icon_list_for(config: &AppConfig) -> Vec<String> {
    let mut list = Vec::new();
    if let Some(path) = resolve_icons_dir(config) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        let lower = name.to_ascii_lowercase();
                        if (lower.ends_with(".png") || lower.ends_with(".ico"))
                            && !name.eq_ignore_ascii_case("ok.png")
                        {
                            list.push(name);
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

pub fn setup_icons_folder_at(dir: &Path) {
    let _ = fs::create_dir_all(dir);

    let defaults = vec![
        ("ok.png", include_bytes!("icons/ok.png").to_vec()),
        ("low_mouse.png", include_bytes!("icons/low_mouse.png").to_vec()),
        ("low_gamepad.png", include_bytes!("icons/low_gamepad.png").to_vec()),
        ("low_buds.png", include_bytes!("icons/low_buds.png").to_vec()),
        ("low_keyboard.png", include_bytes!("icons/low_keyboard.png").to_vec()),
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
        ("keyboard", include_bytes!("icons/low_keyboard.png").to_vec()),
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

pub fn setup_icons_folder() {
    if let Some(dir) = get_icons_dir_path() {
        setup_icons_folder_at(&dir);
    }
}

pub fn import_icon_file(src: &Path, icons_dir: &Path) -> Result<String, String> {
    if !src.is_file() {
        return Err("Selected icon file does not exist".to_string());
    }
    let file_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid icon filename".to_string())?;
    let lower = file_name.to_ascii_lowercase();
    if !lower.ends_with(".png") && !lower.ends_with(".ico") {
        return Err("Icon must be a .png or .ico image".to_string());
    }

    // Verify the file actually decodes as a valid PNG or ICO image before importing
    image::open(src).map_err(|_| "Selected file is not a valid PNG or ICO image".to_string())?;

    fs::create_dir_all(icons_dir)
        .map_err(|e| format!("Failed to create icons directory: {}", e))?;
    let dest = icons_dir.join(file_name);

    let same_file = match (src.canonicalize(), dest.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => src == dest,
    };

    if !same_file {
        fs::copy(src, &dest)
            .map_err(|e| format!("Failed to copy icon into icons folder: {}", e))?;
    }

    Ok(file_name.to_string())
}

#[cfg(test)]
mod tests {

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

        let keyboard_grid = &[
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 1 1 1 1 1 1 1 1 1 1 1 1 1 1 0",
            "0 1 1 1 1 1 1 1 1 1 1 1 1 1 1 0",
            "0 1 0 1 0 1 0 1 0 1 0 1 0 1 1 0",
            "0 1 1 0 1 0 1 0 1 0 1 0 1 0 1 0",
            "0 1 0 1 0 1 0 1 0 1 0 1 0 1 1 0",
            "0 1 1 1 1 0 0 0 0 0 1 1 1 1 1 0",
            "0 1 1 1 1 1 1 1 1 1 1 1 1 1 1 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
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
        std::fs::write(base_dir.join("low_keyboard.png"), generate_pixel_icon(keyboard_grid)).unwrap();
    }

    #[test]
    fn test_removal_simulation() {
        let mut config = super::AppConfig::default();
        config.devices.push(super::DeviceConfig {
            unique_id: "xbox_slot_0".to_string(),
            name: "Xbox Controller (Slot 0)".to_string(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        });
        
        assert_eq!(config.devices.len(), 1);
        config.devices.remove(0);
        assert_eq!(config.devices.len(), 0);
    }

    #[test]
    fn test_config_deserialization_defaults() {
        let toml_content = r#"
            polling_interval_secs = 45
            autostart = true
            enable_notifications = false
            devices = []
        "#;
        let config: super::AppConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.polling_interval_secs, 45);
        assert_eq!(config.autostart, true);
        assert_eq!(config.enable_notifications, false);
        assert_eq!(config.enable_debug_logging, false);
    }

    #[test]
    fn test_custom_icons_dir_resolution_and_serde_backcompat() {
        let legacy_toml = r#"
            polling_interval_secs = 60
            autostart = false
            devices = []
        "#;
        let mut cfg: super::AppConfig = toml::from_str(legacy_toml).unwrap();
        assert_eq!(cfg.custom_icons_dir, None);
        assert_eq!(super::resolve_icons_dir(&cfg), super::get_icons_dir_path());

        cfg.custom_icons_dir = Some("C:\\CustomIcons\\BatStat".to_string());
        assert_eq!(
            super::resolve_icons_dir(&cfg),
            Some(std::path::PathBuf::from("C:\\CustomIcons\\BatStat"))
        );
    }

    #[test]
    fn test_setup_icons_folder_seeds_missing_without_overwriting_existing() {
        let temp = std::env::temp_dir().join(format!("batstat_test_icons_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        let custom_bytes = b"custom-user-icon-bytes";
        std::fs::write(temp.join("low_mouse_red.png"), custom_bytes).unwrap();

        super::setup_icons_folder_at(&temp);

        // Existing customized file must NOT be overwritten
        assert_eq!(std::fs::read(temp.join("low_mouse_red.png")).unwrap(), custom_bytes);
        // Missing built-ins and variants must be seeded
        assert!(temp.join("ok.png").exists());
        assert!(temp.join("low_gamepad_blue.png").exists());

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_import_custom_icon_validates_extension_and_copies_to_icons_dir() {
        let temp = std::env::temp_dir().join(format!("batstat_import_test_{}", std::process::id()));
        let src_dir = temp.join("external");
        let icons_dir = temp.join("icons");
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&icons_dir).unwrap();

        let bad_file = src_dir.join("notes.txt");
        std::fs::write(&bad_file, b"nope").unwrap();
        assert!(super::import_icon_file(&bad_file, &icons_dir).is_err());

        let good_file = src_dir.join("my_mouse.PNG");
        std::fs::write(&good_file, include_bytes!("icons/low_mouse.png")).unwrap();
        let imported = super::import_icon_file(&good_file, &icons_dir).unwrap();
        assert_eq!(imported, "my_mouse.PNG");
        assert!(icons_dir.join("my_mouse.PNG").exists());

        let _ = std::fs::remove_dir_all(&temp);
    }
}
