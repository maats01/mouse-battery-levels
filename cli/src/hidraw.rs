//! Minimal hidraw I/O: one open node, write a report, wait for the matching reply.

use crate::protocol::reply_matches;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

const O_NONBLOCK: i32 = 0o4000;
const POLLIN: i16 = 0x001;

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

unsafe extern "C" {
    fn poll(fds: *mut PollFd, nfds: u64, timeout: i32) -> i32;
}

pub struct Hidraw {
    file: File,
}

impl Hidraw {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).custom_flags(O_NONBLOCK).open(path)?;
        Ok(Self { file })
    }

    fn wait_readable(&self, timeout: Duration) -> io::Result<bool> {
        let mut pfd = PollFd { fd: self.file.as_raw_fd(), events: POLLIN, revents: 0 };
        let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        // SAFETY: one valid pollfd for the duration of the call.
        let r = unsafe { poll(&mut pfd, 1, ms) };
        if r < 0 {
            let e = io::Error::last_os_error();
            return if e.kind() == io::ErrorKind::Interrupted { Ok(false) } else { Err(e) };
        }
        Ok(r > 0)
    }

    /// Send `req` and return the first reply that answers it, or `None` on timeout.
    /// Unrelated reports (e.g. unsolicited events) are skipped.
    pub fn exchange(&mut self, req: &[u8], timeout: Duration) -> io::Result<Option<Vec<u8>>> {
        let mut buf = [0u8; 64];
        // Drop stale reports queued before our request.
        while let Ok(n) = self.file.read(&mut buf) {
            if n == 0 {
                break;
            }
        }
        self.file.write_all(req)?;
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || !self.wait_readable(left)? {
                if Instant::now() >= deadline {
                    return Ok(None);
                }
                continue;
            }
            match self.file.read(&mut buf) {
                Ok(n) if reply_matches(req, &buf[..n]) => return Ok(Some(buf[..n].to_vec())),
                Ok(_) => continue,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e),
            }
        }
    }
}
