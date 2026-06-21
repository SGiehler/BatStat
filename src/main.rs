#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod config;
mod ui;
mod autostart;
mod plugins;
mod updater;

use std::sync::{Arc, Mutex};
use eframe::egui;
use tray_icon::{
    menu::{MenuEvent, Menu, MenuItem, PredefinedMenuItem},
    TrayIconBuilder,
    TrayIconEvent,
};
use crate::config::load_config;
use crate::plugins::{DeviceBatteryStatus, DevicePlugin};

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available(crate::updater::ReleaseInfo),
    NoUpdate,
    Downloading(f32),
    ReadyToInstall(String),
    Error(String),
}

struct SharedState {
    config: crate::config::AppConfig,
    active_device_ids: Vec<String>,
    device_statuses: std::collections::HashMap<String, DeviceBatteryStatus>,
    last_notified: std::collections::HashMap<String, bool>,
    request_poll: bool,
    settings_item_id: Option<tray_icon::menu::MenuId>,
    exit_item_id: Option<tray_icon::menu::MenuId>,
    has_tray: bool,
    tray_thread_id: Option<u32>,
    initialized: bool,
    update_status: UpdateStatus,
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

fn update_tray_icon_and_menu_local(
    state: &Arc<Mutex<SharedState>>,
    tray: &mut tray_icon::TrayIcon,
    settings_item: &MenuItem,
    exit_item: &MenuItem,
) {
    let s = state.lock().unwrap();

    let mut low_devices = Vec::new();
    for dev_cfg in &s.config.devices {
        if !dev_cfg.enabled { continue; }
        if let Some(status) = s.device_statuses.get(&dev_cfg.unique_id) {
            if status.is_online() && status.effective_percentage() <= dev_cfg.threshold {
                low_devices.push(dev_cfg.clone());
            }
        }
    }

    // Capture device statuses map to avoid double borrowing
    let device_statuses = s.device_statuses.clone();

    static mut LAST_DISPLAYED_ID: [u8; 64] = [0; 64];
    static mut LAST_DISPLAYED_LEN: usize = 0;

    let icon = if low_devices.is_empty() {
        unsafe {
            LAST_DISPLAYED_LEN = 0;
        }
        let _ = tray.set_tooltip(Some("BatStat Battery Monitor"));
        load_icon_from_memory(include_bytes!("icons/ok.png"))
    } else {
        unsafe {
            let last_id_str = std::str::from_utf8(&LAST_DISPLAYED_ID[..LAST_DISPLAYED_LEN]).unwrap_or("");
            let index = if LAST_DISPLAYED_LEN > 0 {
                if let Some(pos) = low_devices.iter().position(|d| d.unique_id == last_id_str) {
                    (pos + 1) % low_devices.len()
                } else {
                    0
                }
            } else {
                0
            };

            let dev = &low_devices[index];
            
            let pct_str = if let Some(status) = device_statuses.get(&dev.unique_id) {
                format!("{}%", status.effective_percentage())
            } else {
                "Unknown".to_string()
            };
            let tooltip_msg = format!("Low Battery: {} ({})", dev.name, pct_str);
            let _ = tray.set_tooltip(Some(&tooltip_msg));

            let id_bytes = dev.unique_id.as_bytes();
            let len = id_bytes.len().min(64);
            LAST_DISPLAYED_ID[..len].copy_from_slice(&id_bytes[..len]);
            LAST_DISPLAYED_LEN = len;

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

    let _ = tray.set_icon(Some(icon));

    let new_menu = Menu::new();
    let mut has_devices = false;

    for id in &s.active_device_ids {
        if let Some(status) = s.device_statuses.get(id) {
            if let DeviceBatteryStatus::Online { channels } = status {
                let active_channels: Vec<&crate::plugins::BatteryChannel> = channels.iter().flatten().collect();
                if !active_channels.is_empty() {
                    let dev_name = s.config.devices.iter()
                        .find(|d| &d.unique_id == id)
                        .map(|d| d.name.as_str())
                        .unwrap_or(id.as_str());

                    let mut chan_parts = Vec::new();
                    for chan in active_channels {
                        let prefix = match chan.channel_type {
                            crate::plugins::ChannelType::Main => "",
                            crate::plugins::ChannelType::Left => "L: ",
                            crate::plugins::ChannelType::Right => "R: ",
                            crate::plugins::ChannelType::Case => "Case: ",
                        };
                        let charging_suffix = if chan.charging { "⚡" } else { "" };
                        chan_parts.push(format!("{}{}%{}", prefix, chan.percentage, charging_suffix));
                    }

                    let channels_str = chan_parts.join(" | ");
                    let item_text = format!("{}: {}", dev_name, channels_str);
                    let truncated_text = truncate_string(&item_text, 32);

                    let item = MenuItem::new(truncated_text, false, None);
                    let _ = new_menu.append(&item);
                    has_devices = true;
                }
            }
        }
    }

    if has_devices {
        let _ = new_menu.append(&PredefinedMenuItem::separator());
    }

    let _ = new_menu.append_items(&[settings_item, exit_item]);
    let _ = tray.set_menu(Some(Box::new(new_menu)));
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let mut truncated: String = s.chars().take(max_len - 3).collect();
        truncated.push_str("...");
        truncated
    } else {
        s.to_string()
    }
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

fn request_tray_update(state: &Arc<Mutex<SharedState>>) {
    let tid = state.lock().unwrap().tray_thread_id;
    if let Some(id) = tid {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                id,
                windows_sys::Win32::UI::WindowsAndMessaging::WM_USER + 100,
                0,
                0,
            );
        }
    }
}

struct BatStatApp {
    state: Arc<Mutex<SharedState>>,
    ui_state: Option<crate::ui::SettingsWindow>,
    visible: bool,
    first_frame: bool,
    pending_menu_events: Arc<Mutex<Vec<MenuEvent>>>,
    pending_tray_events: Arc<Mutex<Vec<TrayIconEvent>>>,
}

impl BatStatApp {
    fn new(
        state: Arc<Mutex<SharedState>>,
        pending_menu_events: Arc<Mutex<Vec<MenuEvent>>>,
        pending_tray_events: Arc<Mutex<Vec<TrayIconEvent>>>,
    ) -> Self {
        let visible = !state.lock().unwrap().has_tray;
        Self {
            state,
            ui_state: None,
            visible,
            first_frame: true,
            pending_menu_events,
            pending_tray_events,
        }
    }
}

impl eframe::App for BatStatApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let (settings_id, exit_id, has_tray) = {
            let s = self.state.lock().unwrap();
            (
                s.settings_item_id.clone(),
                s.exit_item_id.clone(),
                s.has_tray,
            )
        };

