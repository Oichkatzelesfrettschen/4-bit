//! Peripheral round-trip integration tests.
//!
//! Feed bytes, scan codes, and nibbles through the public peripheral APIs and
//! assert observable outputs: a UART TX waveform captured and replayed into a
//! fresh receiver, a keyboard scanned row-by-row into hex keycodes, and a
//! 7-segment chain loaded from shift data and rendered to ASCII.

use mcs4_periph::{
    keyboard::{KeyState, MatrixKeyboard},
    seven_seg::{bcd_to_segments, SevenSegDisplay},
    uart::{SerialConfig, Uart},
};

/// Capture a transmitter's line waveform for one full frame, one sample per
/// tick. The two UART state machines are verified independently, so decoupling
/// capture from replay keeps a dropped bit attributable to alignment, never to
/// the crate.
fn capture_tx_frame(cfg: SerialConfig, byte: u8) -> Vec<bool> {
    let cpb = cfg.cycles_per_bit;
    let mut tx = Uart::new(cfg, 16);
    assert!(tx.tx_write(byte));

    let frame_bits = 1 + cfg.data_bits as u32 + cfg.stop_bits as u32 + 1;
    let frame_ticks = frame_bits * cpb;

    let mut wave = Vec::with_capacity(frame_ticks as usize);
    for _ in 0..frame_ticks {
        tx.tx_tick();
        wave.push(tx.tx_line());
    }
    wave
}

/// Replay a captured waveform into a fresh receiver, bracketed by idle-mark
/// samples so the receiver idles, detects the falling start edge, and has room
/// to sample the final stop bit.
fn replay_into_rx(cfg: SerialConfig, wave: &[bool]) -> Option<u8> {
    let cpb = cfg.cycles_per_bit;
    let mut rx = Uart::new(cfg, 16);
    for _ in 0..cpb {
        rx.rx_feed(true);
    }
    for &level in wave {
        rx.rx_feed(level);
    }
    for _ in 0..cpb {
        rx.rx_feed(true);
    }
    rx.rx_read()
}

fn test_config() -> SerialConfig {
    SerialConfig {
        baud_rate: 9600,
        data_bits: 8,
        stop_bits: 1,
        cycles_per_bit: 10,
    }
}

#[test]
fn uart_tx_waveform_replays_into_rx_and_reconstructs_byte() {
    let cfg = test_config();
    let byte = 0xA5;
    let wave = capture_tx_frame(cfg, byte);
    assert_eq!(replay_into_rx(cfg, &wave), Some(byte));
}

#[test]
fn uart_roundtrip_covers_boundary_byte_values() {
    let cfg = test_config();
    for byte in [0x00u8, 0xFF, 0x01, 0x80, 0x3C, 0x55] {
        let wave = capture_tx_frame(cfg, byte);
        assert_eq!(
            replay_into_rx(cfg, &wave),
            Some(byte),
            "byte {byte:#04x} failed roundtrip"
        );
    }
}

#[test]
fn uart_two_stop_bit_frame_roundtrips() {
    // ASR-33 framing (8 data bits, 2 stop bits) with a short bit period.
    let cfg = SerialConfig {
        baud_rate: 110,
        data_bits: 8,
        stop_bits: 2,
        cycles_per_bit: 8,
    };
    let byte = 0x6B;
    let wave = capture_tx_frame(cfg, byte);
    assert_eq!(replay_into_rx(cfg, &wave), Some(byte));
}

#[test]
fn uart_multibyte_stream_roundtrips_in_order() {
    let cfg = test_config();
    let message = [0x48u8, 0x49]; // "HI"
    let mut received = Vec::new();
    for &byte in &message {
        let wave = capture_tx_frame(cfg, byte);
        received.push(replay_into_rx(cfg, &wave).expect("byte reconstructed"));
    }
    assert_eq!(received, message);
}

