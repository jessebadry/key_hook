mod inputs;

use futures::executor::ThreadPool;
use lazy_static::lazy_static;
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use std::sync::Mutex;
use user32::*;
use winapi::winuser::KBDLLHOOKSTRUCT;
use winapi::winuser::MSG;
use winapi::HWND;

const WH_KEYBOARD_LL: i32 = 13;
const KEY_DOWN: u64 = 256;
const KEY_UP: u64 = 257;
//const SYSTEM_KEY: u64 = 260;
lazy_static! {
    static ref CALLBACK: Mutex<Option<Box<dyn FnMut(Key) + Send>>> = Mutex::new(Default::default());
    static ref CAPITALIZED: Mutex<bool> = Mutex::new(false);
    static ref SHIFT_TOGGLED: Mutex<bool> = Mutex::new(false);
}
pub enum Key {
    KeyUp(String),
    KeyDown(String),
    SystemKey(String),
}

async fn start_hook<F: FnMut(Key) + Send + 'static>(call: F) {
    {
        *CALLBACK.lock().unwrap() = Some(Box::new(call));
    }

    unsafe {
        {
            let capped = user32::GetKeyState(20);
            let shift_key_toggled = user32::GetKeyState(160);
            SHIFT_TOGGLED
                .lock()
                .and_then(|mut shift| {
                    *shift = shift_key_toggled < 0;
                    Ok(())
                })
                .unwrap_or_else(|e| panic!("Could not convert to shifted letter."));
            *CAPITALIZED.lock().unwrap() = capped == 1;

            // println!("i = {}", capped);
            // println!("shift = {}", *SHIFT_TOGGLED.lock().unwrap());
        }
        let hook_id =
            SetWindowsHookExA(WH_KEYBOARD_LL, Some(hook_callback), std::ptr::null_mut(), 0);
        let mut msg: MSG = MaybeUninit::uninit().assume_init();
        while GetMessageW(&mut msg, 0 as HWND, 0, 0) != 0 {
            TranslateMessage(&mut msg);
            DispatchMessageA(&mut msg);
        }
        UnhookWindowsHookEx(hook_id);
    }
}
pub fn hook_keyboard<F: FnMut(Key) + Send + 'static>(output: F) {
    let pool = ThreadPool::new().unwrap();
    pool.spawn_ok(async move { start_hook(output).await });
}
fn toggle_shift(val: bool) {
    let toggled = &mut *SHIFT_TOGGLED.lock().unwrap();
    *toggled = val;
}
unsafe extern "system" fn hook_callback(code: i32, w_param: u64, l_param: i64) -> i64 {
    let key = (*(l_param as *const KBDLLHOOKSTRUCT)).vkCode;
    //process ect..
    let is_shift_key = key == 0xA0;
    let key_down = w_param == KEY_DOWN;

    if is_shift_key {
        toggle_shift(key_down);
    }
    let shifted = (*SHIFT_TOGGLED.lock().unwrap()).clone();
    let mut key_string = inputs::vk_to_string(key, shifted);

    if key_string == inputs::CAPS_LOCK && key_down {
        let cap = &mut *CAPITALIZED.lock().unwrap();
        *cap = !*cap;
    }
    let capped = (*CAPITALIZED.lock().unwrap()).clone();

    //determie if the user
    if (capped && shifted) || (!capped && !shifted) {
        key_string = key_string.to_lowercase();
    }

    let key = match w_param {
        KEY_UP => Key::KeyUp(key_string),
        KEY_DOWN => Key::KeyDown(key_string),
        _ => Key::SystemKey(key_string),
    };
    std::thread::spawn(move || {
        if let Some(call) = &mut *CALLBACK.lock().unwrap() {
            call(key);
        }
    });

    user32::CallNextHookEx(null_mut(), code, w_param, l_param);
    0
}

#[test]
fn hook_keyboard_t() {
    hook_keyboard(|key| match key {
        Key::KeyDown(key) => {
            println!("Key = {}", key);
        }
        Key::KeyUp(key) => {
            println!("Key = {}", key);
        }
        _ => {}
    });
}
