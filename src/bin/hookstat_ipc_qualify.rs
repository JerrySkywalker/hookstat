use hookstat::qualification::{QualificationConfig, run_g35};
use std::path::PathBuf;

fn main() {
    let mut output = None;
    let mut config = QualificationConfig::default();
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = arguments.get(index + 1).map(String::as_str);
        match (flag.as_str(), value) {
            ("--output", Some(value)) => output = Some(PathBuf::from(value)),
            ("--max-attempts", Some(value)) => config.max_attempts = value.parse().unwrap_or(0),
            ("--wait-ms", Some(value)) => config.wait_interval_ms = value.parse().unwrap_or(0),
            ("--control-samples", Some(value)) => {
                config.control_samples = value.parse().unwrap_or(0)
            }
            ("--single-samples", Some(value)) => config.single_samples = value.parse().unwrap_or(0),
            ("--client16-samples", Some(value)) => {
                config.client16_samples_per_client = value.parse().unwrap_or(0)
            }
            _ => {
                eprintln!(
                    "usage: hookstat-ipc-qualify --output <sanitized-json> [--max-attempts <5..720>] [--wait-ms <1000..60000>] [--control-samples <100..10000>] [--single-samples <100..10000>] [--client16-samples <100..10000>]"
                );
                std::process::exit(2);
            }
        }
        index += 2;
    }
    let Some(output) = output else {
        eprintln!("hookstat-ipc-qualify requires --output");
        std::process::exit(2);
    };
    match run_g35(&config) {
        Ok(receipt) => match serde_json::to_vec_pretty(&receipt)
            .map_err(|_| std::io::Error::other("qualification receipt serialization"))
            .and_then(|bytes| std::fs::write(&output, bytes))
        {
            Ok(()) => println!("G35_QUALIFICATION_RECEIPT_WRITTEN={}", receipt.outcome),
            Err(_) => {
                eprintln!("hookstat-ipc-qualify could not write the requested receipt");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("hookstat-ipc-qualify: {error}");
            std::process::exit(1);
        }
    }
}