#[test]
fn keyboard_scan_maps_every_pressed_key_to_its_hex_keycode() {
    // Press one key per position, scan its row, decode the low column bit, and
    // confirm the derived keycode matches the physical layout (row*4 + col).
    for row in 0..4u8 {
        for col in 0..4u8 {
            let mut kb = MatrixKeyboard::new(1);
            kb.press(row, col);

            // Active-low one-hot row drive: pull the target row bit low.
            kb.set_row_drive(!(1 << row) & 0x0F);
            let cols = kb.read_columns();

            // Exactly the pressed column reads low.
            let low_cols: Vec<u8> = (0..4u8).filter(|c| cols & (1 << c) == 0).collect();
            assert_eq!(low_cols, vec![col], "row {row} col {col} column readback");

            let decoded = MatrixKeyboard::keycode(row, low_cols[0]).expect("valid keycode");
            assert_eq!(decoded, row * 4 + col);
        }
    }
}

#[test]
fn keyboard_press_then_debounce_emits_pressed_event_with_position() {
    let mut kb = MatrixKeyboard::new(3);
    kb.press(2, 1); // hex '9'

    // Below threshold: no accepted state change yet.
    kb.scan_debounce();
    kb.scan_debounce();
    assert!(kb.drain_events().is_empty());
    assert_eq!(kb.debounced_state(2, 1), KeyState::Released);

    // Threshold reached: a single pressed event fires.
    kb.scan_debounce();
    let events = kb.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].row, 2);
    assert_eq!(events[0].col, 1);
    assert_eq!(events[0].state, KeyState::Pressed);
    assert_eq!(MatrixKeyboard::keycode(events[0].row, events[0].col), Some(9));
}

#[test]
fn keyboard_release_after_press_emits_release_event() {
    let mut kb = MatrixKeyboard::new(1);
    kb.press(3, 3);
    kb.scan_debounce();
    assert_eq!(kb.debounced_state(3, 3), KeyState::Pressed);
    kb.drain_events();

    kb.release(3, 3);
    kb.scan_debounce();
    let events = kb.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, KeyState::Released);
}

#[test]
fn seven_seg_shift_chain_load_renders_multidigit_number() {
    // Two cascaded 4003 chips (10 bits each) feed a two-digit display.
    let mut disp = SevenSegDisplay::new(2);
    let d0 = bcd_to_segments(4) as u16; // rightmost digit
    let d1 = bcd_to_segments(2) as u16; // leftmost digit

    // Pack digit 0 into bits 0-7 of chip 0, digit 1 spanning chip 0 bits 8-9
    // and chip 1 bits 0-5.
    let sr0 = d0 | ((d1 & 0x03) << 8);
    let sr1 = (d1 >> 2) & 0x3F;
    disp.load_from_shift_chain(&[sr0, sr1]);

    assert_eq!(disp.segments(0), bcd_to_segments(4));
    assert_eq!(disp.segments(1), bcd_to_segments(2));
    // Render is most-significant-digit first.
    assert_eq!(disp.render_ascii(), "24");
}

#[test]
fn seven_seg_bcd_mode_decodes_every_hex_nibble() {
    // Store a raw nibble per digit and let BCD mode decode it; render each hex
    // glyph and confirm the full 0-F alphabet round-trips through the display.
    let expected = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'b', 'C', 'd', 'E', 'F',
    ];
    for (nibble, &glyph) in expected.iter().enumerate() {
        let mut disp = SevenSegDisplay::new(1);
        disp.set_bcd_mode(true);
        disp.set_raw(0, nibble as u8);
        assert_eq!(disp.segments(0), bcd_to_segments(nibble as u8));
        assert_eq!(disp.render_ascii(), glyph.to_string());
    }
}

#[test]
fn seven_seg_raw_and_bcd_paths_agree_on_segment_pattern() {
    // A raw pattern written directly and a BCD value decoded must produce the
    // same lit segments for the same digit glyph.
    let mut raw_disp = SevenSegDisplay::new(1);
    let mut bcd_disp = SevenSegDisplay::new(1);

    raw_disp.set_raw(0, bcd_to_segments(6));
    bcd_disp.set_bcd(0, 6, false);

    assert_eq!(raw_disp.segments(0), bcd_disp.segments(0));
    assert_eq!(raw_disp.render_ascii(), bcd_disp.render_ascii());
}
