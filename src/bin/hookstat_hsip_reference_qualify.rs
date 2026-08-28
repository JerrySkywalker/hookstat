//! Feature-gated, sanitized G38B reference-producer performance measurement.

use hookstat::qualification::{ReferenceHsipPerformanceConfig, run_reference_hsip_performance};
use std::path::PathBuf;

fn main() {
    let mut output = None;
    let mut config = ReferenceHsipPerformanceConfig::default();
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = arguments.get(index + 1).map(String::as_str);
        match (flag.as_str(), value) {
            ("--output", Some(value)) => output = Some(PathBuf::from(value)),
            ("--one-producer-frames", Some(value)) => {
                config.one_producer_frames = value.parse().unwrap_or(0)
            }
            ("--five-producer-frames", Some(value)) => {
                config.five_producer_frames_per_producer = value.parse().unwrap_or(0)
            }
            ("--ten-producer-frames", Some(value)) => {
                config.ten_producer_frames_per_producer = value.parse().unwrap_or(0)
            }
            _ => {
                eprintln!(
                    "usage: hookstat-hsip-reference-qualify --output <sanitized-json> [--one-producer-frames <1000..10000>] [--five-producer-frames <200..10000>] [--ten-producer-frames <1000..10000>]"
                );
                std::process::exit(2);
            }
        }
        index += 2;
    }
    let Some(output) = output else {
        eprintln!("hookstat-hsip-reference-qualify requires --output");
        std::process::exit(2);
    };
    match run_reference_hsip_performance(&config) {
        Ok(receipt) => match serde_json::to_vec_pretty(&receipt)
            .map_err(|_| std::io::Error::other("reference HSIP receipt serialization"))
            .and_then(|bytes| std::fs::write(&output, bytes))
        {
            Ok(()) => println!(
                "HSIP_REFERENCE_PERFORMANCE_RECEIPT_WRITTEN={}",
                receipt.performance_gate
            ),
            Err(_) => {
                eprintln!("hookstat-hsip-reference-qualify could not write the requested receipt");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("hookstat-hsip-reference-qualify: {error}");
            std::process::exit(1);
        }
    }
}