        if ctx.input(|i| i.viewport().close_requested()) {
            if has_tray {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.visible = false;
                self.ui_state = None;
                hide_settings_window();
            }
        }

        if self.first_frame {
            self.first_frame = false;
            // Hide the window immediately if we have a tray icon
            if has_tray {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                request_tray_update(&self.state);
            } else {
                // Initialize UI state immediately in fallback window mode
                let (config, active_ids, statuses) = {
                    let s = self.state.lock().unwrap();
                    (s.config.clone(), s.active_device_ids.clone(), s.device_statuses.clone())
                };
                self.ui_state = Some(crate::ui::SettingsWindow::new(config, active_ids, statuses));
            }
        }

        if !self.visible {
            if let Some(hwnd) = find_our_window() {
                let is_visible = unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0
                };
                if is_visible {
                    self.visible = true;
                    let (config, active_ids, statuses) = {
                        let s = self.state.lock().unwrap();
                        (s.config.clone(), s.active_device_ids.clone(), s.device_statuses.clone())
                    };
                    self.ui_state = Some(crate::ui::SettingsWindow::new(config, active_ids, statuses));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
            }
        }

        // Process queued menu events
        let menu_events: Vec<MenuEvent> = {
            let mut queue = self.pending_menu_events.lock().unwrap();
            queue.drain(..).collect()
        };
        for event in menu_events {
            if settings_id.as_ref() == Some(&event.id) {
                self.visible = true;
                let (config, active_ids, statuses) = {
                    let s = self.state.lock().unwrap();
                    (s.config.clone(), s.active_device_ids.clone(), s.device_statuses.clone())
                };
                self.ui_state = Some(crate::ui::SettingsWindow::new(config, active_ids, statuses));
                
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else if exit_id.as_ref() == Some(&event.id) {
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
                let mut needs_tray_update = false;
                {
                    let mut s = self.state.lock().unwrap();
                    
                    if ui_state.device_removed {
                        s.config = ui_state.config.clone();
                        let _ = crate::config::save_config(&s.config);
                        ui_state.device_removed = false;
                        needs_tray_update = true;
                    }
                    
                    ui_state.active_devices = s.active_device_ids.clone();
                    ui_state.device_statuses = s.device_statuses.clone();
                    ui_state.update_status = s.update_status.clone();
                    
                    if ui_state.request_update_check {
                        ui_state.request_update_check = false;
                        s.update_status = UpdateStatus::Checking;
                        
                        let state_for_check = Arc::clone(&self.state);
                        std::thread::spawn(move || {
                            match crate::updater::check_for_update() {
                                Ok(Some(info)) => {
                                    if let Ok(mut s) = state_for_check.lock() {
                                        s.update_status = UpdateStatus::Available(info);
                                    }
                                }
                                Ok(None) => {
                                    if let Ok(mut s) = state_for_check.lock() {
                                        s.update_status = UpdateStatus::NoUpdate;
                                    }
                                }
                                Err(e) => {
                                    if let Ok(mut s) = state_for_check.lock() {
                                        s.update_status = UpdateStatus::Error(e);
                                    }
                                }
                            }
                        });
                    }
                    
                    if let Some(ref download_url) = ui_state.request_download_install {
                        let download_url = download_url.clone();
                        ui_state.request_download_install = None;
                        s.update_status = UpdateStatus::Downloading(0.0);
                        
                        let state_for_download = Arc::clone(&self.state);
                        std::thread::spawn(move || {
                            let state_cb = Arc::clone(&state_for_download);
                            let progress_cb = move |progress| {
                                if let Ok(mut s) = state_cb.lock() {
                                    s.update_status = UpdateStatus::Downloading(progress);
                                }
                            };
                            if let Err(e) = crate::updater::download_and_install_update(&download_url, progress_cb) {
                                if let Ok(mut s) = state_for_download.lock() {
                                    s.update_status = UpdateStatus::Error(e);
                                }
                            }
                        });
                    }
                    
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
                if needs_tray_update {
                    request_tray_update(&self.state);
                }
                ui_state.update(ctx, frame);
                if ui_state.request_close {
                    // Sync main config from UI
                    {
                        let mut s = self.state.lock().unwrap();
                        s.config = ui_state.config.clone();
                    }
                    if !has_tray {
                        // In fallback mode, closing the window exits the app
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    } else {
                        self.visible = false;
                        self.ui_state = None;
                        hide_settings_window();
                        request_tray_update(&self.state);
                    }
                }
            }
        }

        // Poll events periodically when hidden
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<dyn Any>",
            },
        };
        let location = info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let log_content = format!("Panic: {}\nLocation: {}\nBacktrace:\n{:?}", msg, location, backtrace);
        let _ = std::fs::write("c:\\Users\\Gila\\dev\\BatStat\\panic.log", log_content);
    }));

    unsafe {
        let _ = windows_sys::Win32::System::Com::CoInitializeEx(
            std::ptr::null(),
            windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED as u32,
        );
    }

    // Keep mutex handle alive so it doesn't get dropped and released prematurely.
    let _instance_mutex = unsafe {
        use std::os::windows::ffi::OsStrExt;
        let mutex_name: Vec<u16> = std::ffi::OsStr::new("Local\\BatStatSingleInstanceMutex")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mutex = windows_sys::Win32::System::Threading::CreateMutexW(
            std::ptr::null(),
            1, // InitialOwner = TRUE
            mutex_name.as_ptr(),
        );

        if mutex.is_null() {
            eprintln!("Failed to create single instance mutex");
            None
        } else {
            let error = windows_sys::Win32::Foundation::GetLastError();
            if error == windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS {
                eprintln!("BatStat is already running. Showing existing instance.");
                // Try to find the existing window
                let title: Vec<u16> = std::ffi::OsStr::new("BatStat Settings")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW(
                    std::ptr::null(),
                    title.as_ptr(),
                );
                if !hwnd.is_null() {
                    windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                        hwnd,
                        windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW,
                    );
                    windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
                }
                windows_sys::Win32::Foundation::CloseHandle(mutex);
                return Ok(());
            }
            Some(mutex)
        }
    };

    let config = load_config();

    crate::config::setup_icons_folder();
    
    let shared_state = Arc::new(Mutex::new(SharedState {
        config: config.clone(),
        active_device_ids: Vec::new(),
        device_statuses: std::collections::HashMap::new(),
        last_notified: std::collections::HashMap::new(),
        request_poll: false,
        settings_item_id: None,
        exit_item_id: None,
        has_tray: false,
        tray_thread_id: None,
        initialized: false,
        update_status: UpdateStatus::Idle,
    }));

    // Spawn background thread to check for updates on startup
    {
        let state_for_check = Arc::clone(&shared_state);
        std::thread::spawn(move || {
            {
                if let Ok(mut s) = state_for_check.lock() {
                    s.update_status = UpdateStatus::Checking;
                }
            }
            match crate::updater::check_for_update() {
                Ok(Some(info)) => {
                    if let Ok(mut s) = state_for_check.lock() {
                        s.update_status = UpdateStatus::Available(info);
                    }
                }
                Ok(None) => {
                    if let Ok(mut s) = state_for_check.lock() {
                        s.update_status = UpdateStatus::NoUpdate;
                    }
                }
                Err(e) => {
                    if let Ok(mut s) = state_for_check.lock() {
                        s.update_status = UpdateStatus::Error(e);
                    }
                }
            }
        });
    }

    // Spawn background dedicated Tray + Polling thread
    let state_clone = Arc::clone(&shared_state);
    std::thread::spawn(move || {
        unsafe {
            let _ = windows_sys::Win32::System::Com::CoInitializeEx(
                std::ptr::null(),
                windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED as u32,
            );
        }

        let mut settings_item = tray_icon::menu::MenuItem::new("Settings", true, None);
        let mut exit_item = tray_icon::menu::MenuItem::new("Exit", true, None);
        
        let mut tray_icon = None;
        for attempt in 1..=30 {
            let tray_menu = tray_icon::menu::Menu::new();
            settings_item = tray_icon::menu::MenuItem::new("Settings", true, None);
            exit_item = tray_icon::menu::MenuItem::new("Exit", true, None);
            let _ = tray_menu.append_items(&[&settings_item, &exit_item]);

            let default_icon = load_icon_from_memory(include_bytes!("icons/ok.png"));
            match TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("BatStat Battery Monitor")
                .with_icon(default_icon)
                .build()
            {
                Ok(icon) => {
                    let mut s = state_clone.lock().unwrap();
                    s.settings_item_id = Some(settings_item.id().clone());
                    s.exit_item_id = Some(exit_item.id().clone());
                    s.has_tray = true;
                    tray_icon = Some(icon);
                    break;
                }
                Err(e) => {
                    eprintln!("WARNING: Attempt {} failed to build tray icon: {:?}.", attempt, e);
                    if attempt < 30 {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
            }
        }

        // Mark as initialized
        {
            let mut s = state_clone.lock().unwrap();
            s.initialized = true;
        }

        let plugins: Vec<Box<dyn DevicePlugin>> = vec![
            Box::new(plugins::pulsar::PulsarPlugin),
            Box::new(plugins::xbox::XboxPlugin),
            Box::new(plugins::steelseries::SteelSeriesPlugin),
        ];

        let mut last_poll = std::time::Instant::now();
        let mut last_cycle = std::time::Instant::now();
        let mut force_initial_poll = true;

        // Store the thread ID
        let tid = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
        {
            let mut s = state_clone.lock().unwrap();
            s.tray_thread_id = Some(tid);
        }

        // Set a timer to wake us up every 1000ms
        let _timer = unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::SetTimer(
                std::ptr::null_mut(),
                0,
                1000,
                None,
            )
        };

        unsafe {
            let mut msg = std::mem::zeroed();
            while windows_sys::Win32::UI::WindowsAndMessaging::GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                // If it's a timer message or a user message to wake up:
                if msg.message == windows_sys::Win32::UI::WindowsAndMessaging::WM_TIMER
                    || msg.message == windows_sys::Win32::UI::WindowsAndMessaging::WM_USER + 100
                {
                    let is_forced_update = msg.message == windows_sys::Win32::UI::WindowsAndMessaging::WM_USER + 100;

                    let (polling_interval, request_poll) = {
                        let mut state = state_clone.lock().unwrap();
                        let req = state.request_poll;
                        if req {
                            state.request_poll = false;
                        }
                        (state.config.polling_interval_secs, req)
                    };

                    let mut did_poll = false;
                    if force_initial_poll || request_poll || last_poll.elapsed() >= std::time::Duration::from_secs(polling_interval) {
                        force_initial_poll = false;
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
                                        if status.is_online() && status.effective_percentage() <= dev_cfg.threshold {
                                            let notified = state.last_notified.get(&dev_cfg.unique_id).cloned().unwrap_or(false);
                                            if !notified {
                                                state.last_notified.insert(dev_cfg.unique_id.clone(), true);
                                                trigger_notification(&dev_cfg.name, status.effective_percentage());
                                            }
                                        } else if !status.is_online() || status.effective_percentage() > dev_cfg.threshold {
                                            state.last_notified.insert(dev_cfg.unique_id.clone(), false);
                                        }
                                    }
                                }
                            }
                        }
                        did_poll = true;
                    }

                    if did_poll || is_forced_update || last_cycle.elapsed() >= std::time::Duration::from_secs(3) {
                        last_cycle = std::time::Instant::now();
                        if let Some(ref mut tray) = tray_icon {
                            update_tray_icon_and_menu_local(
                                &state_clone,
                                tray,
                                &settings_item,
                                &exit_item,
                            );
                        }
                    }
                }
                windows_sys::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                windows_sys::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
            }
        }
    });

    // Wait for background thread to initialize the tray icon
    loop {
        {
            let s = shared_state.lock().unwrap();
            if s.initialized {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // Run eframe Native UI Event Loop
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("BatStat Settings")
            .with_inner_size([480.0, 700.0])
            .with_resizable(false),
        ..Default::default()
    };

    let state_clone_ui = Arc::clone(&shared_state);
    
    eframe::run_native(
        "BatStat Settings",
        native_options,
        Box::new(move |cc| {
            // Set up event handlers that queue events and wake the UI
            let pending_menu = Arc::new(Mutex::new(Vec::<MenuEvent>::new()));
            let pending_tray = Arc::new(Mutex::new(Vec::<TrayIconEvent>::new()));

            let menu_queue = Arc::clone(&pending_menu);
            let menu_ctx = cc.egui_ctx.clone();
            let state_for_menu = Arc::clone(&state_clone_ui);
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                let (settings_id, exit_id) = {
                    let s = state_for_menu.lock().unwrap();
                    (s.settings_item_id.clone(), s.exit_item_id.clone())
                };
                if Some(event.id.clone()) == settings_id {
                    show_settings_window();
                } else if Some(event.id.clone()) == exit_id {
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
                pending_menu,
                pending_tray,
            )))
        }),
    )?;
    Ok(())
}


