mod config;
mod ui;
mod autostart;
mod plugins;

use std::sync::{Arc, Mutex};
use eframe::egui;
use tray_icon::{
    menu::MenuEvent,
    TrayIconBuilder,
    TrayIconEvent,
};
use crate::config::load_config;
use crate::plugins::{DeviceBatteryStatus, DevicePlugin};

struct SharedState {
    config: crate::config::AppConfig,
    active_device_ids: Vec<String>,
    device_statuses: std::collections::HashMap<String, DeviceBatteryStatus>,
    last_notified: std::collections::HashMap<String, bool>,
    request_poll: bool,
}

fn load_icon_from_memory(bytes: &[u8]) -> tray_icon::Icon {
    let image = image::load_from_memory(bytes).unwrap().into_rgba8();
    let (width, height) = image.dimensions();
    tray_icon::Icon::from_rgba(image.into_raw(), width, height).unwrap()
}

fn load_icon_from_file(path: &str) -> Result<tray_icon::Icon, String> {
    let image = image::open(path)
        .map_err(|e| format!("Failed to open image: {}", e))?
        .into_rgba8();
    let (width, height) = image.dimensions();
    tray_icon::Icon::from_rgba(image.into_raw(), width, height)
        .map_err(|e| format!("Failed to create icon: {}", e))
}

fn get_default_low_icon(dev: &crate::config::DeviceConfig) -> tray_icon::Icon {
    if dev.unique_id.starts_with("pulsar_") {
        load_icon_from_memory(include_bytes!("icons/low_mouse.png"))
    } else if dev.unique_id.starts_with("xbox_") {
        load_icon_from_memory(include_bytes!("icons/low_gamepad.png"))
    } else if dev.unique_id.starts_with("gamebuds") {
        load_icon_from_memory(include_bytes!("icons/low_buds.png"))
    } else if dev.unique_id.starts_with("keyboard") || dev.unique_id.contains("keyboard") {
        load_icon_from_memory(include_bytes!("icons/low_keyboard.png"))
    } else {
        load_icon_from_memory(include_bytes!("icons/ok.png")) // fallback
    }
}

fn trigger_notification(name: &str, percentage: u8) {
    let _ = notify_rust::Notification::new()
        .summary("Low Battery Alert")
        .body(&format!("{} is at {}% battery!", name, percentage))
        .show();
}

fn update_tray_icon_main(
    state: &Arc<Mutex<SharedState>>,
    tray_icon: &mut tray_icon::TrayIcon,
) {
    let low_devices = {
        let s = state.lock().unwrap();
        let mut low = Vec::new();
        for dev_cfg in &s.config.devices {
            if !dev_cfg.enabled { continue; }
            if let Some(status) = s.device_statuses.get(&dev_cfg.unique_id) {
                if status.is_online && status.percentage <= dev_cfg.threshold {
                    low.push(dev_cfg.clone());
                }
            }
        }
        low
    };

    static mut CYCLE_INDEX: usize = 0;

    let icon = if low_devices.is_empty() {
        load_icon_from_memory(include_bytes!("icons/ok.png"))
    } else {
        unsafe {
            if CYCLE_INDEX >= low_devices.len() {
                CYCLE_INDEX = 0;
            }
            let dev = &low_devices[CYCLE_INDEX];
            CYCLE_INDEX = (CYCLE_INDEX + 1) % low_devices.len();

            if let Some(ref path_str) = dev.low_battery_icon_path {
                let resolved_path = if !path_str.contains('\\') && !path_str.contains('/') {
                    crate::config::get_icons_dir_path()
                        .map(|dir| dir.join(path_str))
                } else {
                    Some(std::path::PathBuf::from(path_str))
                };
                
                let icon_loaded = resolved_path.and_then(|p| {
                    load_icon_from_file(&p.to_string_lossy()).ok()
                });
                
                if let Some(icon) = icon_loaded {
                    icon
                } else {
                    get_default_low_icon(dev)
                }
            } else {
                get_default_low_icon(dev)
            }
        }
    };

    let _ = tray_icon.set_icon(Some(icon));
}

