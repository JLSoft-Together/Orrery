use std::process::ExitCode;

fn run(input: &str, output: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(input).map_err(|e| format!("{input}: {e}"))?;
    let doc: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{input}: {e}"))?;
    let (trace, stats) = core_store::replay::replay(&doc)?;
    let out = serde_json::to_string(&trace).map_err(|e| e.to_string())?;
    std::fs::write(output, out).map_err(|e| format!("{output}: {e}"))?;
    Ok(format!(
        "[store-replay] {} ops, {} calls, {} frames, {} dropped, refs reused {} mutated {}, clock skews {} -> {output}",
        stats.ops, stats.calls, stats.frames, stats.dropped, stats.reused_refs, stats.mutated_refs, stats.clock_skews
    ))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output] = args.as_slice() else {
        eprintln!("usage: store-replay <ops.json> <out.trace.json>");
        return ExitCode::from(2);
    };
    match run(input, output) {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[store-replay] {e}");
            ExitCode::FAILURE
        }
    }
}
