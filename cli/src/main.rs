mod device;
mod dpi;
mod hidraw;
mod protocol;
mod status;

use protocol::Path;
use status::{ErrKind, Stages, Status};
use std::io;

/// One request/response round trip. `Ok(None)` means no reply before the timeout.
pub(crate) type Exchange<'a> = dyn FnMut(&[u8]) -> io::Result<Option<Vec<u8>>> + 'a;

/// Battery plus the active stage's DPI (DPI is optional: a failed read leaves it out).
fn run(mode: Path, exchange: &mut Exchange) -> Status {
    match battery_status(mode, exchange) {
        Status::Ok { mode, battery, charging, .. } => {
            let (dpi, stages) = match dpi::read_stages(exchange) {
                Some((d, st)) => (Some(d), st),
                None => (None, Stages::NONE),
            };
            Status::Ok { mode, battery, charging, dpi, stages }
        }
        other => other,
    }
}

/// Ask the receiver whether the mouse is awake; only then ask for the battery,
/// so a sleeping mouse is never contacted.
pub(crate) fn battery_status(mode: Path, exchange: &mut Exchange) -> Status {
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
            Ok(b) => Status::Ok { mode, battery: b.battery, charging: b.charging, dpi: None, stages: Stages::NONE },
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

#[derive(Debug, PartialEq)]
enum Cmd {
    Status,
    Dpi(u16),
}

fn parse_args(args: &[String]) -> Option<Cmd> {
    match args {
        [c] if c == "status" => Some(Cmd::Status),
        [c, v] if c == "dpi" => {
            let dpi: u16 = v.parse().ok()?;
            protocol::dpi_to_code(dpi).map(|_| Cmd::Dpi(dpi))
        }
        _ => None,
    }
}

/// Save the EEPROM image once: an existing backup is never overwritten.
fn save_backup(dir: &std::path::Path, image: &[u8]) -> io::Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(dir)?;
    let file = std::fs::OpenOptions::new().write(true).create_new(true).open(dir.join("backup.hex"));
    let mut file = match file {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Ok(()),
        Err(e) => return Err(e),
    };
    let hex: Vec<String> = image.iter().map(|b| format!("{b:02x}")).collect();
    writeln!(file, "{}", hex.join(" "))
}

fn state_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
        });
    base.join("vxe")
}

const TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);
/// Writes are echoed after ~80 ms; leave margin.
const WRITE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

fn execute(cmd: Cmd) -> Status {
    let Some((mode, node)) = device::find(std::path::Path::new("/sys/class/hidraw")) else {
        return Status::Error(ErrKind::NoReceiver);
    };
    let mut dev = match hidraw::Hidraw::open(&node) {
        Ok(d) => d,
        Err(e) => return open_error(&e),
    };
    let mut ex = |req: &[u8]| {
        let t = if req[1] == protocol::CMD_WRITE { WRITE_TIMEOUT } else { TIMEOUT };
        dev.exchange(req, t)
    };
    match cmd {
        Cmd::Status => run(mode, &mut ex),
        Cmd::Dpi(dpi) => {
            let dir = state_dir();
            dpi::set_dpi(mode, dpi, &mut ex, &mut |img| save_backup(&dir, img))
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = parse_args(&args) else {
        eprintln!("uso: vxe status | vxe dpi <50–10000, múltiplo de 50>");
        std::process::exit(2);
    };
    let s = execute(cmd);
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
        assert_eq!(s, Status::Ok { mode: Path::Wireless, battery: 30, charging: false, dpi: None, stages: Stages::NONE });
        assert_eq!(&sent[..2], &[0x03, 0x04]);
        assert!(sent[2..].iter().all(|&c| c == protocol::CMD_READ));
    }

    #[test]
    fn status_includes_dpi() {
        let mut m = dpi::fake::FakeMouse::new();
        let s = run(Path::Wireless, &mut |r| m.exchange(r));
        assert_eq!(s, Status::Ok { mode: Path::Wireless, battery: 30, charging: false, dpi: Some(1200), stages: Stages::from_slice(&[800, 1200, 1600, 3200]) });
        assert!(m.writes().is_empty());
    }

    #[test]
    fn run_ok_wired_charging() {
        let replies = [(0x03, Ok(Some(hex(ONLINE)))), (0x04, Ok(Some(hex(WIRED_50))))];
        let mut sent = vec![];
        let s = run(Path::Wired, &mut serve(&replies, &mut sent));
        assert_eq!(s, Status::Ok { mode: Path::Wired, battery: 50, charging: true, dpi: None, stages: Stages::NONE });
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

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn dpi_arg_validation() {
        assert_eq!(parse_args(&args(&["status"])), Some(Cmd::Status));
        assert_eq!(parse_args(&args(&["dpi", "1000"])), Some(Cmd::Dpi(1000)));
        assert_eq!(parse_args(&args(&["dpi", "10000"])), Some(Cmd::Dpi(10000)));
        for bad in [&["dpi", "825"][..], &["dpi", "0"], &["dpi", "abc"], &["dpi", "10050"], &["dpi"], &[], &["foo"], &["status", "x"]] {
            assert_eq!(parse_args(&args(bad)), None, "{bad:?}");
        }
    }

    #[test]
    fn backup_written_once() {
        let dir = std::env::temp_dir().join(format!("vxe-backup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        save_backup(&dir, &[0x01, 0x54, 0xab]).unwrap();
        save_backup(&dir, &[0xff]).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("backup.hex")).unwrap(), "01 54 ab\n");
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
