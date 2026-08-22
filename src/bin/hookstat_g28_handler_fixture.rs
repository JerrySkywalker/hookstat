//! Disposable G28 handler fixture. It never reads stdin or writes output.

use std::time::Duration;

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() == 2
        && arguments[0] == "--sleep-ms"
        && let Ok(milliseconds) = arguments[1].parse::<u64>()
    {
        std::thread::sleep(Duration::from_millis(milliseconds.min(10_000)));
    }
}
