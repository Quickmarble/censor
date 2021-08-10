#[cfg(target_arch = "wasm32")]
use stdweb;

#[cfg(target_arch = "wasm32")]
pub fn read_storage(key: &str) -> Option<String> {
    let storage = stdweb::web::window().session_storage();
    return storage.get(key);
}

#[cfg(target_arch = "wasm32")]
pub fn write_storage(key: &str, val: String) {
    let storage = stdweb::web::window().session_storage();
    storage.insert(key, &val).unwrap();
}
