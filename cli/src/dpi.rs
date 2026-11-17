//! Read the active stage's DPI and rewrite it (docs/protocol.md, "DPI").

use crate::Exchange;
use crate::protocol::Path;
use crate::status::{Stages, Status};
use std::io;

/// Bytes saved before the first write: header pairs + 8 stage records.
pub const BACKUP_LEN: u16 = 0x2c;

use crate::protocol::{
    ACTIVE_STAGE_ADDR, STAGE_BASE, decode_pair, decode_stage, encode_stage, is_write_echo,
    parse_read, read_request, write_request,
};
use crate::status::ErrKind;

enum Fail {
    Timeout,
    Io,
}

fn read(ex: &mut Exchange, addr: u16, len: u8) -> Result<Vec<u8>, Fail> {
    match ex(&read_request(addr, len)) {
        Ok(Some(r)) => parse_read(addr, len, &r).map_err(|_| Fail::Io),
        Ok(None) => Err(Fail::Timeout),
        Err(_) => Err(Fail::Io),
    }
}

fn active_stage(pair: &[u8]) -> Option<u16> {
    decode_pair(pair[0], pair[1]).filter(|a| *a < 8).map(u16::from)
}

fn stage_addr(active: u16) -> u16 {
    STAGE_BASE + 4 * active
}

const STAGE_COUNT_ADDR: u16 = 0x0002;

/// Decode `count` stage records from consecutive bytes.
fn decode_stages(bytes: &[u8], count: usize) -> Option<Stages> {
    let v: Option<Vec<u16>> = bytes.chunks(4).take(count).map(decode_stage).collect();
    v.filter(|v| v.len() == count).map(|v| Stages::from_slice(&v))
}

/// Count and active index from the header pairs at 0x0002..0x0006.
fn header(pairs: &[u8]) -> Option<(usize, usize)> {
    let count = decode_pair(pairs[0], pairs[1]).filter(|c| (1..=8).contains(c))? as usize;
    let active = decode_pair(pairs[2], pairs[3]).filter(|a| (*a as usize) < count)? as usize;
    Some((count, active))
}

/// Active stage's DPI and all stages, or `None` if anything is missing or invalid.
pub fn read_stages(ex: &mut Exchange) -> Option<(u16, Stages)> {
    let (count, active) = header(&read(ex, STAGE_COUNT_ADDR, 4).ok()?)?;
    let total = (count * 4) as u16;
    let mut bytes = Vec::with_capacity(total as usize);
    for off in (0..total).step_by(8) {
        bytes.extend(read(ex, STAGE_BASE + off, (total - off).min(8) as u8).ok()?);
    }
    let stages = decode_stages(&bytes, count)?;
    Some((stages.as_slice()[active], stages))
}

