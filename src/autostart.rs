use std::env;
use std::os::windows::ffi::OsStrExt;
use std::ffi::OsStr;
use windows_sys::Win32::System::Registry::{
    RegOpenKeyExW, RegSetValueExW, RegDeleteValueW, RegCloseKey,
    HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ
};

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let exe_path = env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;
    
    // Add quotes around the path to handle spaces
    let exe_path_str = format!("\"{}\"", exe_path.to_string_lossy());
    
    let subkey: Vec<u16> = OsStr::new("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
        
    let value_name: Vec<u16> = OsStr::new("BatStat")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey = std::ptr::null_mut();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );

        if status != 0 {
            return Err(format!("Failed to open registry key: status {}", status));
        }

        if enabled {
            let value_data: Vec<u16> = OsStr::new(&exe_path_str)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            
            let status = RegSetValueExW(
                hkey,
                value_name.as_ptr(),
                0,
                REG_SZ,
                value_data.as_ptr() as *const u8,
                (value_data.len() * 2) as u32,
            );

            RegCloseKey(hkey);

            if status != 0 {
                return Err(format!("Failed to set registry value: status {}", status));
            }
        } else {
            let status = RegDeleteValueW(hkey, value_name.as_ptr());
            RegCloseKey(hkey);

            // Ignore "file not found" (status 2) errors when disabling
            if status != 0 && status != 2 {
                return Err(format!("Failed to delete registry value: status {}", status));
            }
        }
    }

    Ok(())
}
