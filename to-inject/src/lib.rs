use std::{
    ffi::CString,
    sync::{Mutex, OnceLock},
};

use once_cell::sync::Lazy;
use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::*,
        System::{SystemServices::*, Threading},
        UI::WindowsAndMessaging::{self, MessageBoxA},
    },
};

static STARTUP_HWND: OnceLock<HWND> = OnceLock::new();

#[no_mangle]
extern "system" fn DllMain(_dll_module: HINSTANCE, call_reason: u32, _: *mut ()) -> bool {
    match call_reason {
        DLL_PROCESS_ATTACH => attach(),
        DLL_PROCESS_DETACH => detach(),
        _ => (),
    }

    true
}

fn attach() {}

fn detach() {}

#[no_mangle]
extern "system" fn do_setup(hwnd: isize) -> bool {
    unsafe {
        WindowsAndMessaging::SetWindowDisplayAffinity(
            HWND(hwnd),
            WindowsAndMessaging::WDA_EXCLUDEFROMCAPTURE,
        )
        .map_err(|error| message_box("error setting the thing :3", format!("error: {error:?}")))
        .ok()
    };

    true
}

fn message_box(top_text: impl Into<String>, bottom_text: impl Into<String>) {
    let top_text = CString::new(top_text.into().as_bytes().to_vec()).unwrap();
    let bottom_text = CString::new(bottom_text.into().as_bytes().to_vec()).unwrap();

    unsafe {
        MessageBoxA(
            HWND(0),
            PCSTR(bottom_text.as_ptr() as *const u8),
            PCSTR(top_text.as_ptr() as *const u8),
            Default::default(),
        );
    }
}
