use core_monitor::serve::{run, ServeConfig};

fn main() {
    let dev = std::env::args().any(|a| a == "--dev");
    std::process::exit(run(ServeConfig::from_env(dev)));
}
