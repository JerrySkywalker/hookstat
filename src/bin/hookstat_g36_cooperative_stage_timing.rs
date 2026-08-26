//! Feature-gated, sanitized G36 cooperative connection-lifecycle probe.

use hookstat::qualification::{
    G36CooperativeStageTimingConfig, run_g36_cooperative_stage_timing_diagnostic,
};
use std::path::PathBuf;

fn main() {
    let mut output = None;
    let mut config = G36CooperativeStageTimingConfig::default();
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = arguments.get(index + 1).map(String::as_str);
        match (flag.as_str(), value) {
            ("--output", Some(value)) => output = Some(PathBuf::from(value)),
            ("--samples", Some(value)) => config.samples = value.parse().unwrap_or(0),
            _ => {
                eprintln!(
                    "usage: hookstat-g36-cooperative-stage-timing --output <sanitized-json> [--samples <100..10000>]"
                );
                std::process::exit(2);
            }
        }
        index += 2;
    }
    let Some(output) = output else {
        eprintln!("hookstat-g36-cooperative-stage-timing requires --output");
        std::process::exit(2);
    };
    match run_g36_cooperative_stage_timing_diagnostic(&config) {
        Ok(receipt) => match serde_json::to_vec_pretty(&receipt)
            .map_err(|_| std::io::Error::other("G36 stage timing receipt serialization"))
            .and_then(|bytes| std::fs::write(&output, bytes))
        {
            Ok(()) => println!("G36_COOPERATIVE_STAGE_TIMING_RECEIPT_WRITTEN"),
            Err(_) => {
                eprintln!("hookstat-g36-cooperative-stage-timing could not write the receipt");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("hookstat-g36-cooperative-stage-timing: {error}");
            std::process::exit(1);
        }
    }
}
