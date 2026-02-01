use libc::STDIN_FILENO;
use libc::tcflush;
use libc::TCIFLUSH;
use libc::read;
use std::os::fd::RawFd;
use std::io;
use std::io::Write;


// Key codes for Control and Alt (Linux evdev codes)
const KEY_LEFTCTRL: u16 = 29;
const KEY_RIGHTCTRL: u16 = 97;
const KEY_LEFTALT: u16 = 56;
const KEY_RIGHTALT: u16 = 100;


fn editor_read_key(fd: RawFd) -> io::Result<char> {
    let mut buf = [0u8];
    // Use the libc read function to get a single byte
    if unsafe { read(fd, &mut buf as *mut _ as *mut libc::c_void, 1) } == 1 {
        Ok(buf[0] as char)
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Returns true if Control is currently pressed
fn is_control_pressed(this_key: u16) -> bool {
	if this_key == KEY_LEFTCTRL {
		return true;
	} else if this_key == KEY_RIGHTCTRL {
		return true;
	} else {
		return false;
	}
}



pub fn process_keypress() -> Option<u8> {

		let c = editor_read_key(STDIN_FILENO).ok()?;
		std::io::stdout().flush().unwrap();
		if c as u8 == 27 {
            println!("Alt key pressed. ASCII value: {}\r", c as u8);
            let d = editor_read_key(STDIN_FILENO).ok()?;
            println!("key pressed after alt. ASCII value: {}\r", d as u8);
        } else if c as u8 >= 1 && c as u8 <= 26 {
            println!("Control key pressed. ASCII value: {}\r", c as u8);
        } else if c as u8 >= 32 && c as u8 <= 122 {
            println!("ASCII key pressed: '{}', ASCII value: {}\r", c, c as u8);
            unsafe {
        		tcflush(0, TCIFLUSH);
    		}
        } else {
            // Handle non-ASCII or multi-byte characters if needed (omitted for this request)
            println!("Other character pressed: '{}', ASCII value: {}\r", c, c as u8);
        }
        std::io::stdout().flush().unwrap();
        return Some(c as u8);
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::io::AsRawFd;
    use std::os::unix::io::RawFd;

    // Helper function to create a temporary file descriptor with data
    fn setup_pipe_with_data(data: &[u8]) -> (RawFd, std::fs::File) {
        // Create a named pipe or a temporary file. 
        // For simplicity in testing, using a file is easier.
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut file_handle = file.reopen().unwrap();
        file_handle.write_all(data).unwrap();
        
        // Reset file pointer to the beginning for reading
        let file_handle = file.reopen().unwrap();
        (file_handle.as_raw_fd(), file_handle)
    }

    #[test]
    fn test_editor_read_key_success() {
        let (fd, _file) = setup_pipe_with_data(b"a");
        
        let result = editor_read_key(fd);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 'a');
    }

    #[test]
    fn test_editor_read_key_eof() {
        // Create an empty file
        let file = tempfile::NamedTempFile::new().unwrap();
        let file_handle = file.reopen().unwrap();
        let fd = file_handle.as_raw_fd();

        let result = editor_read_key(fd);
        // Depending on implementation, EOF might return an error or 0 bytes.
        // Assuming your code handles EOF via the `else` block (last_os_error).
        assert!(result.is_err());
    }

    #[test]
    fn test_editor_read_key_special_char() {
        let (fd, _file) = setup_pipe_with_data(b"\x1B"); // ESC key
        
        let result = editor_read_key(fd);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), '\u{1B}');
    }
}
