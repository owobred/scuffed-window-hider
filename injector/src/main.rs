use std::sync::Mutex;

use dll_syringe::{error::InjectError, process::OwnedProcess, Syringe};
use once_cell::sync::Lazy;
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM},
    System::Threading,
    UI::WindowsAndMessaging,
};

static OUTPUT_VEC: Lazy<Mutex<Vec<(u32, u32, HWND, String, Option<bool>)>>> = Lazy::new(|| Mutex::new(vec![]));

fn main() {
    unsafe {
        WindowsAndMessaging::EnumWindows(Some(window_iter), LPARAM::default())
            .expect("failed to iterate over windows");
    }

    let output = std::mem::take(&mut (*OUTPUT_VEC.lock().unwrap()));
    let output = output
        .into_iter()
        .filter(|info| info.4.is_some())
        .map(|info| (info.0, info.1, info.2, info.3, info.4.unwrap()))
        .filter(|info| info.3.contains("Task Switching"))
        .collect::<Vec<_>>();

    println!("{:?}", output.iter().map(|info| info.3.to_string()).collect::<Vec<String>>());

    let processes = output
        .iter()
        .map(|(pid, _window_thread_pid, _hwnd, _name, _is_32_bit)| OwnedProcess::from_pid(*pid).unwrap());

    let mut syringes = vec![];
    for process in processes {
        let syringe = Syringe::for_process(process);
        syringes.push(syringe);
    }

    let mut modules = vec![];
    for (syringe, info) in syringes.iter().zip(output) {
        println!("attempting to inject into {}", info.3);
        let module = match info.4 {
            true => syringe.inject("target\\i686-pc-windows-msvc\\debug\\to_inject.dll"),
            false => syringe.inject("target\\debug\\to_inject.dll"),
        };
        modules.push((module, info));
    }

    let to_eject = syringes
        .iter()
        .zip(modules)
        .filter_map(
            |(syringe, (result, (pid, thread_pid, hwnd, name, is_32_bit)))| match result {
                Ok(module) => Some((syringe, (module, (pid, thread_pid, hwnd, name, is_32_bit)))),
                Err(error) => {
                    println!("failed to inject into {name} (pid {pid}): {error:?}");
                    None
                }
            },
        )
        .collect::<Vec<_>>();

    for (syringe, (module, info)) in to_eject {
        let result = match info.4 {
            true => {
                // 32 bit process
                let setup_fn = unsafe {
                    syringe
                        .get_raw_procedure::<extern "system" fn(i32) -> bool>(module, "do_setup")
                        .expect("error loading")
                        .expect("missing procedure")
                };
                setup_fn.call(info.2.0 as i32).expect("failed to rpc call")
            },
            false => {
                // probably 64 bit
                let setup_fn = unsafe {
                    syringe
                        .get_raw_procedure::<extern "system" fn(i64) -> bool>(module, "do_setup")
                        .expect("error loading")
                        .expect("missing procedure")
                };
                setup_fn.call(info.2.0 as i64).expect("failed to rpc call")
            }
        };
        println!("called setup and got {result:?}");
        syringe.eject(module).expect("failed to eject");
        println!("ejected {info:?}")
    }
}

unsafe extern "system" fn window_iter(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    let mut pid: u32 = 0;
    let window_thread_pid =
        WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32));

    let mut window_name = [0u8; 1024];
    let window_name_len = WindowsAndMessaging::GetWindowTextA(hwnd, &mut window_name);
    let window_name = &window_name[..window_name_len as usize];
    let name = String::from_utf8_lossy(window_name);

    let is_32_bit = check_process(pid);

    let mut lock = OUTPUT_VEC.lock().unwrap();
    lock.push((pid, window_thread_pid, hwnd, name.to_string(), is_32_bit));

    true.into()
}

unsafe fn check_process(pid: u32) -> Option<bool> {
    let handle = Threading::OpenProcess(Threading::PROCESS_ALL_ACCESS, true, pid).ok()?;
    let mut inner: BOOL = false.into();
    Threading::IsWow64Process(handle, &mut inner as *mut BOOL)
        .expect("failed to tell if process is 64 bit");
    Some(inner.as_bool())
}