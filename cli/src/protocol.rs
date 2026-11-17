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

    #[test]
    fn parse_clamps_battery() {
        let mut f = hex(WIRELESS_30);
        f[6] = 0xff;
        fix_checksum(&mut f);
        assert_eq!(parse_battery(&f).unwrap().battery, 100);
    }
}
