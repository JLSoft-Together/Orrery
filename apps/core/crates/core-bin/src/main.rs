fn main() {
    let code = match std::env::args_os().nth(1) {
        Some(mode) if mode == "procs" => core_procs::serve::run(),
        Some(mode) if mode == "transcript" => core_transcript::serve::run(),
        _ => core_pty::server::run(),
    };
    std::process::exit(code);
}
