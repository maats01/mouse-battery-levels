//! Result of one `vxe status` run and its one-line JSON form.

use crate::protocol::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrKind {
    NoReceiver,
    Permission,
    Io,
    WriteFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok { mode: Path, battery: u8, charging: bool, dpi: Option<u16> },
    Disconnected,
    Error(ErrKind),
}

impl Status {
    pub fn to_json(&self) -> String {
        match self {
            Status::Ok { mode, battery, charging, dpi } => {
                let mode = match mode {
                    Path::Wireless => "wireless",
                    Path::Wired => "wired",
                };
                let dpi = dpi.map(|d| format!(r#","dpi":{d}"#)).unwrap_or_default();
                format!(
                    r#"{{"connected":true,"mode":"{mode}","battery":{battery},"charging":{charging}{dpi}}}"#
                )
            }
            Status::Disconnected => r#"{"connected":false}"#.to_string(),
            Status::Error(kind) => {
                let e = match kind {
                    ErrKind::NoReceiver => "no_receiver",
                    ErrKind::Permission => "permission",
                    ErrKind::Io => "io",
                    ErrKind::WriteFailed => "write_failed",
                };
                format!(r#"{{"error":"{e}"}}"#)
            }
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Status::Error(_) => 1,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_ok() {
        let s = Status::Ok { mode: Path::Wireless, battery: 82, charging: false, dpi: None };
        assert_eq!(
            s.to_json(),
            r#"{"connected":true,"mode":"wireless","battery":82,"charging":false}"#
        );
    }

    #[test]
    fn json_ok_wired_charging() {
        let s = Status::Ok { mode: Path::Wired, battery: 50, charging: true, dpi: None };
        assert_eq!(
            s.to_json(),
            r#"{"connected":true,"mode":"wired","battery":50,"charging":true}"#
        );
    }

    #[test]
    fn json_ok_with_dpi() {
        let s = Status::Ok { mode: Path::Wireless, battery: 35, charging: false, dpi: Some(1200) };
        assert_eq!(
            s.to_json(),
            r#"{"connected":true,"mode":"wireless","battery":35,"charging":false,"dpi":1200}"#
        );
    }

    #[test]
    fn json_write_failed() {
        assert_eq!(Status::Error(ErrKind::WriteFailed).to_json(), r#"{"error":"write_failed"}"#);
        assert_eq!(Status::Error(ErrKind::WriteFailed).exit_code(), 1);
    }

    #[test]
    fn json_disconnected() {
        assert_eq!(Status::Disconnected.to_json(), r#"{"connected":false}"#);
    }

    #[test]
    fn json_errors() {
        assert_eq!(Status::Error(ErrKind::NoReceiver).to_json(), r#"{"error":"no_receiver"}"#);
        assert_eq!(Status::Error(ErrKind::Permission).to_json(), r#"{"error":"permission"}"#);
        assert_eq!(Status::Error(ErrKind::Io).to_json(), r#"{"error":"io"}"#);
    }

    #[test]
    fn exit_codes() {
        let ok = Status::Ok { mode: Path::Wired, battery: 1, charging: true, dpi: None };
        assert_eq!(ok.exit_code(), 0);
        assert_eq!(Status::Disconnected.exit_code(), 0);
        assert_eq!(Status::Error(ErrKind::Io).exit_code(), 1);
        assert_eq!(Status::Error(ErrKind::Permission).exit_code(), 1);
    }
}
