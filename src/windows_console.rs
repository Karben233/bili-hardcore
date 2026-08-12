#[cfg(feature = "gui")]
pub fn detach_for_gui() {
    unsafe {
        use windows_sys::Win32::System::Console::{
            FreeConsole, GetConsoleProcessList, GetConsoleWindow,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};

        let mut processes = [0u32; 2];
        if GetConsoleProcessList(processes.as_mut_ptr(), processes.len() as u32) == 1 {
            let window = GetConsoleWindow();
            if !window.is_null() {
                ShowWindow(window, SW_HIDE);
            }
        }
        FreeConsole();
    }
}
