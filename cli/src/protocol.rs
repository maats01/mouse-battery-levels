//! COMPX v1 frames (see docs/protocol.md). Pure: no I/O.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Path {
    Wireless,
    Wired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub battery: u8,
    pub charging: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError(pub String);

const REPORT_ID: u8 = 0x08;
const FRAME_LEN: usize = 17;
const CHECKSUM_TOTAL: u8 = 0x55;
pub const CMD_ONLINE: u8 = 0x03;
pub const CMD_BATTERY: u8 = 0x04;

fn frame(cmd: u8) -> Vec<u8> {
    let mut f = vec![0u8; FRAME_LEN];
    f[0] = REPORT_ID;
    f[1] = cmd;
    let sum = f[..16].iter().fold(0u8, |a, &b| a.wrapping_add(b));
    f[16] = CHECKSUM_TOTAL.wrapping_sub(sum);
    f
}

/// Validate a reply to `cmd` and return its data bytes (after the length byte).
fn reply_data(cmd: u8, resp: &[u8]) -> Result<&[u8], ParseError> {
    if resp.len() < FRAME_LEN {
        return Err(ParseError(format!("short reply ({} bytes)", resp.len())));
    }
    let f = &resp[..FRAME_LEN];
    if f[0] != REPORT_ID {
        return Err(ParseError(format!("report id {:#04x}", f[0])));
    }
    if f.iter().fold(0u8, |a, &b| a.wrapping_add(b)) != CHECKSUM_TOTAL {
        return Err(ParseError("bad checksum".into()));
    }
    if f[1] != cmd {
        return Err(ParseError(format!("reply to command {:#04x}", f[1])));
    }
    if f[2] != 0 {
        return Err(ParseError(format!("status {:#04x}", f[2])));
    }
    Ok(&f[6..16])
}

/// True when `resp` is a well-formed reply to `cmd` (used to skip unrelated reports).
pub fn is_reply_to(cmd: u8, resp: &[u8]) -> bool {
    resp.len() >= 2 && resp[0] == REPORT_ID && resp[1] == cmd
}

pub fn online_request() -> Vec<u8> {
    frame(CMD_ONLINE)
}

pub fn battery_request() -> Vec<u8> {
    frame(CMD_BATTERY)
}

pub fn parse_online(resp: &[u8]) -> Result<bool, ParseError> {
    Ok(reply_data(CMD_ONLINE, resp)?[0] == 1)
}

pub fn parse_battery(resp: &[u8]) -> Result<Reading, ParseError> {
    let d = reply_data(CMD_BATTERY, resp)?;
    Ok(Reading { battery: d[0].min(100), charging: d[1] != 0 })
}

pub const CMD_WRITE: u8 = 0x07;
pub const CMD_READ: u8 = 0x08;
pub const ACTIVE_STAGE_ADDR: u16 = 0x0004;
pub const STAGE_BASE: u16 = 0x000c;

fn frame_with(cmd: u8, addr: u16, len: u8, data: &[u8]) -> Vec<u8> {
    let mut f = vec![0u8; FRAME_LEN];
    f[0] = REPORT_ID;
    f[1] = cmd;
    f[3] = (addr >> 8) as u8;
    f[4] = addr as u8;
    f[5] = len;
    f[6..6 + data.len()].copy_from_slice(data);
    let sum = f[..16].iter().fold(0u8, |a, &b| a.wrapping_add(b));
    f[16] = CHECKSUM_TOTAL.wrapping_sub(sum);
    f
}

/// EEPROM read of `len` (1–10) bytes at `addr`.
pub fn read_request(addr: u16, len: u8) -> Vec<u8> {
    assert!((1..=10).contains(&len));
    frame_with(CMD_READ, addr, len, &[])
}

/// EEPROM write of 1–10 bytes at `addr`.
pub fn write_request(addr: u16, data: &[u8]) -> Vec<u8> {
    assert!((1..=10).contains(&data.len()));
    frame_with(CMD_WRITE, addr, data.len() as u8, data)
}

pub fn parse_read(addr: u16, len: u8, resp: &[u8]) -> Result<Vec<u8>, ParseError> {
    let d = reply_data(CMD_READ, resp)?;
    let got = u16::from_be_bytes([resp[3], resp[4]]);
    if got != addr {
        return Err(ParseError(format!("reply for address {got:#06x}")));
    }
    if resp[5] < len {
        return Err(ParseError(format!("short read ({} bytes)", resp[5])));
    }
    Ok(d[..len as usize].to_vec())
}

/// The mouse acknowledges a write by echoing the request unchanged.
pub fn is_write_echo(req: &[u8], resp: &[u8]) -> bool {
    resp.len() >= FRAME_LEN && resp[..FRAME_LEN] == req[..FRAME_LEN]
}

