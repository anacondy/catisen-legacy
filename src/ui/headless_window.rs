// Headless window management using Windows API
// Ensures windows never appear even when created by eframe/egui

#[cfg(target_os = "windows")]
use winapi::um::winuser::{ShowWindow, SetWindowPos, SW_HIDE, HWND_MESSAGE, SWP_HIDEWINDOW};
#[cfg(target_os = "windows")]
use winapi::um::winuser::EnumWindows;
#[cfg(target_os = "windows")]
use winapi::um::winuser::GetWindowThreadProcessId;
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::GetCurrentProcessId;

/// Hide all Catisen windows on Windows using native APIs
#[cfg(target_os = "windows")]
pub fn hide_catisen_windows() {
    unsafe {
        // Enumerate all windows and hide them
        let _ = EnumWindows(Some(enum_window_callback), 0 as isize);
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_window_callback(hwnd: winapi::shared::windef::HWND, _lparam: isize) -> winapi::shared::minwindef::BOOL {
    // Check if window belongs to our process
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    
    if pid == GetCurrentProcessId() {
        // Hide this window immediately
        let _ = ShowWindow(hwnd, SW_HIDE);
        
        // Also try to move it completely off-screen
        let _ = SetWindowPos(hwnd, HWND_MESSAGE, -99999, -99999, 1, 1, SWP_HIDEWINDOW);
    }
    
    1 // Continue enumeration
}

#[cfg(not(target_os = "windows"))]
pub fn hide_catisen_windows() {
    // No-op on non-Windows platforms
}

/// Force window to stay hidden during test mode
pub fn ensure_headless_mode() {
    #[cfg(target_os = "windows")]
    {
        hide_catisen_windows();
        std::thread::sleep(std::time::Duration::from_millis(100));
        hide_catisen_windows(); // Double-call to be extra sure
    }
}

