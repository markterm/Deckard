# Maschine JAM over USB

Learned from one unit on Windows, with the tools in `crates/jam/examples/`. The design is
`docs/superpowers/specs/2026-11-05-maschine-jam-design.md`.

**Status, 2026-11-05.** Everything marked *measured* was read from the device. The map from
bits and bytes to named controls isn't known yet, because learning it takes someone pressing
each control and looking at each light.

## Device

VID 0x17CC, PID 0x1500. Interfaces: **0 HID** (vendor page 0xFF01; Windows' own `HidUsb`
driver), 1 DFU (NI's `nimjcusb`: firmware update, never touched). There's no audio, MIDI or
bulk interface, and no screens. The HID interface opens shared next to NI's agent.

## Input

Reports come only when something changes: 20 seconds untouched brought none (measured).
Asking for one (`HidD_GetInputReport`) works with NI's agent running.

### Report 0x01 (16 bytes): buttons

The descriptor cuts it into two 4-bit fields, then **120 one-bit fields** (measured). All
zeros at rest. The manual's controls come to 119 switches: 64 pads, 8 scene and 8 group
buttons, 35 others, the encoder's push and touch, and the footswitch's tip and ring. So
every switch should have its own bit, with one spare. The 4-bit fields are probably the
encoder's position, as the low nibble of the MK3's byte 11 is.

### Report 0x02 (48 bytes): Smart Strips

24 u16 values (measured), which is 8 strips of three: 6 bytes a strip. That's the MK3 strip's
record: a timestamp, then two fingers. At rest the first value of most strips is nonzero and
differs from strip to strip, and the other two are 0, which is the MK3's "not touched" (measured).
Which is which, the byte order and the range are still to confirm by touching them.

### Report 0x40 (24 bytes)

12 u16 values, all zero at rest (measured). Unknown. Twelve values don't fit 64 pads, and
the pads have only their bits in 0x01, so the click-pads very likely report on and off with no
pressure.

## Output

| Report | Bytes | Fields (measured) | Inferred |
|---|---|---|---|
| 0x80 | 53 | 37, then 16 | Plain LEDs, then the two 8-LED meters. The manual gives 35 lit buttons; two more are to find. |
| 0x81 | 80 | 80 | 64 pads, 8 scene, 8 group |
| 0x82 | 88 | 88 | 8 strips × 11 LEDs |
| 0xF4 | 3 | 3 | Unknown. Not written. |

The device accepted frames on 0x80, 0x81 and 0x82 with NI's agent running (measured). Nobody
was watching, so what they light is still to learn.

## The palette (feature reports 0xF1, 0xF2)

Each is 204 bytes: **17 colours × 4 shades × RGB** (measured), running red round to fuchsia,
then white. That's the MK3's scheme, so a colour LED very likely takes the same byte:
`(colour + 1) × 4 + shade`, colour 0 red … 15 fuchsia, 16 white; shade 0 dim, 1 dim flash, 2
bright, 3 flash (`crates/mk3/src/leds.rs`). In 0xF1 the fourth shade of every colour is a paler
version of the third, which suggests a flash; in 0xF2 the third and fourth are equal. Which
report drives which LEDs isn't known. Read only: nothing here writes them.

## Other feature reports

0xD0 reads all 0xFF. 0xD8 and 0xDA–0xDD hold data that looks specific to the unit, and 0xD9 an
ASCII number (a serial?). Their values aren't recorded here. Not written.

## Learning the rest

Close Maschine first. The prompts are spoken (`JAM_SILENT=1` turns that off) and there's no
time limit.

```
cargo run -p deckard-jam --example jam-learn        # every control: writes jam-map.txt
cargo run -p deckard-jam --example jam-led-learn 80 # plain LEDs and meters: jam-leds.txt
cargo run -p deckard-jam --example jam-led-learn 81 # pads, scenes, groups
cargo run -p deckard-jam --example jam-led-learn 82 # strips: two strips' worth shows the pattern
cargo run -p deckard-jam --example jam-led-colors   # the palette and brightness: jam-colors.txt
```

`jam-probe` watches the input reports and writes nothing, so it's safe next to Maschine;
`jam-hid-reports` lists the descriptor. Only 0x80, 0x81 and 0x82 are ever written.
