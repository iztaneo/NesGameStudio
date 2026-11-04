use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

#[repr(C)]
struct StartupInfoW {
    cb: u32,
    lp_reserved: *mut u16,
    lp_desktop: *mut u16,
    lp_title: *mut u16,
    dw_x: u32,
    dw_y: u32,
    dw_x_size: u32,
    dw_y_size: u32,
    dw_x_count_chars: u32,
    dw_y_count_chars: u32,
    dw_fill_attribute: u32,
    dw_flags: u32,
    w_show_window: u16,
    cb_reserved2: u16,
    lp_reserved2: *mut u8,
    h_std_input: isize,
    h_std_output: isize,
    h_std_error: isize,
}

#[repr(C)]
struct ProcessInformation {
    h_process: isize,
    h_thread: isize,
    dw_process_id: u32,
    dw_thread_id: u32,
}

#[link(name = "user32")]
extern "system" {
    fn GetThreadDesktop(dw_thread_id: u32) -> isize;
    fn GetUserObjectInformationW(
        h_obj: isize,
        n_index: i32,
        pv_info: *mut u16,
        n_length: u32,
        lpn_length_needed: *mut u32,
    ) -> i32;
    fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> isize;
    fn ShowWindow(hwnd: isize, n_cmd_show: i32) -> i32;
    fn SetForegroundWindow(hwnd: isize) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentThreadId() -> u32;
    fn GetCommandLineW() -> *mut u16;
    fn CreateProcessW(
        lp_application_name: *const u16,
        lp_command_line: *mut u16,
        lp_process_attributes: *const u8,
        lp_thread_attributes: *const u8,
        b_inherit_handles: i32,
        dw_creation_flags: u32,
        lp_environment: *const u8,
        lp_current_directory: *const u16,
        lp_startup_info: *const StartupInfoW,
        lp_process_information: *mut ProcessInformation,
    ) -> i32;
    fn WaitForSingleObject(h_handle: isize, dw_milliseconds: u32) -> u32;
    fn GetConsoleWindow() -> isize;
}

fn wide_null(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn ensure_on_default_desktop() {
    unsafe {
        let tid = GetCurrentThreadId();
        let cur_desk = GetThreadDesktop(tid);
        let mut buf = [0u16; 128];
        let mut needed = 0u32;
        let mut desk_name = String::new();
        if GetUserObjectInformationW(cur_desk, 2, buf.as_mut_ptr(), 256, &mut needed) != 0 {
            let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
            desk_name = String::from_utf16_lossy(&buf[..len]);
        }
        println!("[NesGameStudio] Desktop del proceso: '{}'", desk_name);

        if !desk_name.is_empty() && !desk_name.eq_ignore_ascii_case("Default") {
            println!(
                "[NesGameStudio] Relanzando en el escritorio interactivo 'WinSta0\\Default'..."
            );
            let mut desktop = wide_null("WinSta0\\Default");
            let si = StartupInfoW {
                cb: std::mem::size_of::<StartupInfoW>() as u32,
                lp_reserved: ptr::null_mut(),
                lp_desktop: desktop.as_mut_ptr(),
                lp_title: ptr::null_mut(),
                dw_x: 0,
                dw_y: 0,
                dw_x_size: 0,
                dw_y_size: 0,
                dw_x_count_chars: 0,
                dw_y_count_chars: 0,
                dw_fill_attribute: 0,
                dw_flags: 1,      // STARTF_USESHOWWINDOW
                w_show_window: 5, // SW_SHOW
                cb_reserved2: 0,
                lp_reserved2: ptr::null_mut(),
                h_std_input: 0,
                h_std_output: 0,
                h_std_error: 0,
            };
            let mut pi = ProcessInformation {
                h_process: 0,
                h_thread: 0,
                dw_process_id: 0,
                dw_thread_id: u32::default(),
            };

            const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
            let ok = CreateProcessW(
                ptr::null(),
                GetCommandLineW(),
                ptr::null(),
                ptr::null(),
                0,
                CREATE_NEW_CONSOLE,
                ptr::null(),
                ptr::null(),
                &si,
                &mut pi,
            );
            if ok != 0 {
                println!(
                    "[NesGameStudio] Proceso GUI iniciado en WinSta0\\Default (PID = {}).",
                    pi.dw_process_id
                );
                WaitForSingleObject(pi.h_process, 0xFFFF_FFFF);
                std::process::exit(0);
            }
        }
    }
}

pub fn force_show_window_after_creation(title: &str) {
    let wide_title = wide_null(title);

    std::thread::spawn(move || {
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(80));
            unsafe {
                let hwnd = FindWindowW(ptr::null(), wide_title.as_ptr());
                if hwnd != 0 {
                    let console_hwnd = GetConsoleWindow();
                    if console_hwnd != 0 && console_hwnd != hwnd {
                        ShowWindow(console_hwnd, 0); // SW_HIDE
                    }
                    ShowWindow(hwnd, 9); // SW_RESTORE
                    ShowWindow(hwnd, 5); // SW_SHOW
                    SetForegroundWindow(hwnd);
                    break;
                }
            }
        }
    });
}
