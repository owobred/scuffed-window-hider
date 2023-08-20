use std::sync::{Mutex};

use dll_syringe::{process::OwnedProcess, Syringe};
use once_cell::sync::Lazy;
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM},
    UI::WindowsAndMessaging,
};

static OUTPUT_VEC: Lazy<Mutex<Vec<(u32, u32, HWND, String)>>> = Lazy::new(|| Mutex::new(vec![]));

fn main() {
    unsafe {
        WindowsAndMessaging::EnumWindows(Some(window_iter), LPARAM::default())
            .expect("failed to iterate over windows");
    }

    let output = std::mem::take(&mut (*OUTPUT_VEC.lock().unwrap()));
    let output = output.into_iter().filter(|info| info.3 == "Spotify Free").collect::<Vec<_>>();

    println!("{output:?}");

    let processes = output
        .iter()
        .map(|(pid, _window_thread_pid, _hwnd, _name)| OwnedProcess::from_pid(*pid).unwrap());

    let mut syringes = vec![];
    for process in processes {
        let syringe = Syringe::for_process(process);
        syringes.push(syringe);
    }

    let mut modules = vec![];
    for (syringe, info) in syringes.iter().zip(output) {
        let module = syringe.inject("target\\debug\\to_inject.dll");
        modules.push((module, info));
    }

    let to_eject =  syringes.iter().zip(modules).filter_map(|(syringe, (result, (pid, thread_pid, hwnd, name)))| {
        match result {
            Ok(module) => Some((syringe, (module, (pid, thread_pid, hwnd, name)))),
            Err(error) => {
                println!("failed to inject into {name} (pid {pid}): {error:?}");
                None
            },
        }
    }).collect::<Vec<_>>();

    for (syringe, (module, info)) in to_eject {
        syringe.eject(module).expect("failed to eject");
        println!("ejected {info:?}")
    }

    // let processes = OwnedProcess::find_all_by_name("Spotify");

    // let mut syringes = vec![];
    // for process in processes {
    //     let syringe = Syringe::for_process(process);
    //     syringes.push(syringe);
    // }

    // let mut modules = vec![];
    // for syringe in &syringes {
    //     let module = syringe.inject("target\\debug\\to_inject.dll").map_err(|_| eprintln!("failed to inject into {:?}", syringe.process()));
    //     if let Ok(module) = module {
    //         println!("injected into {:?}", syringe.process());
    //         modules.push((syringe, module));
    //     }
    // }

    // std::thread::sleep(std::time::Duration::from_secs(4));

    // for (syringe, module) in modules {
    //     syringe.eject(module).expect("failed to eject");
    // }
}

unsafe extern "system" fn window_iter(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    let mut pid: u32 = 0;
    let window_thread_pid =
        WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32));

    let mut window_name = [0u8; 1024];
    let window_name_len = WindowsAndMessaging::GetWindowTextA(hwnd, &mut window_name);
    let window_name = &window_name[..window_name_len as usize];
    let name = String::from_utf8_lossy(window_name);

    let mut lock = OUTPUT_VEC.lock().unwrap();
    lock.push((pid, window_thread_pid, hwnd, name.to_string()));

    return true.into();
}
