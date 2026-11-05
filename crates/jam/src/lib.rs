//! Maschine JAM over USB (docs/jam-usb.md). So far, what the tools in `examples/` need to
//! learn the protocol: the device's ids and reports, opening it, and telling what changed
//! between two reports. The decoder comes when the map is known.

/// Native Instruments' USB vendor id.
pub const VENDOR: u16 = 0x17cc;
/// The Maschine JAM.
pub const PRODUCT: u16 = 0x1500;

/// Input reports: id, bytes after the id.
pub const INPUTS: [(u8, usize); 3] = [(0x01, 16), (0x02, 48), (0x40, 24)];
/// The LED reports, the only ones anything here writes: id, bytes after the id.
pub const LEDS: [(u8, usize); 3] = [(0x80, 53), (0x81, 80), (0x82, 88)];
/// Feature reports: id, bytes after the id. Read only.
pub const FEATURES: [(u8, usize); 9] =
    [(0xd0, 32), (0xd8, 32), (0xd9, 32), (0xda, 32), (0xdb, 32), (0xdc, 32), (0xdd, 32), (0xf1, 204), (0xf2, 204)];

/// The JAM's HID interface, opened shared.
pub fn open() -> Result<hidapi::HidDevice, String> {
    let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
    // Shared with NI's agent (macOS; elsewhere HID devices open shared).
    #[cfg(target_os = "macos")]
    api.set_open_exclusive(false);
    api.open(VENDOR, PRODUCT).map_err(|e| format!("no Maschine JAM: {e}"))
}

/// An LED report with every byte 0: everything off.
pub fn dark(id: u8, len: usize) -> Vec<u8> {
    let mut r = vec![0; len + 1];
    r[0] = id;
    r
}

/// Turn every LED off.
pub fn all_dark(dev: &hidapi::HidDevice) -> Result<(), String> {
    for (id, len) in LEDS {
        dev.write(&dark(id, len)).map_err(|e| format!("report {id:#04x}: {e}"))?;
    }
    Ok(())
}

/// A byte that changed between two reports of one id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    /// From 0, the report's id being byte 0.
    pub byte: usize,
    pub was: u8,
    pub now: u8,
}

/// The bytes that differ. A report that got shorter or longer counts its extra bytes as
/// changed from or to 0.
pub fn changes(was: &[u8], now: &[u8]) -> Vec<Change> {
    (0..was.len().max(now.len()))
        .filter_map(|byte| {
            let (w, n) = (was.get(byte).copied().unwrap_or(0), now.get(byte).copied().unwrap_or(0));
            (w != n).then_some(Change { byte, was: w, now: n })
        })
        .collect()
}

/// The bits that went from 0 to 1, as (byte, bit).
pub fn rising(changes: &[Change]) -> Vec<(usize, u8)> {
    changes.iter().flat_map(|c| (0..8u8).filter(move |b| c.now & !c.was & (1 << b) != 0).map(move |b| (c.byte, b))).collect()
}

/// The eight columns' names, as printed under the group buttons.
pub const COLUMNS: [char; 8] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];

/// Every control a press can report, as the panel names them, in an order that's easy to
/// follow by hand: top to bottom, left to right. Pads are `A1` (top left) to `H8`.
pub fn switches() -> Vec<String> {
    let mut out: Vec<String> = (1..=8).map(|n| format!("SCENE {n}")).collect();
    for row in 1..=8 {
        out.extend(COLUMNS.iter().map(|c| format!("{c}{row}")));
    }
    out.extend(COLUMNS.iter().map(|c| format!("GROUP {c}")));
    out.extend(
        [
            "SONG",
            "STEP",
            "PAD MODE",
            "CLEAR",
            "DUPLICATE",
            "NOTE REPEAT",
            "D-pad UP",
            "D-pad LEFT",
            "D-pad RIGHT",
            "D-pad DOWN",
            "MST",
            "GRP",
            "IN1",
            "CUE",
            "BROWSE",
            // Touch first: a push touches it too, and a bit already named doesn't count again.
            "encoder TOUCH (just rest a finger on it)",
            "encoder PUSH",
            "MACRO",
            "LEVEL",
            "AUX",
            "CONTROL",
            "AUTO",
            "PERFORM",
            "NOTES",
            "LOCK",
            "TUNE",
            "SWING",
            "SELECT",
            "SOLO",
            "MUTE",
            "SHIFT",
            "PLAY",
            "REC",
            "page LEFT",
            "page RIGHT",
            "TEMPO",
            "GRID",
        ]
        .map(String::from),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_bytes_that_differ_are_changes() {
        let c = changes(&[1, 0b0000_0001, 9], &[1, 0b0000_0011, 7]);
        assert_eq!(c, vec![Change { byte: 1, was: 1, now: 3 }, Change { byte: 2, was: 9, now: 7 }]);
    }

    #[test]
    fn a_report_of_another_length_counts_its_extra_bytes() {
        assert_eq!(changes(&[1], &[1, 5]), vec![Change { byte: 1, was: 0, now: 5 }]);
        assert_eq!(changes(&[1, 5], &[1]), vec![Change { byte: 1, was: 5, now: 0 }]);
    }

    #[test]
    fn rising_names_only_the_bits_that_came_on() {
        // Bit 0 stays on, bit 1 comes on, bit 7 goes off; in the next byte bits 2 and 3 come on.
        let c = changes(&[0b1000_0001, 0], &[0b0000_0011, 0b0000_1100]);
        assert_eq!(rising(&c), vec![(0, 1), (1, 2), (1, 3)]);
    }

    #[test]
    fn the_list_has_every_switch_the_design_counts() {
        let s = switches();
        // 8 scenes, 64 pads, 8 groups, 35 lit buttons, the encoder's push and touch. (The
        // footswitch needs a footswitch: jam-learn asks for it apart.)
        assert_eq!(s.len(), 8 + 64 + 8 + 35 + 2);
        assert_eq!(s.iter().collect::<std::collections::HashSet<_>>().len(), s.len(), "no name twice");
        assert_eq!((s[8].as_str(), s[8 + 63].as_str()), ("A1", "H8"));
    }

    #[test]
    fn a_dark_report_is_its_id_then_zeros() {
        assert_eq!(dark(0x81, 3), vec![0x81, 0, 0, 0]);
    }

    #[test]
    fn the_led_reports_have_the_sizes_the_controls_add_up_to() {
        assert_eq!(LEDS[1].1, 64 + 8 + 8, "pads, scenes, groups");
        assert_eq!(LEDS[2].1, 8 * 11, "strips");
    }
}
