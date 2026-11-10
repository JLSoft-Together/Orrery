fn main() {
    let code = match std::env::args_os().nth(1) {
        Some(mode) if mode == "procs" => core_procs::serve::run(),
        Some(mode) if mode == "transcript" => core_transcript::serve::run(),
        Some(mode) if mode == "desk" => core_desk::stdio::run(),
        Some(mode) if mode == "serve" => core_serve::main(&std::env::args().skip(2).collect::<Vec<_>>()),
        _ => core_pty::server::run(),
    };
    std::process::exit(code);
}
