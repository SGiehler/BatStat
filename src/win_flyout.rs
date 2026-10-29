use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetCursorPos, GetWindowLongPtrW, GetWindowRect, SetForegroundWindow,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, SW_HIDE, SW_SHOW, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

pub const WINDOW_TITLE: &str = "BatStat";
pub const FLYOUT_WIDTH: f32 = 380.0;
pub const FLYOUT_HEIGHT: f32 = 490.0;
pub const TASKBAR_GAP_PX: i32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ScreenRect {
    pub fn is_valid(&self) -> bool {
        self.right > self.left && self.bottom > self.top
    }

    pub fn center(&self) -> (i32, i32) {
        (
            self.left + (self.right - self.left) / 2,
            self.top + (self.bottom - self.top) / 2,
        )
    }
}

pub fn compute_flyout_position(
    tray_rect: Option<ScreenRect>,
    cursor_pos: Option<(i32, i32)>,
    monitor_rect: ScreenRect,
    work_rect: ScreenRect,
    flyout_phys_size: (i32, i32),
) -> (i32, i32) {
    let (w, h) = (flyout_phys_size.0.max(1), flyout_phys_size.1.max(1));
    let gap = TASKBAR_GAP_PX;

    let anchor = tray_rect
        .filter(|r| r.is_valid())
        .map(|r| r.center())
        .or(cursor_pos)
        .unwrap_or((work_rect.right - w / 2 - gap, work_rect.bottom - gap));

    let top_inset = work_rect.top - monitor_rect.top;
    let bottom_inset = monitor_rect.bottom - work_rect.bottom;
    let left_inset = work_rect.left - monitor_rect.left;
    let right_inset = monitor_rect.right - work_rect.right;

    let (raw_x, raw_y) = if top_inset > bottom_inset && top_inset > left_inset && top_inset > right_inset {
        // Taskbar docked at top
        (anchor.0 - w / 2, work_rect.top + gap)
    } else if left_inset > bottom_inset && left_inset > right_inset {
        // Taskbar docked at left
        (work_rect.left + gap, anchor.1 - h / 2)
    } else if right_inset > bottom_inset {
        // Taskbar docked at right
        (work_rect.right - w - gap, anchor.1 - h / 2)
    } else {
        // Standard bottom taskbar (or auto-hidden taskbar)
        (anchor.0 - w / 2, work_rect.bottom - h - gap)
    };

    let min_x = work_rect.left + gap;
    let max_x = (work_rect.right - w - gap).max(min_x);
    let min_y = work_rect.top + gap;
    let max_y = (work_rect.bottom - h - gap).max(min_y);

    (raw_x.clamp(min_x, max_x), raw_y.clamp(min_y, max_y))
}

pub fn should_open_on_tray_click(
    currently_visible: bool,
    last_focus_lost: Option<Instant>,
    now: Instant,
) -> bool {
    if currently_visible {
        return false;
    }
    if let Some(lost_at) = last_focus_lost {
        if now.saturating_duration_since(lost_at) < Duration::from_millis(250) {
            return false;
        }
    }
    true
}

pub fn find_flyout_window() -> Option<HWND> {
    use std::os::windows::ffi::OsStrExt;
    let title: Vec<u16> = std::ffi::OsStr::new(WINDOW_TITLE)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if hwnd.is_null() {
        None
    } else {
        Some(hwnd)
    }
}

pub fn apply_flyout_window_style(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        let corner_pref: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &corner_pref as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new_style = (ex_style | (WS_EX_TOOLWINDOW as isize)) & !(WS_EX_APPWINDOW as isize);
        if new_style != ex_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
        }
    }
}

