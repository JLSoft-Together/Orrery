# apps/mobile index (Orrery Remote, Flutter, spike WP-1)

- lib/main.dart: RustLib.init, protoVersion()/echo() qua FRB, OrreryRemoteApp(rustStatus, terminal?) -> SpikePage
- lib/src/spike/spike_page.dart: TerminalView (xterm 4.0.0), nạp assets/fixture.ansi, đo cuộn, ô nhập UTF-8 + Gửi (local echo), ExtraKeysBar
- lib/src/spike/bench.dart: FrameStats/FrameReport (FrameTiming), rssMb()
- lib/src/spike/extra_keys.dart: thanh phím phụ, visibleEscape()
- lib/src/rust/*: sinh bởi flutter_rust_bridge_codegen generate (không sửa tay)
- rust/src/api/simple.rs: proto_version() -> u32, echo(Vec<u8>)
- rust_builder/: cargokit, build Rust trong Gradle (chạy cả 3 ABI, debug ~12 phút lần đầu)
- tool/gen_fixture.mjs: sinh assets/fixture.ansi (~256 KiB)
- android: network_security_config (cleartext), data_extraction_rules, allowBackup=false, minSdk 24, org.gradle.workers.max=4