/// Rewrite the active stage with `dpi` (already validated) and return the new status.
/// `backup` receives the EEPROM image `0x0000..BACKUP_LEN` before anything is written.
pub fn set_dpi(
    mode: Path,
    dpi: u16,
    ex: &mut Exchange,
    backup: &mut dyn FnMut(&[u8]) -> io::Result<()>,
) -> Status {
    let (battery, charging) = match crate::battery_status(mode, ex) {
        Status::Ok { battery, charging, .. } => (battery, charging),
        other => return other,
    };
    let fail = |f: Fail| match f {
        Fail::Timeout => Status::Disconnected,
        Fail::Io => Status::Error(ErrKind::Io),
    };

    let mut image = Vec::with_capacity(BACKUP_LEN as usize);
    for start in (0..BACKUP_LEN).step_by(10) {
        let len = (BACKUP_LEN - start).min(10) as u8;
        match read(ex, start, len) {
            Ok(chunk) => image.extend(chunk),
            Err(f) => return fail(f),
        }
    }
    let Some(active) = active_stage(&image[ACTIVE_STAGE_ADDR as usize..]) else {
        return Status::Error(ErrKind::Io);
    };
    let Some(record) = encode_stage(dpi) else {
        return Status::Error(ErrKind::Io);
    };
    if backup(&image).is_err() {
        return Status::Error(ErrKind::Io);
    }

    let addr = stage_addr(active);
    let req = write_request(addr, &record);
    match ex(&req) {
        Ok(Some(r)) if is_write_echo(&req, &r) => {}
        Ok(_) => return Status::Error(ErrKind::WriteFailed),
        Err(_) => return Status::Error(ErrKind::Io),
    }
    match read(ex, addr, 4) {
        Ok(back) if back == record => {
            let stages = header(&image[STAGE_COUNT_ADDR as usize..])
                .and_then(|(count, _)| {
                    image[STAGE_BASE as usize + 4 * active as usize..][..4].copy_from_slice(&record);
                    decode_stages(&image[STAGE_BASE as usize..], count)
                })
                .unwrap_or(Stages::NONE);
            Status::Ok { mode, battery, charging, dpi: Some(dpi), stages }
        }
        _ => Status::Error(ErrKind::WriteFailed),
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use crate::protocol::{self, hex};
    use std::io;

    const WIRELESS_30: &str = include_str!("../tests/fixtures/wireless_30.hex");
    const EE_0000: &str = include_str!("../tests/fixtures/eeprom_0000.hex");
    const EE_000C: &str = include_str!("../tests/fixtures/eeprom_000c.hex");

    /// A simulated mouse behind the receiver, with the user's real EEPROM header
    /// and stages (800/1200/1600/3200, active stage 1).
    pub struct FakeMouse {
        pub eeprom: Vec<u8>,
        pub awake: bool,
        pub echo_writes: bool,
        pub apply_writes: bool,
        pub sent: Vec<Vec<u8>>,
    }

    impl FakeMouse {
        pub fn new() -> Self {
            let mut eeprom = vec![0u8; 0xc0];
            eeprom[0..10].copy_from_slice(&hex(EE_0000)[6..16]);
            eeprom[0x0c..0x16].copy_from_slice(&hex(EE_000C)[6..16]);
            eeprom[0x14..0x1c].copy_from_slice(&hex("25 25 00 0b 4b 4b 00 bf"));
            Self { eeprom, awake: true, echo_writes: true, apply_writes: true, sent: vec![] }
        }

        fn reply(cmd: u8, addr: u16, data: &[u8]) -> Vec<u8> {
            let mut f = vec![0u8; 17];
            f[0] = 0x08;
            f[1] = cmd;
            f[3] = (addr >> 8) as u8;
            f[4] = addr as u8;
            f[5] = data.len() as u8;
            f[6..6 + data.len()].copy_from_slice(data);
            let sum = f[..16].iter().fold(0u8, |a, &b| a.wrapping_add(b));
            f[16] = 0x55u8.wrapping_sub(sum);
            f
        }

        pub fn exchange(&mut self, req: &[u8]) -> io::Result<Option<Vec<u8>>> {
            self.sent.push(req.to_vec());
            let addr = u16::from_be_bytes([req[3], req[4]]);
            let len = req[5] as usize;
            Ok(match req[1] {
                0x03 => Some(Self::reply(0x03, 0, &[self.awake as u8])),
                _ if !self.awake => None,
                0x04 => Some(hex(WIRELESS_30)),
                protocol::CMD_READ => {
                    let a = addr as usize;
                    Some(Self::reply(protocol::CMD_READ, addr, &self.eeprom[a..a + len]))
                }
                protocol::CMD_WRITE => {
                    if self.apply_writes {
                        let a = addr as usize;
                        self.eeprom[a..a + len].copy_from_slice(&req[6..6 + len]);
                    }
                    self.echo_writes.then(|| req.to_vec())
                }
                _ => None,
            })
        }

        pub fn writes(&self) -> Vec<&Vec<u8>> {
            self.sent.iter().filter(|f| f[1] == protocol::CMD_WRITE).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeMouse;
    use super::*;
    use crate::protocol::write_request;
    use crate::status::ErrKind;

    fn no_backup() -> impl FnMut(&[u8]) -> io::Result<()> {
        |_| Ok(())
    }

    #[test]
    fn read_dpi_active_stage() {
        let mut m = FakeMouse::new();
        assert_eq!(read_stages(&mut |r| m.exchange(r)).map(|(d, _)| d), Some(1200));
    }

    #[test]
    fn read_stages_all() {
        let mut m = FakeMouse::new();
        assert_eq!(read_stages(&mut |r| m.exchange(r)), Some((1200, Stages::from_slice(&[800, 1200, 1600, 3200]))));
    }

    #[test]
    fn read_stages_bad_count_is_none() {
        let mut m = FakeMouse::new();
        m.eeprom[2] = 9;
        m.eeprom[3] = 0x4c;
        assert_eq!(read_stages(&mut |r| m.exchange(r)), None);
    }

    #[test]
    fn read_dpi_bad_record_is_none() {
        let mut m = FakeMouse::new();
        m.eeprom[0x13] ^= 1; // break stage 1 checksum
        assert_eq!(read_stages(&mut |r| m.exchange(r)), None);
    }

    #[test]
    fn dpi_writes_only_active_record() {
        let mut m = FakeMouse::new();
        let s = set_dpi(Path::Wireless, 1000, &mut |r| m.exchange(r), &mut no_backup());
        assert_eq!(s, Status::Ok { mode: Path::Wireless, battery: 30, charging: false, dpi: Some(1000), stages: Stages::from_slice(&[800, 1000, 1600, 3200]) });
        let rec = crate::protocol::encode_stage(1000).unwrap();
        assert_eq!(m.writes(), vec![&write_request(0x0010, &rec)]);
        assert_eq!(&m.eeprom[0x10..0x14], &rec);
        assert_eq!(&m.eeprom[0x0c..0x10], &[0x12, 0x12, 0x00, 0x31]); // stage 0 untouched
    }

    #[test]
    fn dpi_sleeping_writes_nothing() {
        let mut m = FakeMouse::new();
        m.awake = false;
        let s = set_dpi(Path::Wireless, 1000, &mut |r| m.exchange(r), &mut no_backup());
        assert_eq!(s, Status::Disconnected);
        assert_eq!(m.sent.len(), 1);
        assert_eq!(m.sent[0][1], 0x03);
    }

    #[test]
    fn dpi_backup_before_write() {
        // The written stage (0x10..0x14) lies inside the backup range, so an image
        // equal to the original proves the backup was taken before the write.
        let mut m = FakeMouse::new();
        let original = m.eeprom[..BACKUP_LEN as usize].to_vec();
        let mut saved: Option<Vec<u8>> = None;
        let mut backup = |img: &[u8]| {
            saved = Some(img.to_vec());
            Ok(())
        };
        set_dpi(Path::Wireless, 1000, &mut |r| m.exchange(r), &mut backup);
        assert_eq!(saved.as_deref(), Some(&original[..]));
        assert_ne!(&m.eeprom[0x10..0x14], &original[0x10..0x14]);
    }

    #[test]
    fn dpi_backup_failure_writes_nothing() {
        let mut m = FakeMouse::new();
        let mut fail = |_: &[u8]| Err(io::Error::from(io::ErrorKind::PermissionDenied));
        let s = set_dpi(Path::Wireless, 1000, &mut |r| m.exchange(r), &mut fail);
        assert_eq!(s, Status::Error(ErrKind::Io));
        assert!(m.writes().is_empty());
    }

    #[test]
    fn dpi_echo_timeout_is_write_failed() {
        let mut m = FakeMouse::new();
        m.echo_writes = false;
        let s = set_dpi(Path::Wireless, 1000, &mut |r| m.exchange(r), &mut no_backup());
        assert_eq!(s, Status::Error(ErrKind::WriteFailed));
    }

    #[test]
    fn dpi_readback_mismatch_is_write_failed() {
        let mut m = FakeMouse::new();
        m.apply_writes = false; // echoes, but the EEPROM keeps 1200
        let s = set_dpi(Path::Wireless, 1000, &mut |r| m.exchange(r), &mut no_backup());
        assert_eq!(s, Status::Error(ErrKind::WriteFailed));
    }

    #[test]
    fn dpi_bad_active_stage_is_io() {
        for (v, c) in [(9u8, 0x4cu8), (1, 0x50)] {
            let mut m = FakeMouse::new();
            m.eeprom[4] = v;
            m.eeprom[5] = c;
            let s = set_dpi(Path::Wireless, 1000, &mut |r| m.exchange(r), &mut no_backup());
            assert_eq!(s, Status::Error(ErrKind::Io), "pair {v:#x}/{c:#x}");
            assert!(m.writes().is_empty());
        }
    }
}