fn find_our_window() -> Option<windows_sys::Win32::Foundation::HWND> {
    use std::os::windows::ffi::OsStrExt;
    let title: Vec<u16> = std::ffi::OsStr::new("BatStat Settings")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let hwnd = unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW(
            std::ptr::null(),
            title.as_ptr(),
        )
    };
    if hwnd.is_null() {
        None
    } else {
        Some(hwnd)
    }
}

fn show_settings_window() {
    if let Some(hwnd) = find_our_window() {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW,
            );
            windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
        }
    }
}

fn hide_settings_window() {
    if let Some(hwnd) = find_our_window() {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE,
            );
        }
    }
}

struct BatStatApp {
    state: Arc<Mutex<SharedState>>,
    settings_item: tray_icon::menu::MenuItem,
    exit_item: tray_icon::menu::MenuItem,
    tray_icon: Option<tray_icon::TrayIcon>,
    ui_state: Option<crate::ui::SettingsWindow>,
    visible: bool,
    last_icon_update: std::time::Instant,
    first_frame: bool,
    pending_menu_events: Arc<Mutex<Vec<MenuEvent>>>,
    pending_tray_events: Arc<Mutex<Vec<TrayIconEvent>>>,
}

impl BatStatApp {
    fn new(
        state: Arc<Mutex<SharedState>>,
        settings_item: tray_icon::menu::MenuItem,
        exit_item: tray_icon::menu::MenuItem,
        tray_icon: Option<tray_icon::TrayIcon>,
        pending_menu_events: Arc<Mutex<Vec<MenuEvent>>>,
        pending_tray_events: Arc<Mutex<Vec<TrayIconEvent>>>,
    ) -> Self {
        let visible = tray_icon.is_none();
        Self {
            state,
            settings_item,
            exit_item,
            tray_icon,
            ui_state: None,
            visible,
            last_icon_update: std::time::Instant::now(),
            first_frame: true,
            pending_menu_events,
            pending_tray_events,
        }
    }
}

