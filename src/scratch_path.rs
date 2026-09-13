//! Narrow validation for HookStat-owned disposable development state.
//!
//! This deliberately does not select a governed root. It only rejects the
//! unsafe Windows locations that would put HookStat project state at `C:\` or
//! immediately below it.

use std::io;
use std::path::Path;

/// Reject a HookStat-created scratch destination at `C:\` or directly below it.
///
/// The check is lexical so it does not require the candidate to exist and never
/// creates it. It recognizes ordinary Windows absolute-path spellings even
/// when the test host is not Windows.
pub fn assert_safe_scratch_destination(candidate: &Path) -> io::Result<()> {
    if is_unsafe_direct_c_root_destination(candidate) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "HookStat scratch destination '{}' is prohibited: direct C:\\ project/temp roots are prohibited; choose a normal temporary or build location",
                candidate.display()
            ),
        ));
    }
    Ok(())
}

fn is_unsafe_direct_c_root_destination(candidate: &Path) -> bool {
    let value = candidate.to_string_lossy().replace('/', "\\");
    let bytes = value.as_bytes();
    if bytes.len() < 2 || !bytes[0].eq_ignore_ascii_case(&b'C') || bytes[1] != b':' {
        return false;
    }
    if bytes.len() < 3 || bytes[2] != b'\\' {
        return true;
    }

    let mut components = Vec::new();
    for component in value[3..].split('\\') {
        match component {
            "" | "." => {}
            ".." => {
                let _ = components.pop();
            }
            value => components.push(value),
        }
    }
    components.len() <= 1
}

#[cfg(test)]
mod tests {
    use super::assert_safe_scratch_destination;
    use std::path::Path;

    #[test]
    fn rejects_direct_c_root_destinations_without_creating_them() {
        for candidate in [
            r"C:\",
            r"C:\hookstat-review",
            r"C:\hookstat-temp-lab",
            r"C:\hookstat-target",
            r"c:\hookstat-temp-lab",
            "C:/hookstat-temp-lab",
            r"C:\.\hookstat-temp-lab",
            r"C:\hookstat-parent\..\hookstat-temp-lab",
            r"C:hookstat-temp-lab",
        ] {
            let error = assert_safe_scratch_destination(Path::new(candidate)).unwrap_err();
            assert!(error.to_string().contains("direct C:\\ project/temp roots"));
        }
    }

    #[test]
    fn accepts_non_root_child_destinations() {
        for candidate in [
            r"C:\Users\test\AppData\Local\Temp\hookstat-lab",
            r"D:\temp\hookstat-lab",
            r"C:\hookstat-parent\child",
            "relative-test-root/hookstat-lab",
        ] {
            assert!(assert_safe_scratch_destination(Path::new(candidate)).is_ok());
        }
    }
}