/// Whether `resp` answers `req`: writes by exact echo, reads by command and address.
pub fn reply_matches(req: &[u8], resp: &[u8]) -> bool {
    match req[1] {
        CMD_WRITE => is_write_echo(req, resp),
        CMD_READ => is_reply_to(CMD_READ, resp) && resp.len() >= 5 && resp[3..5] == req[3..5],
        cmd => is_reply_to(cmd, resp),
    }
}

/// PAW3395SE stage codes: 1..=235, skipping these (docs/protocol.md).
const SKIPPED_CODES: [u8; 35] = [
    7, 13, 20, 26, 33, 40, 46, 53, 60, 66, 73, 80, 86, 93, 100, 106, 113, 120, 126, 133, 140,
    146, 153, 160, 166, 173, 180, 186, 193, 200, 206, 213, 220, 226, 233,
];
pub const DPI_MAX: u16 = 10_000;

fn codes() -> impl Iterator<Item = u8> {
    (1..=235u8).filter(|c| !SKIPPED_CODES.contains(c))
}

pub fn dpi_to_code(dpi: u16) -> Option<u8> {
    if dpi == 0 || dpi % 50 != 0 || dpi > DPI_MAX {
        return None;
    }
    codes().nth(dpi as usize / 50 - 1)
}

pub fn code_to_dpi(code: u8) -> Option<u16> {
    codes().position(|c| c == code).map(|i| (i as u16 + 1) * 50)
}

/// Value/complement pair: valid when `v + c == 0x55`.
pub fn decode_pair(v: u8, c: u8) -> Option<u8> {
    (v.wrapping_add(c) == CHECKSUM_TOTAL).then_some(v)
}

pub fn encode_stage(dpi: u16) -> Option<[u8; 4]> {
    let c = dpi_to_code(dpi)?;
    Some([c, c, 0, CHECKSUM_TOTAL.wrapping_sub(c.wrapping_add(c))])
}

pub fn decode_stage(rec: &[u8]) -> Option<u16> {
    let [x, y, ex, ck] = *rec.get(..4)? else { return None };
    let sum = x.wrapping_add(y).wrapping_add(ex);
    if x != y || ex != 0 || CHECKSUM_TOTAL.wrapping_sub(sum) != ck {
        return None;
    }
    code_to_dpi(x)
}