impl eframe::App for BatStatApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.tray_icon.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.visible = false;
                self.ui_state = None;
                hide_settings_window();
            }
        }

        if self.first_frame {
            self.first_frame = false;
            // Hide the window immediately if we have a tray icon
            if self.tray_icon.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            } else {
                // Initialize UI state immediately in fallback window mode
                let (config, active_ids, statuses) = {
                    let s = self.state.lock().unwrap();
                    (s.config.clone(), s.active_device_ids.clone(), s.device_statuses.clone())
                };
                self.ui_state = Some(crate::ui::SettingsWindow::new(config, active_ids, statuses));
            }
        }

        // Cycle through tray icons for low devices every 3 seconds
        if self.last_icon_update.elapsed() >= std::time::Duration::from_secs(3) {
            self.last_icon_update = std::time::Instant::now();
            if let Some(ref mut tray) = self.tray_icon {
                update_tray_icon_main(&self.state, tray);
            }
        }

        // Process queued menu events
        let menu_events: Vec<MenuEvent> = {
            let mut queue = self.pending_menu_events.lock().unwrap();
            queue.drain(..).collect()
        };
        for event in menu_events {
            if event.id == self.settings_item.id() {
                self.visible = true;
                let (config, active_ids, statuses) = {
                    let s = self.state.lock().unwrap();
                    (s.config.clone(), s.active_device_ids.clone(), s.device_statuses.clone())
                };
                self.ui_state = Some(crate::ui::SettingsWindow::new(config, active_ids, statuses));
                
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else if event.id == self.exit_item.id() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }

        // Process queued tray events
        let tray_events: Vec<TrayIconEvent> = {
            let mut queue = self.pending_tray_events.lock().unwrap();
            queue.drain(..).collect()
        };
        for event in tray_events {
            match event {
                TrayIconEvent::DoubleClick { .. } => {
                    self.visible = true;
                    let (config, active_ids, statuses) = {
                        let s = self.state.lock().unwrap();
                        (s.config.clone(), s.active_device_ids.clone(), s.device_statuses.clone())
                    };
                    self.ui_state = Some(crate::ui::SettingsWindow::new(config, active_ids, statuses));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                _ => {}
            }
        }

        if self.visible {
            if let Some(ref mut ui_state) = self.ui_state {
                // Sync live state from background thread and handle manual scan request
                {
                    let mut s = self.state.lock().unwrap();
                    ui_state.active_devices = s.active_device_ids.clone();
                    ui_state.device_statuses = s.device_statuses.clone();
                    
                    // Merge newly discovered devices into UI config
                    for dev in &s.config.devices {
                        if !ui_state.config.devices.iter().any(|d| d.unique_id == dev.unique_id) {
                            ui_state.config.devices.push(dev.clone());
                        }
                    }
                    
                    if ui_state.request_poll {
                        s.request_poll = true;
                        ui_state.request_poll = false;
                    }
                }
                ui_state.update(ctx, frame);
                if ui_state.request_close {
                    // Sync main config from UI
                    {
                        let mut s = self.state.lock().unwrap();
                        s.config = ui_state.config.clone();
                    }
                    if self.tray_icon.is_none() {
                        // In fallback mode, closing the window exits the app
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    } else {
                        self.visible = false;
                        self.ui_state = None;
                        hide_settings_window();
                    }
                }
            }
        }

        // Poll events periodically when hidden
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let _ = windows_sys::Win32::System::Com::CoInitializeEx(
            std::ptr::null(),
            windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED as u32,
        );
    }

    let config = load_config();

    crate::config::setup_icons_folder();
    
    let shared_state = Arc::new(Mutex::new(SharedState {
        config: config.clone(),
        active_device_ids: Vec::new(),
        device_statuses: std::collections::HashMap::new(),
        last_notified: std::collections::HashMap::new(),
        request_poll: false,
    }));
    // Spawn background polling thread
    let state_clone = Arc::clone(&shared_state);
    std::thread::spawn(move || {
        let plugins: Vec<Box<dyn DevicePlugin>> = vec![
            Box::new(plugins::pulsar::PulsarPlugin),
            Box::new(plugins::xbox::XboxPlugin),
            Box::new(plugins::steelseries::SteelSeriesPlugin),
        ];

        let mut last_poll = std::time::Instant::now() - std::time::Duration::from_secs(3600);

        loop {
            let (polling_interval, request_poll) = {
                let mut state = state_clone.lock().unwrap();
                let req = state.request_poll;
                if req {
                    state.request_poll = false;
                }
                (state.config.polling_interval_secs, req)
            };

            if request_poll || last_poll.elapsed() >= std::time::Duration::from_secs(polling_interval) {
                last_poll = std::time::Instant::now();
                
                if let Ok(api) = hidapi::HidApi::new() {
                    let mut active_instances = Vec::new();
                    for plugin in &plugins {
                        active_instances.extend(plugin.scan(&api));
                    }

                    let mut active_ids = Vec::new();
                    let mut new_statuses = std::collections::HashMap::new();

                    for inst in &active_instances {
                        let id = inst.unique_id();
                        if !active_ids.contains(&id) {
                            active_ids.push(id.clone());
                        }

                        if !new_statuses.contains_key(&id) {
                            match inst.query_battery(&api) {
                                Ok(status) => {
                                    new_statuses.insert(id.clone(), status);
                                }
                                Err(e) => {
                                    // Fallback to last known battery status if the device is sleeping / not moving
                                    let last_status = {
                                        let s = state_clone.lock().unwrap();
                                        s.device_statuses.get(&id).cloned()
                                    };
                                    if let Some(last) = last_status {
                                        new_statuses.insert(id.clone(), last);
                                    } else {
                                        eprintln!("Failed to query battery for {}: {}", inst.default_name(), e);
                                    }
                                }
                            }
                        }
                    }

                    let mut state = state_clone.lock().unwrap();
                    state.active_device_ids = active_ids.clone();
                    state.device_statuses = new_statuses;

                    // Automatically add discovered devices to config if not present
                    let mut config_changed = false;
                    for inst in &active_instances {
                        let id = inst.unique_id();
                        if !state.config.devices.iter().any(|d| d.unique_id == id) {
                            state.config.devices.push(crate::config::DeviceConfig {
                                unique_id: id,
                                name: inst.default_name(),
                                enabled: true,
                                threshold: 20,
                                low_battery_icon_path: None,
                            });
                            config_changed = true;
                        }
                    }
                    if config_changed {
                        let _ = crate::config::save_config(&state.config);
                    }

                    // Check low-battery thresholds and send alerts
                    if state.config.enable_notifications {
                        let devices_to_check = state.config.devices.clone();
                        for dev_cfg in &devices_to_check {
                            if !dev_cfg.enabled { continue; }
                            if let Some(status) = state.device_statuses.get(&dev_cfg.unique_id).copied() {
                                if status.is_online && status.percentage <= dev_cfg.threshold {
                                    let notified = state.last_notified.get(&dev_cfg.unique_id).cloned().unwrap_or(false);
                                    if !notified {
                                        state.last_notified.insert(dev_cfg.unique_id.clone(), true);
                                        trigger_notification(&dev_cfg.name, status.percentage);
                                    }
                                } else if !status.is_online || status.percentage > dev_cfg.threshold {
                                    state.last_notified.insert(dev_cfg.unique_id.clone(), false);
                                }
                            }
                        }
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    // Run eframe Native UI Event Loop
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("BatStat Settings")
            .with_inner_size([480.0, 540.0])
            .with_resizable(false),
        ..Default::default()
    };

    let state_clone_ui = Arc::clone(&shared_state);
    
    eframe::run_native(
        "BatStat Settings",
        native_options,
        Box::new(move |cc| {
            let tray_menu = tray_icon::menu::Menu::new();
            let settings_item = tray_icon::menu::MenuItem::new("Settings", true, None);
            let exit_item = tray_icon::menu::MenuItem::new("Exit", true, None);
            let _ = tray_menu.append_items(&[&settings_item, &exit_item]);

            let default_icon = load_icon_from_memory(include_bytes!("icons/ok.png"));
            
            let tray_icon = match TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("BatStat Battery Monitor")
                .with_icon(default_icon)
                .build()
            {
                Ok(icon) => {
                    Some(icon)
                }
                Err(e) => {
                    eprintln!("WARNING: Failed to build tray icon: {:?}. Running in window fallback mode.", e);
                    None
                }
            };

            // Set up event handlers that queue events and wake the UI
            let pending_menu = Arc::new(Mutex::new(Vec::<MenuEvent>::new()));
            let pending_tray = Arc::new(Mutex::new(Vec::<TrayIconEvent>::new()));

            let settings_id = settings_item.id().clone();
            let exit_id = exit_item.id().clone();

            let menu_queue = Arc::clone(&pending_menu);
            let menu_ctx = cc.egui_ctx.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                if event.id == settings_id {
                    show_settings_window();
                } else if event.id == exit_id {
                    std::process::exit(0);
                }
                menu_queue.lock().unwrap().push(event);
                menu_ctx.request_repaint();
            }));

            let tray_queue = Arc::clone(&pending_tray);
            let tray_ctx = cc.egui_ctx.clone();
            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                if let TrayIconEvent::DoubleClick { .. } = event {
                    show_settings_window();
                }
                tray_queue.lock().unwrap().push(event);
                tray_ctx.request_repaint();
            }));

            Ok(Box::new(BatStatApp::new(
                state_clone_ui,
                settings_item,
                exit_item,
                tray_icon,
                pending_menu,
                pending_tray,
            )))
        }),
    )?;
    Ok(())
}
