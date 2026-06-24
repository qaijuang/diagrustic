#![cfg(feature = "std")]

use std::env;
use std::io::{self, IsTerminal};

pub(crate) fn stderr_supports_color() -> bool {
    if env::var_os("FORCE_COLOR") == Some("1".into()) {
        return true;
    }

    let stderr = io::stderr();
    stderr.is_terminal() && enable_ansi_stderr()
}

#[cfg(not(windows))]
fn enable_ansi_stderr() -> bool {
    true
}

#[cfg(windows)]
fn enable_ansi_stderr() -> bool {
    windows_terminal::enable_ansi_stderr()
}

#[cfg(windows)]
mod windows_terminal {
    use core::ffi::c_void;

    type Handle = *mut c_void;

    const STD_ERROR_HANDLE: u32 = (-12i32).cast_unsigned();
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        #[link_name = "GetStdHandle"]
        fn get_std_handle(n_std_handle: u32) -> Handle;

        #[link_name = "GetConsoleMode"]
        fn get_console_mode(console_handle: Handle, mode: *mut u32) -> i32;

        #[link_name = "SetConsoleMode"]
        fn set_console_mode(console_handle: Handle, mode: u32) -> i32;
    }

    pub(super) fn enable_ansi_stderr() -> bool {
        unsafe {
            let handle = get_std_handle(STD_ERROR_HANDLE);
            let mut mode = 0;
            if handle.is_null()
                || handle as isize == -1
                || get_console_mode(handle, &raw mut mode) == 0
            {
                return false;
            }

            let ansi_mode = mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            ansi_mode == mode || set_console_mode(handle, ansi_mode) != 0
        }
    }
}
