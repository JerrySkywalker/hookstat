#[cfg(windows)]
fn main() -> std::io::Result<()> {
    use interprocess::local_socket::{
        GenericNamespaced, ListenerNonblockingMode, ListenerOptions, prelude::*,
    };
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};

    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let endpoint = arguments
        .windows(2)
        .find(|values| values[0] == "--endpoint")
        .map(|values| values[1].clone())
        .filter(|value| value.len() <= 128)
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let maximum_connections = arguments
        .windows(2)
        .find(|values| values[0] == "--max-connections")
        .and_then(|values| values[1].parse::<usize>().ok())
        .filter(|value| (1..=10_000).contains(value))
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let idle_expiry_ms = arguments
        .windows(2)
        .find(|values| values[0] == "--idle-expiry-ms")
        .and_then(|values| values[1].parse::<u64>().ok())
        .filter(|value| (100..=600_000).contains(value))
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    if arguments.len() != 6 {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }

    let name = endpoint.to_ns_name::<GenericNamespaced>()?;
    let listener = ListenerOptions::new()
        .name(name)
        .nonblocking(ListenerNonblockingMode::Accept)
        .create_sync()?;
    println!("READY");
    std::io::stdout().flush()?;

    let idle_expiry = Duration::from_millis(idle_expiry_ms);
    let mut last_activity = Instant::now();
    let mut accepted = 0_usize;
    while accepted < maximum_connections && last_activity.elapsed() < idle_expiry {
        match listener.accept() {
            Ok(mut stream) => {
                let mut request = [0_u8; 8];
                stream.read_exact(&mut request)?;
                if request != *b"HSGF\x01\0\0\0" {
                    return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                }
                stream.write_all(b"HSGA\x01\0\0\0")?;
                stream.flush()?;
                accepted += 1;
                last_activity = Instant::now();
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_micros(100));
            }
            Err(error) => return Err(error),
        }
    }
    if accepted != maximum_connections {
        return Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "idle-expiring helper reached its bounded idle deadline",
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    std::process::exit(2);
}
