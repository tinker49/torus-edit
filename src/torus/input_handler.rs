use libc::STDIN_FILENO;
use libc::read;
use std::os::fd::RawFd;
use std::io;



fn editor_read_key(fd: RawFd) -> io::Result<char> {
    let mut buf = [0u8];
    // Use the libc read function to get a single byte
    if unsafe { read(fd, &mut buf as *mut _ as *mut libc::c_void, 1) } == 1 {
        Ok(buf[0] as char)
    } else {
        Err(io::Error::last_os_error())
    }
}

// Map Ctrl key combinations
fn ctrl_key(c: char) -> char {
    // Ctrl key typically strips bits 5 and 6 (bitwise AND with 00011111 binary, or 31 decimal)
    (c as u8 & 0b00011111) as char
}

pub fn process_keypress() -> Option<u8> {

		let c = editor_read_key(STDIN_FILENO).ok()?;

        if c.is_control() {
            println!("Control key pressed. ASCII value: {}\r", c as u8);
        } else if c.is_ascii() {
            println!("ASCII key pressed: '{}', ASCII value: {}\r", c, c as u8);
        } else {
            // Handle non-ASCII or multi-byte characters if needed (omitted for this request)
            println!("Other character pressed: '{}', ASCII value: {}\r", c, c as u8);
        }
        return Some(c as u8);
}

