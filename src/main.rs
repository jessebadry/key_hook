mod key_hook;
use crate::key_hook::Key;
fn main() {
    key_hook::hook_keyboard(|the_key| match the_key {
        Key::KeyDown(key) => println!("{}", key),
        _ => {}
    });
    std::thread::park();
}
