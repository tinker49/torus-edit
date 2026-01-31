use libc::{
    tcgetattr, tcsetattr, termios as Termios, ECHO, ICANON, TCSANOW, VMIN, VTIME, STDOUT_FILENO, c_void, ISIG , IEXTEN, ICRNL
};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::{mem};
use crate::torus::input_handler;

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
        raw_termios.c_lflag &= !(ECHO | ICANON | ISIG | IEXTEN);
    	raw_termios.c_iflag &= !(ICRNL);
        
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

pub fn run_app_in_raw_mode() {
    let _guard = match RawModeGuard::enable_raw_mode() {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("Failed to enable raw mode: {}", err);
            return;
        }
    };
    
    clear_screen();

    println!("Type characters. Press 'q' to quit, or hit Ctrl-C/Panic to test Drop guard.");

    let mut stdin = io::stdin();
    let mut byte = [0; 1];

    loop {
        if stdin.read_exact(&mut byte).is_ok() {
            //let char_byte = byte[0];
            
            let char_byte = input_handler::process_keypress();

			if Some(char_byte).is_some() {
				let char_byte_val = char_byte.unwrap();
            	// Echo character back manually
            	io::stdout().write_all(&[char_byte_val]).unwrap();
            	io::stdout().flush().unwrap();

            	if char_byte_val == b'q' {
                	clear_screen();
                	break; // Exits loop, guard drops, mode restored
            	}
			}
            // Uncomment the following line to simulate a panic:
            // if char_byte == b'p' {
            //     panic!("Simulating a panic to test the Drop guard!");
            // }
        }
    }
}

pub fn clear_screen() {
    // \x1B[2J - Clear entire screen
    // \x1B[H  - Move cursor to top-left (home)
    let sequence = b"\x1B[2J\x1B[H";
    unsafe {
        libc::write(
            STDOUT_FILENO,
            sequence.as_ptr() as *const c_void,
            sequence.len(),
        );
    }
}


#[cfg(test)]
mod panic_tests {
    use super::*;
    use libc::{tcgetattr, termios, STDIN_FILENO};
    use std::panic;
    use std::mem;

    #[test]
    fn test_raw_mode_guard_restores_on_panic() {
        let mut original_term: termios = unsafe { mem::zeroed() };
        
        // Skip if not a TTY (standard for CI environments)
        if unsafe { tcgetattr(STDIN_FILENO, &mut original_term) } != 0 {
            return;
        }

        // Use catch_unwind to trap the panic and allow the test to continue
        let result = panic::catch_unwind(|| {
            let _guard = RawModeGuard::enable_raw_mode().expect("Failed to enter raw mode");
            
            // Verify we are actually in raw mode before panicking
            let mut raw_term: termios = unsafe { mem::zeroed() };
            unsafe { tcgetattr(STDIN_FILENO, &mut raw_term) };
            assert_ne!(raw_term.c_lflag, original_term.c_lflag);

            panic!("Intentional panic during raw mode");
        });

        // The result should be an Err because of the panic
        assert!(result.is_err());

        // Verify the terminal has been restored to its original state
        let mut restored_term: termios = unsafe { mem::zeroed() };
        unsafe { tcgetattr(STDIN_FILENO, &mut restored_term) };
        
        assert_eq!(
            restored_term.c_lflag, 
            original_term.c_lflag, 
            "Terminal state was not restored after panic"
        );
    }
    
    #[test]
    fn test_raw_mode_manual_lifecycle() {
        // 1. Capture the initial state of the terminal.
        // tcgetattr returns -1 if STDIN is not a TTY (standard for CI).
        let mut original_term: termios = unsafe { mem::zeroed() };
        if unsafe { tcgetattr(STDIN_FILENO, &mut original_term) } != 0 {
            eprintln!("Skipping: STDIN is not a terminal.");
            return;
        }

        // 2. Manually enter raw mode by creating the guard.
        let guard = RawModeGuard::enable_raw_mode().expect("Failed to enter raw mode");

        // 3. Inspect the terminal state while the guard is alive.
        let mut raw_term: termios = unsafe { mem::zeroed() };
        unsafe { tcgetattr(STDIN_FILENO, &mut raw_term) };

        // Verify that ICANON and ECHO are disabled (0).
        assert_eq!(
            raw_term.c_lflag & (ICANON | ECHO), 
            0, 
            "Terminal should be in raw mode (ICANON/ECHO disabled)"
        );

        // 4. Manually drop the guard to trigger terminal restoration.
        drop(guard);

        // 5. Verify the terminal state has returned to the original configuration.
        let mut restored_term: termios = unsafe { mem::zeroed() };
        unsafe { tcgetattr(STDIN_FILENO, &mut restored_term) };
        
        assert_eq!(
            restored_term.c_lflag, 
            original_term.c_lflag, 
            "Terminal state was not restored after dropping the guard"
        );
    }    

}

