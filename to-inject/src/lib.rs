use std::{ffi::CString, sync::Mutex};

use once_cell::sync::Lazy;
use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::*,
        System::{SystemServices::*, Threading},
        UI::WindowsAndMessaging::{self, MessageBoxA},
    },
};

#[no_mangle]
extern "system" fn DllMain(_dll_module: HINSTANCE, call_reason: u32, _: *mut ()) -> bool {
    match call_reason {
        DLL_PROCESS_ATTACH => attach(),
        DLL_PROCESS_DETACH => detach(),
        _ => (),
    }

    true
}

fn attach() {
    unsafe {
        let our_hwnds = get_whwnds();
        // let text = CString::new(format!("{our_hwnds:?}").as_bytes().to_vec()).unwrap();
        // MessageBoxA(
        //     HWND(0),
        //     PCSTR(text.as_ptr() as *const u8),
        //     s!("inject 1"),
        //     Default::default(),
        // );
        for hwnd in our_hwnds {
            WindowsAndMessaging::SetWindowDisplayAffinity(
                hwnd,
                WindowsAndMessaging::WDA_EXCLUDEFROMCAPTURE,
            )
            .map_err(|error| {
                let text = CString::new(format!("{error:?}").as_bytes().to_vec()).unwrap();

                MessageBoxA(
                    HWND(0),
                    PCSTR(text.as_ptr() as *const u8),
                    s!("error excluding"),
                    Default::default(),
                )
            })
            .ok();
        }
    }
}

fn detach() {
    // unsafe {
    // MessageBoxA(
    //     HWND(0),
    //     s!("uninject 1"),
    //     s!("uninject 2"),
    //     Default::default(),
    // );
    // }
}

static OUTPUT_VEC: Lazy<Mutex<Vec<(u32, HWND)>>> = Lazy::new(|| Mutex::new(vec![]));

unsafe fn get_whwnds() -> Vec<HWND> {
    // #[cfg(target_pointer_width = "32")]
    // {
    //     compile_error!("this is 100% gonna be broken on 32 bit");
    // }

    let our_pid = Threading::GetCurrentProcessId();
    WindowsAndMessaging::EnumWindows(Some(window_iter), LPARAM(our_pid as isize))
        .expect("failed to iter windows");
    let output = std::mem::take(&mut (*OUTPUT_VEC.lock().unwrap()));
    output.into_iter().map(|(_, hwnd)| hwnd).collect()
}

unsafe extern "system" fn window_iter(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut pid: u32 = 0;
    WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32));
    if pid == lparam.0 as u32 {
        let mut lock = OUTPUT_VEC.lock().expect("failed to unwrap vec");
        lock.push((pid, hwnd));
    }

    true.into()
}
