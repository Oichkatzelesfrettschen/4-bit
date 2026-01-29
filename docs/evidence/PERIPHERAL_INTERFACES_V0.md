# Peripheral Interface Design (v0)

**Date**: 2026-01-29
**Status**: DESIGN DOCUMENT
**Target**: Phase 5+ implementation

---

## Overview

Era-appropriate peripherals for Intel MCS-4 systems.

---

## Peripheral Types

### 1. 7-Segment LED Display

**Interface**:
```rust
trait SevenSegmentDisplay {
    fn write_digit(&mut self, digit: u8, position: usize);
    fn set_brightness(&mut self, level: u8);
}
```

**Verilog**:
```verilog
module seven_seg (
    input wire [3:0] bcd,   // BCD input (0-9)
    output reg [6:0] seg    // Segments a-g
);
    always @(*) begin
        case (bcd)
            4'h0: seg = 7'b0111111;
            4'h1: seg = 7'b0000110;
            // ... etc
        endcase
    end
endmodule
```

### 2. Nixie Tube Driver

**Interface**:
```rust
trait NixieTubeDriver {
    fn write_digit(&mut self, digit: u8);  // BCD to decimal decoder
    fn enable_high_voltage(&mut self, enable: bool);
}
```

**Requirements**:
- BCD to decimal decoder (74141 or equivalent)
- High voltage driver (~170V for nixie tubes)
- Multiplexing for multiple tubes

### 3. Matrix Keyboard Scanner

**Interface**:
```rust
trait MatrixKeyboard {
    fn scan(&mut self) -> Option<u8>;  // Returns key code if pressed
    fn set_scan_rate(&mut self, hz: u32);
}
```

**Configuration**:
- 4x4 matrix (16 keys)
- Row scanning at 1 kHz
- Debouncing: 10ms threshold

### 4. Serial UART

**Interface**:
```rust
trait SerialUart {
    fn send_byte(&mut self, byte: u8);
    fn recv_byte(&mut self) -> Option<u8>;
    fn set_baud_rate(&mut self, baud: u32);
}
```

**Parameters**:
- Baud rates: 300, 1200, 9600, 115200
- 8-N-1 (8 data bits, no parity, 1 stop bit)
- RS-232 levels: +12V / -12V (requires level shifter)

---

## Implementation Priority

1. **7-Segment Display**: Highest (simple, visual feedback)
2. **Matrix Keyboard**: High (input device)
3. **Serial UART**: Medium (debugging, external communication)
4. **Nixie Tubes**: Low (specialized, requires HV driver)

---

**Status**: DESIGN FRAMEWORK ONLY - IMPLEMENTATION DEFERRED TO FUTURE PHASES
