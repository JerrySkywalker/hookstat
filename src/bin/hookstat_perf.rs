use hookstat::performance::{HarnessConfig, HarnessPaths, run};
use std::path::PathBuf;

fn main() {
    let mut output = None;
    let mut config = HarnessConfig::default();
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = arguments.get(index + 1).map(String::as_str);
        match (flag.as_str(), value) {
            ("--output", Some(value)) => output = Some(PathBuf::from(value)),
            ("--process-iterations", Some(value)) => {
                config.process_iterations = value.parse().unwrap_or(0)
            }
            ("--io-iterations", Some(value)) => config.io_iterations = value.parse().unwrap_or(0),
            ("--pipe-iterations", Some(value)) => {
                config.pipe_iterations = value.parse().unwrap_or(0)
            }
            _ => {
                eprintln!(
                    "usage: hookstat-perf --output <sanitized-json> [--process-iterations <n>] [--io-iterations <n>] [--pipe-iterations <n>]"
                );
                std::process::exit(2);
            }
        }
        index += 2;
    }
    let Some(output) = output else {
        eprintln!("hookstat-perf requires --output");
        std::process::exit(2);
    };
    let result = HarnessPaths::from_current_executable().and_then(|paths| run(&config, &paths));
    match result {
        Ok(receipt) => match serde_json::to_vec_pretty(&receipt)
            .map_err(|_| std::io::Error::other("receipt serialization"))
            .and_then(|bytes| std::fs::write(&output, bytes))
        {
            Ok(()) => println!("G28_SANITIZED_RECEIPT_WRITTEN"),
            Err(_) => {
                eprintln!("hookstat-perf could not write the requested receipt");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("hookstat-perf: {error}");
            std::process::exit(1);
        }
    }
}
