mod device;
mod hidraw;
mod protocol;
mod status;

use protocol::Path;
use status::{ErrKind, Status};
use std::io;

/// One request/response round trip. `Ok(None)` means no reply before the timeout.
type Exchange<'a> = dyn FnMut(&[u8]) -> io::Result<Option<Vec<u8>>> + 'a;

/// Ask the receiver whether the mouse is awake; only then ask for the battery,
/// so a sleeping mouse is never contacted.
fn run(mode: Path, exchange: &mut Exchange) -> Status {
    let online = match exchange(&protocol::online_request()) {
        Ok(Some(r)) => protocol::parse_online(&r),
        Ok(None) => return Status::Disconnected,
        Err(_) => return Status::Error(ErrKind::Io),
    };
    match online {
        Ok(true) => {}
        Ok(false) => return Status::Disconnected,
        Err(_) => return Status::Error(ErrKind::Io),
    }
    match exchange(&protocol::battery_request()) {
        Ok(Some(r)) => match protocol::parse_battery(&r) {
            Ok(b) => Status::Ok { mode, battery: b.battery, charging: b.charging },
            Err(_) => Status::Error(ErrKind::Io),
        },
        Ok(None) => Status::Disconnected,
        Err(_) => Status::Error(ErrKind::Io),
    }
}

fn open_error(e: &io::Error) -> Status {
    match e.kind() {
        io::ErrorKind::PermissionDenied => Status::Error(ErrKind::Permission),
        io::ErrorKind::NotFound => Status::Error(ErrKind::NoReceiver),
        _ => Status::Error(ErrKind::Io),
    }
}

const TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);

fn status() -> Status {
    let Some((mode, node)) = device::find(std::path::Path::new("/sys/class/hidraw")) else {
        return Status::Error(ErrKind::NoReceiver);
    };
    match hidraw::Hidraw::open(&node) {
        Ok(mut dev) => run(mode, &mut |req| dev.exchange(req, TIMEOUT)),
        Err(e) => open_error(&e),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 1 || args[0] != "status" {
        eprintln!("uso: vxe status");
        std::process::exit(2);
    }
    let s = status();
    println!("{}", s.to_json());
    std::process::exit(s.exit_code());
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::hex;

    const WIRELESS_30: &str = include_str!("../tests/fixtures/wireless_30.hex");
    const WIRED_50: &str = include_str!("../tests/fixtures/wired_charging_50.hex");
    const ONLINE: &str = include_str!("../tests/fixtures/online_yes.hex");
    const OFFLINE: &str = include_str!("../tests/fixtures/online_no.hex");

    /// Serve canned replies keyed by command byte; record which commands were sent.
    fn serve<'a>(
        replies: &'a [(u8, io::Result<Option<Vec<u8>>>)],
        sent: &'a mut Vec<u8>,
    ) -> impl FnMut(&[u8]) -> io::Result<Option<Vec<u8>>> + 'a {
        move |req| {
            sent.push(req[1]);
            match replies.iter().find(|(c, _)| *c == req[1]) {
                Some((_, Ok(r))) => Ok(r.clone()),
                Some((_, Err(e))) => Err(io::Error::new(e.kind(), "x")),
                None => Ok(None),
            }
        }
    }

    #[test]
    fn run_ok_wireless() {
        let replies = [(0x03, Ok(Some(hex(ONLINE)))), (0x04, Ok(Some(hex(WIRELESS_30))))];
        let mut sent = vec![];
        let s = run(Path::Wireless, &mut serve(&replies, &mut sent));
        assert_eq!(s, Status::Ok { mode: Path::Wireless, battery: 30, charging: false });
        assert_eq!(sent, vec![0x03, 0x04]);
    }

    #[test]
    fn run_ok_wired_charging() {
        let replies = [(0x03, Ok(Some(hex(ONLINE)))), (0x04, Ok(Some(hex(WIRED_50))))];
        let mut sent = vec![];
        let s = run(Path::Wired, &mut serve(&replies, &mut sent));
        assert_eq!(s, Status::Ok { mode: Path::Wired, battery: 50, charging: true });
    }

    #[test]
    fn run_sleeping_never_asks_battery() {
        let replies = [(0x03, Ok(Some(hex(OFFLINE)))), (0x04, Ok(Some(hex(WIRELESS_30))))];
        let mut sent = vec![];
        let s = run(Path::Wireless, &mut serve(&replies, &mut sent));
        assert_eq!(s, Status::Disconnected);
        assert_eq!(sent, vec![0x03]);
    }

    #[test]
    fn run_online_timeout_is_disconnected() {
        let replies = [(0x03, Ok(None))];
        let mut sent = vec![];
        assert_eq!(run(Path::Wireless, &mut serve(&replies, &mut sent)), Status::Disconnected);
        assert_eq!(sent, vec![0x03]);
    }

    #[test]
    fn run_battery_timeout_is_disconnected() {
        let replies = [(0x03, Ok(Some(hex(ONLINE)))), (0x04, Ok(None))];
        let mut sent = vec![];
        assert_eq!(run(Path::Wireless, &mut serve(&replies, &mut sent)), Status::Disconnected);
    }

    #[test]
    fn run_garbage_reply_is_io_error() {
        let replies = [(0x03, Ok(Some(hex(ONLINE)))), (0x04, Ok(Some(vec![0x08, 0x04])))];
        let mut sent = vec![];
        assert_eq!(run(Path::Wireless, &mut serve(&replies, &mut sent)), Status::Error(ErrKind::Io));
    }

    #[test]
    fn run_io_error() {
        let replies = [(0x03, Err(io::Error::from(io::ErrorKind::BrokenPipe)))];
        let mut sent = vec![];
        assert_eq!(run(Path::Wireless, &mut serve(&replies, &mut sent)), Status::Error(ErrKind::Io));
    }

    #[test]
    fn open_permission_denied() {
        let e = io::Error::from(io::ErrorKind::PermissionDenied);
        assert_eq!(open_error(&e), Status::Error(ErrKind::Permission));
    }

    #[test]
    fn open_not_found_is_no_receiver() {
        let e = io::Error::from(io::ErrorKind::NotFound);
        assert_eq!(open_error(&e), Status::Error(ErrKind::NoReceiver));
    }

    #[test]
    fn open_other_is_io() {
        let e = io::Error::from(io::ErrorKind::Other);
        assert_eq!(open_error(&e), Status::Error(ErrKind::Io));
    }
}
