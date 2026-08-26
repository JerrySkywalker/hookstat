#[cfg(windows)]
fn main() -> std::io::Result<()> {
    use interprocess::ConnectWaitMode;
    use interprocess::local_socket::{ConnectOptions, GenericNamespaced, prelude::*};
    use std::io::{Read, Write};
    use std::time::Duration;

    let mut values = std::env::args().skip(1);
    let Some(flag) = values.next() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "bounded endpoint argument required",
        ));
    };
    if flag == "--help" {
        return Ok(());
    }
    let endpoint = values
        .next()
        .filter(|_| flag == "--endpoint")
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "bounded endpoint argument required",
            )
        })?;
    if values.next().is_some() || endpoint.len() > 128 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "bounded endpoint argument required",
        ));
    }
    let name = endpoint.to_ns_name::<GenericNamespaced>()?;
    let mut stream = ConnectOptions::new()
        .name(name)
        .wait_mode(ConnectWaitMode::Timeout(Duration::from_millis(10)))
        .connect_sync()?;
    stream.write_all(b"HSGF\x01\0\0\0")?;
    let mut response = [0_u8; 8];
    stream.read_exact(&mut response)?;
    if response != *b"HSGA\x01\0\0\0" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid bounded helper response",
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    std::process::exit(2);
}
