//! Locate the VXE R1 configuration hidraw node through sysfs.

use crate::protocol::Path;
use std::path::{Path as FsPath, PathBuf};

const RECEIVER_ID: &str = "0003:0000373B:000010C9";
const WIRED_ID: &str = "0003:0000373B:0000121F";
/// Vendor collection: usage page 0xFF02, usage 2, report id 0x08.
const CONFIG_COLLECTION: [u8; 9] = [0x06, 0x02, 0xff, 0x09, 0x02, 0xa1, 0x01, 0x85, 0x08];

fn path_for(uevent: &str) -> Option<Path> {
    let id = uevent.lines().find_map(|l| l.strip_prefix("HID_ID="))?;
    if id.eq_ignore_ascii_case(WIRED_ID) {
        Some(Path::Wired)
    } else if id.eq_ignore_ascii_case(RECEIVER_ID) {
        Some(Path::Wireless)
    } else {
        None
    }
}

fn has_config_collection(desc: &[u8]) -> bool {
    desc.windows(CONFIG_COLLECTION.len()).any(|w| w == CONFIG_COLLECTION)
}

/// Return the config interface of the wired mouse or the receiver
/// (wired wins if both are present) and its `/dev/hidrawN` path.
pub fn find(sysfs_hidraw: &FsPath) -> Option<(Path, PathBuf)> {
    let mut best: Option<(Path, PathBuf)> = None;
    for entry in std::fs::read_dir(sysfs_hidraw).ok()?.flatten() {
        let dev = entry.path().join("device");
        let Some(path) = std::fs::read_to_string(dev.join("uevent")).ok().as_deref().and_then(path_for) else {
            continue;
        };
        let Ok(desc) = std::fs::read(dev.join("report_descriptor")) else { continue };
        if !has_config_collection(&desc) {
            continue;
        }
        let node = PathBuf::from("/dev").join(entry.file_name());
        if best.as_ref().is_none_or(|(p, _)| *p == Path::Wireless && path == Path::Wired) {
            best = Some((path, node));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const CONFIG_DESC: &str = "0602ff0902a1018508150026ff00750895100902810009029100c0";
    const MOUSE_DESC: &str = "05010902a1010901a100050919012905c0c0";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    /// Build a fake /sys/class/hidraw tree in a fresh temp dir.
    fn tree(name: &str, nodes: &[(&str, &str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vxe-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (node, hid_id, desc) in nodes {
            let dev = root.join(node).join("device");
            fs::create_dir_all(&dev).unwrap();
            fs::write(dev.join("uevent"), format!("DRIVER=hid-generic\nHID_ID={hid_id}\nHID_NAME=x\n")).unwrap();
            fs::write(dev.join("report_descriptor"), unhex(desc)).unwrap();
        }
        root
    }

    #[test]
    fn find_none_when_no_match() {
        let root = tree("none", &[("hidraw0", "0003:00001CA2:00001901", CONFIG_DESC)]);
        assert_eq!(find(&root), None);
    }

    #[test]
    fn find_none_when_root_missing() {
        assert_eq!(find(FsPath::new("/nonexistent/vxe")), None);
    }

    #[test]
    fn find_receiver_interface() {
        let root = tree("rx", &[
            ("hidraw0", "0003:0000373B:000010C9", MOUSE_DESC),
            ("hidraw1", "0003:0000373B:000010C9", CONFIG_DESC),
            ("hidraw2", "0003:0000373B:000010C9", MOUSE_DESC),
        ]);
        assert_eq!(find(&root), Some((Path::Wireless, PathBuf::from("/dev/hidraw1"))));
    }

    #[test]
    fn find_wired_interface() {
        let root = tree("wired", &[
            ("hidraw4", "0003:0000373B:0000121F", MOUSE_DESC),
            ("hidraw5", "0003:0000373B:0000121F", CONFIG_DESC),
        ]);
        assert_eq!(find(&root), Some((Path::Wired, PathBuf::from("/dev/hidraw5"))));
    }

    #[test]
    fn find_prefers_wired_when_both_present() {
        let root = tree("both", &[
            ("hidraw1", "0003:0000373B:000010C9", CONFIG_DESC),
            ("hidraw7", "0003:0000373B:0000121F", CONFIG_DESC),
        ]);
        assert_eq!(find(&root), Some((Path::Wired, PathBuf::from("/dev/hidraw7"))));
    }

    #[test]
    fn find_case_insensitive_hid_id() {
        let root = tree("case", &[("hidraw3", "0003:0000373b:000010c9", CONFIG_DESC)]);
        assert_eq!(find(&root), Some((Path::Wireless, PathBuf::from("/dev/hidraw3"))));
    }

    #[test]
    fn find_ignores_device_without_config_collection() {
        let root = tree("noconf", &[("hidraw0", "0003:0000373B:000010C9", MOUSE_DESC)]);
        assert_eq!(find(&root), None);
    }
}