pub fn position_and_show_flyout(
    hwnd: HWND,
    tray_rect: Option<ScreenRect>,
    cursor_pos: Option<(i32, i32)>,
    scale_factor: f32,
    preserve_position: bool,
) {
    if hwnd.is_null() {
        return;
    }
    apply_flyout_window_style(hwnd);

    unsafe {
        if preserve_position {
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            return;
        }

        let fallback_cursor = cursor_pos.or_else(|| {
            let mut pt: POINT = std::mem::zeroed();
            if GetCursorPos(&mut pt) != 0 {
                Some((pt.x, pt.y))
            } else {
                None
            }
        });

        let anchor_pt = tray_rect
            .filter(|r| r.is_valid())
            .map(|r| r.center())
            .or(fallback_cursor)
            .unwrap_or((0, 0));

        let hmon = MonitorFromPoint(
            POINT {
                x: anchor_pt.0,
                y: anchor_pt.1,
            },
            MONITOR_DEFAULTTONEAREST,
        );

        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

        let (mon_rect, work_rect) = if GetMonitorInfoW(hmon, &mut mi) != 0 {
            (
                ScreenRect {
                    left: mi.rcMonitor.left,
                    top: mi.rcMonitor.top,
                    right: mi.rcMonitor.right,
                    bottom: mi.rcMonitor.bottom,
                },
                ScreenRect {
                    left: mi.rcWork.left,
                    top: mi.rcWork.top,
                    right: mi.rcWork.right,
                    bottom: mi.rcWork.bottom,
                },
            )
        } else {
            (
                ScreenRect {
                    left: 0,
                    top: 0,
                    right: 1920,
                    bottom: 1080,
                },
                ScreenRect {
                    left: 0,
                    top: 0,
                    right: 1920,
                    bottom: 1032,
                },
            )
        };

        let mut win_rect: RECT = std::mem::zeroed();
        let scale = if scale_factor > 0.1 { scale_factor } else { 1.0 };
        let phys_size = if GetWindowRect(hwnd, &mut win_rect) != 0
            && (win_rect.right - win_rect.left) > 100
            && (win_rect.bottom - win_rect.top) > 100
        {
            (
                win_rect.right - win_rect.left,
                win_rect.bottom - win_rect.top,
            )
        } else {
            (
                (FLYOUT_WIDTH * scale).round() as i32,
                (FLYOUT_HEIGHT * scale).round() as i32,
            )
        };

        let (x, y) =
            compute_flyout_position(tray_rect, fallback_cursor, mon_rect, work_rect, phys_size);

        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_SHOWWINDOW,
        );
        SetForegroundWindow(hwnd);
    }
}

pub fn hide_flyout(hwnd: HWND) {
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_bottom_taskbar_centers_over_tray_and_clamps_right_edge() {
        let mon = ScreenRect { left: 0, top: 0, right: 1920, bottom: 1080 };
        let work = ScreenRect { left: 0, top: 0, right: 1920, bottom: 1032 };
        let tray = ScreenRect { left: 1840, top: 1032, right: 1872, bottom: 1080 };
        let (x, y) = compute_flyout_position(Some(tray), None, mon, work, (380, 490));
        // Centered at 1856 -> 1856 - 190 = 1666, which overflows 1920 - 380 - 12 = 1528 -> clamped to 1528
        assert_eq!(x, 1528);
        assert_eq!(y, 1032 - 490 - 12);
    }

    #[test]
    fn test_top_and_side_taskbars_anchor_to_correct_work_edge() {
        let mon = ScreenRect { left: 0, top: 0, right: 1920, bottom: 1080 };
        // Top taskbar (48px at top)
        let work_top = ScreenRect { left: 0, top: 48, right: 1920, bottom: 1080 };
        let tray_top = ScreenRect { left: 1700, top: 0, right: 1732, bottom: 48 };
        let (_, y_top) = compute_flyout_position(Some(tray_top), None, mon, work_top, (380, 490));
        assert_eq!(y_top, 48 + 12);

        // Left taskbar (72px on left)
        let work_left = ScreenRect { left: 72, top: 0, right: 1920, bottom: 1080 };
        let tray_left = ScreenRect { left: 0, top: 900, right: 72, bottom: 932 };
        let (x_left, _) = compute_flyout_position(Some(tray_left), None, mon, work_left, (380, 490));
        assert_eq!(x_left, 72 + 12);
    }

    #[test]
    fn test_flyout_position_negative_secondary_monitor_coords() {
        let mon = ScreenRect { left: -1920, top: 0, right: 0, bottom: 1080 };
        let work = ScreenRect { left: -1920, top: 0, right: 0, bottom: 1032 };
        let tray = ScreenRect { left: -220, top: 1032, right: -188, bottom: 1080 };
        let (x, y) = compute_flyout_position(Some(tray), None, mon, work, (380, 490));
        assert!(x >= -1920 + 12 && x + 380 <= -12);
        assert_eq!(y, 1032 - 490 - 12);
    }

    #[test]
    fn test_tray_toggle_suppresses_reopen_after_recent_focus_loss() {
        let t0 = Instant::now();
        assert!(!should_open_on_tray_click(true, None, t0));
        assert!(!should_open_on_tray_click(false, Some(t0), t0 + Duration::from_millis(80)));
        assert!(should_open_on_tray_click(false, Some(t0), t0 + Duration::from_millis(300)));
        assert!(should_open_on_tray_click(false, None, t0));
    }
}
