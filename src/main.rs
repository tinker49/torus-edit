extern crate libc;

use std::io;
use std::os::unix::io::AsRawFd;
use std::io::Read;

fn set_raw_mode() -> Result<libc::termios, io::Error> {
    unsafe {
        let fd = io::stdin().as_raw_fd();
        let mut termios = std::mem::zeroed();

        // 1. Get current terminal attributes
        if libc::tcgetattr(fd, &mut termios) != 0 {
            return Err(io::Error::last_os_error());
        }

        // Store original to restore later
        let original = termios;

        // 2. Disable ICANON (canonical mode), ECHO, and ISIG (signals)
        termios.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);

        // 3. Apply new attributes
        if libc::tcsetattr(fd, libc::TCSANOW, &termios) != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(original)
    }
}

fn restore_mode(original: libc::termios) {
    unsafe {
        let fd = io::stdin().as_raw_fd();
        libc::tcsetattr(fd, libc::TCSANOW, &original);
    }
}

fn main() {
    match set_raw_mode() {
        Ok(orig) => {
            println!("Raw mode set. Type characters (Ctrl+C won't exit). Press Enter to exit.");
            
            // Read raw characters
            let mut buf = [0; 1];
            while let Ok(_) = io::stdin().read(&mut buf) {
                if buf[0] == b'\n' { break; }
                print!("Pressed: {}\r\n", buf[0] as char);
            }
            
            restore_mode(orig);
            println!("Restored mode.");
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use libc::{tcgetattr, tcsetattr, termios, ECHO, ICANON, TCSANOW};
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_terminal_raw_mode() {
        unsafe {
            let fd = std::io::stdin().as_raw_fd();
            let mut original: termios = std::mem::zeroed();
            
            // Get original settings
            assert_eq!(tcgetattr(fd, &mut original), 0, "Failed to get termios");

            // Create raw settings
            let mut raw = original;
            raw.c_lflag &= !(ICANON | ECHO); // Disable canonical mode and echo
            
            // Apply raw mode
            assert_eq!(tcsetattr(fd, TCSANOW, &raw), 0, "Failed to set raw mode");

            // Verify raw mode
            let mut current: termios = std::mem::zeroed();
            tcgetattr(fd, &mut current);
            assert_eq!(current.c_lflag & (ICANON | ECHO), 0, "Terminal not in raw mode");

            // Restore original settings
            tcsetattr(fd, TCSANOW, &original);
        }
    }
}


