#[flutter_rust_bridge::frb(sync)]
pub fn proto_version() -> u32 {
    1
}

#[flutter_rust_bridge::frb(sync)]
pub fn echo(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
