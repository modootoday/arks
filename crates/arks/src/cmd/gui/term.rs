//! Terminal plumbing for the full-screen views: raw mode (termios through the
//! `libc` dependency on Unix), window size, and key decoding of the bytes a
//! terminal sends (ANSI / VT100 escape sequences).

use std::io::Read;

/// One decoded key press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Enter,
    Tab,
    Backspace,
    Esc,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    /// Input closed.
    Eof,
}

/// Decodes key presses from a byte stream. With `tty`, a zero-byte read is
/// the raw-mode read timeout (keep waiting); otherwise it is end of input.
pub struct Keys<R: Read> {
    r: R,
    tty: bool,
    eof: bool,
    /// A byte read ahead after a lone Esc.
    pending: Option<u8>,
}

impl<R: Read> Keys<R> {
    pub fn new(r: R, tty: bool) -> Self {
        Keys {
            r,
            tty,
            eof: false,
            pending: None,
        }
    }

    /// One byte; `wait` blocks through raw-mode timeouts.
    fn byte(&mut self, wait: bool) -> Option<u8> {
        if let Some(b) = self.pending.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut b = [0u8; 1];
        loop {
            match self.r.read(&mut b) {
                Ok(1) => return Some(b[0]),
                Ok(_) if self.tty && wait => continue,
                Ok(_) if self.tty => return None,
                Ok(_) => {
                    self.eof = true;
                    return None;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Next key press.
    pub fn next_key(&mut self) -> Key {
        let Some(b) = self.byte(true) else {
            return Key::Eof;
        };
        match b {
            b'\r' | b'\n' => Key::Enter,
            b'\t' => Key::Tab,
            0x7f | 0x08 => Key::Backspace,
            0x1b => self.escape(),
            1..=26 => Key::Ctrl((b'a' + b - 1) as char),
            _ => self.utf8(b),
        }
    }

    fn escape(&mut self) -> Key {
        let Some(b) = self.byte(false) else {
            return Key::Esc;
        };
        if b != b'[' && b != b'O' {
            self.pending = Some(b);
            return Key::Esc;
        }
        let mut params = Vec::new();
        loop {
            let Some(c) = self.byte(false) else {
                return Key::Esc;
            };
            match c {
                b'A' => return Key::Up,
                b'B' => return Key::Down,
                b'C' => return Key::Right,
                b'D' => return Key::Left,
                b'H' => return Key::Home,
                b'F' => return Key::End,
                b'~' => {
                    return match params.as_slice() {
                        [b'5'] => Key::PageUp,
                        [b'6'] => Key::PageDown,
                        [b'1'] | [b'7'] => Key::Home,
                        [b'4'] | [b'8'] => Key::End,
                        _ => Key::Esc,
                    };
                }
                b'0'..=b'9' | b';' => params.push(c),
                _ => return Key::Esc,
            }
        }
    }

    fn utf8(&mut self, first: u8) -> Key {
        let len = match first {
            0x00..=0x7f => 1,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            _ => 4,
        };
        let mut buf = vec![first];
        for _ in 1..len {
            match self.byte(false) {
                Some(b) => buf.push(b),
                None => break,
            }
        }
        String::from_utf8(buf)
            .ok()
            .and_then(|s| s.chars().next())
            .map(Key::Char)
            .unwrap_or(Key::Char('?'))
    }
}

/// Whether standard input and output are both terminals.
pub fn is_terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// Terminal size as (columns, rows); 80x24 when unknown.
pub fn size() -> (usize, usize) {
    #[cfg(unix)]
    {
        // SAFETY: TIOCGWINSZ fills a plain struct; fd 1 is ours.
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        let ok = unsafe { libc::ioctl(1, libc::TIOCGWINSZ, &mut ws) } == 0;
        if ok && ws.ws_col > 0 && ws.ws_row > 0 {
            return (usize::from(ws.ws_col), usize::from(ws.ws_row));
        }
    }
    let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse().ok());
    (
        env("COLUMNS").unwrap_or(80).max(40),
        env("LINES").unwrap_or(24).max(12),
    )
}

/// Raw mode on standard input for the guard's lifetime (no echo, no line
/// buffering, no signal keys; reads time out after 0.1 s).
pub struct RawMode {
    #[cfg(unix)]
    saved: Option<libc::termios>,
}

impl RawMode {
    pub fn enter() -> Self {
        #[cfg(unix)]
        {
            // SAFETY: tcgetattr/tcsetattr on fd 0 with a zeroed, then filled, termios.
            let mut t: libc::termios = unsafe { std::mem::zeroed() };
            if unsafe { libc::tcgetattr(0, &mut t) } != 0 {
                return RawMode { saved: None };
            }
            let saved = t;
            t.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG | libc::IEXTEN);
            t.c_iflag &= !(libc::IXON | libc::ICRNL);
            t.c_cc[libc::VMIN] = 0;
            t.c_cc[libc::VTIME] = 1;
            unsafe { libc::tcsetattr(0, libc::TCSAFLUSH, &t) };
            RawMode { saved: Some(saved) }
        }
        #[cfg(not(unix))]
        {
            RawMode {}
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(t) = self.saved {
            // SAFETY: restores the attributes read in `enter`.
            unsafe { libc::tcsetattr(0, libc::TCSAFLUSH, &t) };
        }
    }
}

/// Read one line from the terminal with echo off (password prompts).
pub fn read_hidden_line() -> std::io::Result<Option<String>> {
    #[cfg(unix)]
    {
        // SAFETY: as in RawMode, only ECHO is cleared and restored afterwards.
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        let have = unsafe { libc::tcgetattr(0, &mut t) } == 0;
        let saved = t;
        if have {
            t.c_lflag &= !libc::ECHO;
            unsafe { libc::tcsetattr(0, libc::TCSAFLUSH, &t) };
        }
        let line = read_line_stdin();
        if have {
            unsafe { libc::tcsetattr(0, libc::TCSAFLUSH, &saved) };
        }
        line
    }
    #[cfg(not(unix))]
    {
        read_line_stdin()
    }
}

/// One line from standard input without its line ending; `None` at EOF.
pub fn read_line_stdin() -> std::io::Result<Option<String>> {
    let mut s = String::new();
    if std::io::stdin().read_line(&mut s)? == 0 {
        return Ok(None);
    }
    Ok(Some(s.trim_end_matches(['\r', '\n']).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(bytes: &[u8]) -> Vec<Key> {
        let mut k = Keys::new(bytes, false);
        let mut out = Vec::new();
        loop {
            let key = k.next_key();
            if key == Key::Eof {
                return out;
            }
            out.push(key);
        }
    }

    #[test]
    fn decodes_keys() {
        assert_eq!(
            keys(b"j\x1b[A\x1b[B\r\t\x7f\x03\x1b[5~\x1b[6~"),
            vec![
                Key::Char('j'),
                Key::Up,
                Key::Down,
                Key::Enter,
                Key::Tab,
                Key::Backspace,
                Key::Ctrl('c'),
                Key::PageUp,
                Key::PageDown,
            ]
        );
        assert_eq!(keys(b"\x1b"), vec![Key::Esc]);
        assert_eq!(keys(b"\x1bc"), vec![Key::Esc, Key::Char('c')]);
        assert_eq!(keys("é".as_bytes()), vec![Key::Char('é')]);
    }
}
