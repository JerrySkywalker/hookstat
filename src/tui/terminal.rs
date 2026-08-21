//! RAII terminal lifecycle restoration with injectable operations for tests.

use crossterm::{
    cursor::Show,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io;

trait TerminalOperations {
    fn enable_raw_mode(&mut self) -> io::Result<()>;
    fn disable_raw_mode(&mut self) -> io::Result<()>;
    fn enter_alternate_screen(&mut self) -> io::Result<()>;
    fn leave_alternate_screen(&mut self) -> io::Result<()>;
    fn show_cursor(&mut self) -> io::Result<()>;
}

#[derive(Default)]
struct SystemTerminalOperations;

impl TerminalOperations for SystemTerminalOperations {
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        let mut output = io::stdout();
        execute!(output, EnterAlternateScreen)
    }

    fn leave_alternate_screen(&mut self) -> io::Result<()> {
        let mut output = io::stdout();
        execute!(output, LeaveAlternateScreen)
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        let mut output = io::stdout();
        execute!(output, Show)
    }
}

struct LifecycleGuard<O: TerminalOperations> {
    operations: O,
    raw_mode_active: bool,
    alternate_screen_active: bool,
}

/// Owns raw-mode and alternate-screen restoration across all return paths.
pub struct TerminalGuard {
    inner: LifecycleGuard<SystemTerminalOperations>,
}

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        LifecycleGuard::with_operations(SystemTerminalOperations).map(|inner| Self { inner })
    }

    pub fn restore(&mut self) -> io::Result<()> {
        self.inner.restore()
    }
}

impl<O: TerminalOperations> LifecycleGuard<O> {
    fn with_operations(operations: O) -> io::Result<Self> {
        let mut guard = Self {
            operations,
            raw_mode_active: false,
            alternate_screen_active: false,
        };
        guard.operations.enable_raw_mode()?;
        guard.raw_mode_active = true;
        guard.operations.enter_alternate_screen()?;
        guard.alternate_screen_active = true;
        Ok(guard)
    }

    pub fn restore(&mut self) -> io::Result<()> {
        let mut first_error = None;
        if self.alternate_screen_active {
            if let Err(error) = self.operations.show_cursor() {
                first_error = Some(error);
            }
            if let Err(error) = self.operations.leave_alternate_screen()
                && first_error.is_none()
            {
                first_error = Some(error);
            }
            self.alternate_screen_active = false;
        }
        if self.raw_mode_active {
            if let Err(error) = self.operations.disable_raw_mode()
                && first_error.is_none()
            {
                first_error = Some(error);
            }
            self.raw_mode_active = false;
        }
        first_error.map_or(Ok(()), Err)
    }
}

impl<O: TerminalOperations> Drop for LifecycleGuard<O> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct FakeOperations {
        calls: Rc<RefCell<Vec<&'static str>>>,
        fail_enter: bool,
    }

    impl TerminalOperations for FakeOperations {
        fn enable_raw_mode(&mut self) -> io::Result<()> {
            self.calls.borrow_mut().push("raw-on");
            Ok(())
        }
        fn disable_raw_mode(&mut self) -> io::Result<()> {
            self.calls.borrow_mut().push("raw-off");
            Ok(())
        }
        fn enter_alternate_screen(&mut self) -> io::Result<()> {
            self.calls.borrow_mut().push("alt-on");
            if self.fail_enter {
                Err(io::Error::other("injected"))
            } else {
                Ok(())
            }
        }
        fn leave_alternate_screen(&mut self) -> io::Result<()> {
            self.calls.borrow_mut().push("alt-off");
            Ok(())
        }
        fn show_cursor(&mut self) -> io::Result<()> {
            self.calls.borrow_mut().push("show-cursor");
            Ok(())
        }
    }

    #[test]
    fn guard_restores_every_terminal_mode() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let operations = FakeOperations {
            calls: Rc::clone(&calls),
            fail_enter: false,
        };
        drop(LifecycleGuard::with_operations(operations).unwrap());
        assert_eq!(
            *calls.borrow(),
            ["raw-on", "alt-on", "show-cursor", "alt-off", "raw-off"]
        );
    }

    #[test]
    fn partial_entry_restores_raw_mode() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let operations = FakeOperations {
            calls: Rc::clone(&calls),
            fail_enter: true,
        };
        assert!(LifecycleGuard::with_operations(operations).is_err());
        assert_eq!(*calls.borrow(), ["raw-on", "alt-on", "raw-off"]);
    }
}