#[cfg(test)]
pub(crate) fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIRELESS_30: &str = include_str!("../tests/fixtures/wireless_30.hex");
    const WIRED_50: &str = include_str!("../tests/fixtures/wired_charging_50.hex");
    const ONLINE: &str = include_str!("../tests/fixtures/online_yes.hex");

    /// Recompute the checksum after editing a frame in a test.
    fn fix_checksum(f: &mut [u8]) {
        f[16] = 0;
        let s: u32 = f[..16].iter().map(|&b| b as u32).sum();
        f[16] = (0x55u32.wrapping_sub(s) & 0xff) as u8;
    }

    #[test]
    fn battery_request_bytes() {
        assert_eq!(
            battery_request(),
            hex("08 04 00 00 00 00 00 00 00 00 00 00 00 00 00 00 49")
        );
    }

    #[test]
    fn online_request_bytes() {
        assert_eq!(
            online_request(),
            hex("08 03 00 00 00 00 00 00 00 00 00 00 00 00 00 00 4a")
        );
    }

    #[test]
    fn parse_wireless_fixture() {
        assert_eq!(
            parse_battery(&hex(WIRELESS_30)).unwrap(),
            Reading { battery: 30, charging: false }
        );
    }

    #[test]
    fn parse_wired_charging() {
        assert_eq!(
            parse_battery(&hex(WIRED_50)).unwrap(),
            Reading { battery: 50, charging: true }
        );
    }

    #[test]
    fn parse_online_fixture() {
        assert_eq!(parse_online(&hex(ONLINE)), Ok(true));
    }

    #[test]
    fn parse_online_offline() {
        let mut f = hex(ONLINE);
        f[6] = 0;
        fix_checksum(&mut f);
        assert_eq!(parse_online(&f), Ok(false));
    }

    #[test]
    fn parse_rejects_wrong_report_id() {
        assert!(parse_battery(&[0x00; 17]).is_err());
    }

    #[test]
    fn parse_rejects_short() {
        assert!(parse_battery(&[0x08]).is_err());
    }

    #[test]
    fn parse_rejects_bad_checksum() {
        let mut f = hex(WIRELESS_30);
        f[16] ^= 1;
        assert!(parse_battery(&f).is_err());
    }

    #[test]
    fn parse_rejects_other_command() {
        assert!(parse_battery(&hex(ONLINE)).is_err());
    }

    #[test]
    fn parse_rejects_error_status() {
        let mut f = hex(WIRELESS_30);
        f[2] = 1;
        fix_checksum(&mut f);
        assert!(parse_battery(&f).is_err());
    }

    const EE_0000: &str = include_str!("../tests/fixtures/eeprom_0000.hex");
    const EE_000C: &str = include_str!("../tests/fixtures/eeprom_000c.hex");

    fn sums_to_55(f: &[u8]) -> bool {
        f.len() == 17 && f.iter().fold(0u8, |a, &b| a.wrapping_add(b)) == 0x55
    }

    #[test]
    fn read_request_bytes() {
        let r = read_request(0x000c, 10);
        assert_eq!(&r[..7], &[0x08, 0x08, 0x00, 0x00, 0x0c, 0x0a, 0x00]);
        assert!(sums_to_55(&r));
    }

    #[test]
    fn write_request_bytes() {
        let r = write_request(0x0010, &[0x1c, 0x1c, 0x00, 0x1d]);
        assert_eq!(&r[..10], &[0x08, 0x07, 0x00, 0x00, 0x10, 0x04, 0x1c, 0x1c, 0x00, 0x1d]);
        assert!(sums_to_55(&r));
    }

    #[test]
    fn parse_read_fixture_000c() {
        assert_eq!(
            parse_read(0x000c, 10, &hex(EE_000C)).unwrap(),
            hex("12 12 00 31 1c 1c 00 1d 25 25")
        );
    }

    #[test]
    fn parse_read_rejects_other_addr() {
        assert!(parse_read(0x0000, 10, &hex(EE_000C)).is_err());
    }

    #[test]
    fn write_echo_matches_request_only() {
        let req = write_request(0x0010, &[0x1c, 0x1c, 0x00, 0x1d]);
        assert!(is_write_echo(&req, &req));
        let other = write_request(0x0010, &[0x12, 0x12, 0x00, 0x31]);
        assert!(!is_write_echo(&req, &other));
        assert!(!is_write_echo(&req, &req[..5]));
    }

    #[test]
    fn reply_matches_by_kind() {
        let rd = read_request(0x000c, 10);
        assert!(reply_matches(&rd, &hex(EE_000C)));
        assert!(!reply_matches(&read_request(0x0000, 10), &hex(EE_000C)));
        let wr = write_request(0x0010, &[0x1c, 0x1c, 0x00, 0x1d]);
        assert!(reply_matches(&wr, &wr));
        assert!(!reply_matches(&wr, &write_request(0x0014, &[0x1c, 0x1c, 0x00, 0x1d])));
        assert!(reply_matches(&battery_request(), &hex(WIRELESS_30)));
        assert!(!reply_matches(&battery_request(), &hex(ONLINE)));
    }

    #[test]
    fn dpi_table_known_codes() {
        for (dpi, code) in [(50, 1), (400, 0x09), (800, 0x12), (1200, 0x1c), (1600, 0x25), (3200, 0x4b), (10000, 235)] {
            assert_eq!(dpi_to_code(dpi), Some(code), "dpi {dpi}");
            assert_eq!(code_to_dpi(code), Some(dpi), "code {code}");
        }
    }

    #[test]
    fn dpi_table_rejects() {
        for d in [0, 25, 825, 10050] {
            assert_eq!(dpi_to_code(d), None, "dpi {d}");
        }
        for c in [0, 7, 13, 233, 236] {
            assert_eq!(code_to_dpi(c), None, "code {c}");
        }
    }

    #[test]
    fn encode_stage_1200() {
        assert_eq!(encode_stage(1200), Some([0x1c, 0x1c, 0x00, 0x1d]));
        assert_eq!(encode_stage(825), None);
    }

    #[test]
    fn decode_stage_fixture() {
        let d = parse_read(0x000c, 10, &hex(EE_000C)).unwrap();
        assert_eq!(decode_stage(&d[0..4]), Some(800));
        assert_eq!(decode_stage(&d[4..8]), Some(1200));
    }

    #[test]
    fn decode_stage_rejects_bad_check() {
        assert_eq!(decode_stage(&[0x1c, 0x1c, 0x00, 0x1e]), None);
        assert_eq!(decode_stage(&[0x1c, 0x12, 0x00, 0x27]), None); // x != y
        assert_eq!(decode_stage(&[0x1c]), None);
    }

    #[test]
    fn decode_pair_active_stage() {
        let d = parse_read(0x0000, 10, &hex(EE_0000)).unwrap();
        assert_eq!(decode_pair(d[4], d[5]), Some(1));
        assert_eq!(decode_pair(1, 0x50), None);
    }

    #[test]
    fn parse_clamps_battery() {
        let mut f = hex(WIRELESS_30);
        f[6] = 0xff;
        fix_checksum(&mut f);
        assert_eq!(parse_battery(&f).unwrap().battery, 100);
    }
}
