use hookstat::qualification::{CollectorComparisonConfig, run_collector_comparison};
use std::path::PathBuf;

fn main() {
    let mut output = None;
    let mut config = CollectorComparisonConfig::default();
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = arguments.get(index + 1).map(String::as_str);
        match (flag.as_str(), value) {
            ("--output", Some(value)) => output = Some(PathBuf::from(value)),
            ("--single-samples", Some(value)) => config.single_samples = value.parse().unwrap_or(0),
            ("--client16-samples", Some(value)) => {
                config.client16_samples_per_client = value.parse().unwrap_or(0)
            }
            _ => {
                eprintln!(
                    "usage: hookstat-ipc-collector-ab --output <sanitized-json> [--single-samples <100..10000>] [--client16-samples <100..10000>]"
                );
                std::process::exit(2);
            }
        }
        index += 2;
    }
    let Some(output) = output else {
        eprintln!("hookstat-ipc-collector-ab requires --output");
        std::process::exit(2);
    };
    match run_collector_comparison(&config) {
        Ok(receipt) => match serde_json::to_vec_pretty(&receipt)
            .map_err(|_| std::io::Error::other("collector comparison receipt serialization"))
            .and_then(|bytes| std::fs::write(&output, bytes))
        {
            Ok(()) => println!("G35_COLLECTOR_COMPARISON_RECEIPT_WRITTEN"),
            Err(_) => {
                eprintln!("hookstat-ipc-collector-ab could not write the requested receipt");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("hookstat-ipc-collector-ab: {error}");
            std::process::exit(1);
        }
    }
}
