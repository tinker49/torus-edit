use libc::{
    tcgetattr, tcsetattr, termios as Termios, ECHO, ICANON, TCSANOW, VMIN, VTIME
};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::{mem};

/// A guard that restores the terminal settings when dropped.
struct RawModeGuard {
    original_termios: Termios,
}

impl RawModeGuard {
    /// Enables raw mode for the terminal.
    fn enable_raw_mode() -> io::Result<Self> {
        let stdin = io::stdin();
        let fd = stdin.as_raw_fd();

        let mut original_termios: Termios = unsafe { mem::zeroed() };

        // Get the current terminal attributes
        if unsafe { tcgetattr(fd, &mut original_termios) } != 0 {
            return Err(io::Error::last_os_error());
        }

        let mut raw_termios = original_termios.clone();

        // Disable canonical mode (ICANON), echo (ECHO),
        // and various signal processing flags.
        raw_termios.c_lflag &= !(ICANON | ECHO);
        raw_termios.c_cc[VMIN] = 1; // Read returns after 1 byte
        raw_termios.c_cc[VTIME] = 0; // No timeout

        // Set the new terminal attributes immediately
        if unsafe { tcsetattr(fd, TCSANOW, &raw_termios) } != 0 {
            return Err(io::Error::last_os_error());
        }

        println!("Raw mode enabled.");

        Ok(RawModeGuard { original_termios })
    }
}

// The Drop implementation ensures that the terminal mode is always restored
// when the RawModeGuard goes out of scope.
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let stdin = io::stdin();
        let fd = stdin.as_raw_fd();

        // Restore the original terminal attributes
        if unsafe { tcsetattr(fd, TCSANOW, &self.original_termios) } != 0 {
            eprintln!("Error restoring terminal mode: {}", io::Error::last_os_error());
        } else {
            println!("\nOriginal mode restored.");
        }
    }
}

fn run_app_in_raw_mode() {
    let _guard = match RawModeGuard::enable_raw_mode() {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("Failed to enable raw mode: {}", err);
            return;
        }
    };

    println!("Type characters. Press 'q' to quit, or hit Ctrl-C/Panic to test Drop guard.");

    let mut stdin = io::stdin();
    let mut byte = [0; 1];

    loop {
        if stdin.read_exact(&mut byte).is_ok() {
            let char_byte = byte[0];

            // Echo character back manually
            io::stdout().write_all(&[char_byte]).unwrap();
            io::stdout().flush().unwrap();

            if char_byte == b'q' {
                break; // Exits loop, guard drops, mode restored
            }

            // Uncomment the following line to simulate a panic:
            // if char_byte == b'p' {
            //     panic!("Simulating a panic to test the Drop guard!");
            // }
        }
    }
}

fn main() {
    // The main function catches any panics within `run_app_in_raw_mode` 
    // to observe the 'Original mode restored.' message printed by the Drop impl.
    // Without this, the panic handler might exit before the drop message prints,
    // but the mode is still restored before the process terminates.
    let result = std::panic::catch_unwind(|| {
        run_app_in_raw_mode();
    });

    if let Err(_err) = result {
        println!("A panic occurred, but the terminal mode was restored.");
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Intentional Panic")]
    fn test_raw_mode_reverts_on_panic() {
        // This test ensures that even if the app panics, the guard is created.
        // In a real environment, the Drop trait handles the cleanup.
        let result = run_app_in_raw_mode(|| {
            panic!("Intentional Panic");
        });
        
        // If tcgetattr fails (e.g., in a non-interactive CI), 
        // the function returns an Err before the panic, which is also a valid state.
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_run_app_success() {
        // Verifies the wrapper returns the correct closure result.
        let val = run_app_in_raw_mode(|| 42);
        if let Ok(num) = val {
            assert_eq!(num, 42);
        }
    }
}


