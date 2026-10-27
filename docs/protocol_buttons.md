# Technical Specification: MCHOSE G3 V2 Hardware Button Remapping

## 1. Overview and Architecture

Button remapping for the **MCHOSE G3 V2** mouse (both wired `PID 0x418C` and wireless 2.4G `PID 0x4245`) is implemented entirely at the **hardware level** using on-board flash memory.

All custom button bindings are persisted directly into the mouse's microcontroller (YiJiaXun / YJX architecture). Once written, **no background daemon or system hooks are required** (Set & Forget). The mouse autonomously emits the configured HID signals (mouse clicks, keyboard strokes, hotkey combinations, and media actions) across all operating systems (Windows, Linux, macOS).

---

## 2. USB HID Interface and Transport

- **Vendor ID (VID):** `0x3837` (14391) / `0x0BDA` (Realtek dongle)
- **Product ID (PID):**
  - Wired Mode: `0x418C` (16780)
  - 2.4G Wireless Dongle: `0x4245` (16965)
- **HID Configuration Endpoint:**
  - **Interface:** Interface 2 (Vendor Config Interface)
  - **Usage Page:** `0xFF01` (Vendor-defined)
  - **Usage:** `0x0010`
- **Packet Sizing:** 64-byte payload. On Windows `hidapi`, transmitted as 65 bytes with leading `0x00` Report ID.

---

## 3. Command Protocol Framing

All control packets on the G3 V2 use the standard MCHOSE vendor framing:

```text
+---------+---------+---------+---------+---------+---------+---------+---------+--------------------+
| Byte 0  | Byte 1  | Byte 2  | Byte 3  | Byte 4  | Byte 5  | Byte 6  | Byte 7  | Bytes 8 .. 63      |
| 0x55    | Cmd ID  | 0xA5    | Length  | SubLen  | 0x00    | 0x00    | 0x00    | Data Payload       |
+---------+---------+---------+---------+---------+---------+---------+---------+--------------------+
```

### Protocol Header Fields
1. **Byte 0 — Magic Byte:** `0x55` (85).
2. **Byte 1 — Command ID:**
   - `0x08` (8) — **Read** Button Mappings (Get Keymap).
   - `0x09` (9) — **Write** Button Mappings (Set Keymap).
3. **Byte 2 — Magic Marker:** `0xA5` (165).
4. **Byte 3 — Length Parameter:**
   - `0x0B` (11) for Read (Cmd 8).
   - `0x22` (34) for Write (Cmd 9).
5. **Byte 4 — Payload Byte Count:** `0x20` (32 bytes = 8 slots × 4 bytes).
6. **Bytes 5..7 — Reserved:** `0x00, 0x00, 0x00`.

---

## 4. Hardware Slot Layout

The microcontroller defines 8 consecutive slots (4 bytes per slot) starting at byte offset 8:

| Slot | Offset | Physical Function | Factory Default Encoding (4 Bytes) | M-RustHub Key Index |
| :---: | :---: | :--- | :--- | :---: |
| **`Slot 0`** | `8..12` | Left Click (LMB) | `32, 1, 0, 0` (`0x20, 0x01, 0, 0`) | `0` |
| **`Slot 1`** | `12..16` | Right Click (RMB) | `32, 2, 0, 0` (`0x20, 0x02, 0, 0`) | `1` |
| **`Slot 2`** | `16..20` | Middle Click (MMB) | `32, 4, 0, 0` (`0x20, 0x04, 0, 0`) | `2` |
| **`Slot 3`** | `20..24` | Side Back (Mouse 4) | `32, 8, 0, 0` (`0x20, 0x08, 0, 0`) | `4` |
| **`Slot 4`** | `24..28` | Side Forward (Mouse 5)| `32, 16, 0, 0` (`0x20, 0x10, 0, 0`) | `3` |
| **`Slot 5`** | `28..32` | Top DPI Button | `33, 85, 0, 0` (`0x21, 0x55, 0, 0`) | — |
| **`Slot 6`** | `32..36` | Scroll Up | `33, 56, 1, 0` (`0x21, 0x38, 0x01, 0`) | — |
| **`Slot 7`** | `36..40` | Scroll Down | `33, 56, 255, 0` (`0x21, 0x38, 0xFF, 0`) | — |

> **Note on Side Buttons:** In hardware firmware, Slot 3 corresponds to Back (`32, 8`) and Slot 4 corresponds to Forward (`32, 16`). In UI/CLI indexing:
> - Button 3 = Forward (`mappings[3]`) $\rightarrow$ written to `Slot 4`
> - Button 4 = Backward (`mappings[4]`) $\rightarrow$ written to `Slot 3`

> **Note on Default Values:** When reading from flash, if a slot returns all zeroes (`[0, 0, 0, 0]`), the firmware operates in factory default mode for that button.

---

## 5. Read Button Map Command: `0x08` (Get Keymap)

### Request Packet (64 bytes):
```text
55 08 A5 0B 20 00 00 00 00 00 00 00 ... (zero-padded)
```

### Response Packet:
Returns 64 bytes starting with `AA 08 A5 ...`:
- Bytes 8..39 contain the 8 4-byte slots described above.

---

## 6. Write Button Map Command: `0x09` (Set Keymap)

### Request Packet (64 bytes):
```text
55 09 A5 22 20 00 00 00 [Slot 0 (4B)] [Slot 1 (4B)] [Slot 2 (4B)] [Slot 3 (4B)] [Slot 4 (4B)] [Slot 5 (4B)] [Slot 6 (4B)] [Slot 7 (4B)] 00 ...
```

### Response / ACK:
The mouse writes to flash and responds with `AA 09 A5 41 20 ...`.
Dongle/mouse briefly re-enumerates to apply new descriptors.

---

## 7. Action Encoding Values

### 7.1 Mouse Actions (Prefix `0x20` / 32)
- **Left Click:** `[32, 1, 0, 0]` (`0x20, 0x01, 0, 0`)
- **Right Click:** `[32, 2, 0, 0]` (`0x20, 0x02, 0, 0`)
- **Middle Click:** `[32, 4, 0, 0]` (`0x20, 0x04, 0, 0`)
- **Backward (Mouse 4):** `[32, 8, 0, 0]` (`0x20, 0x08, 0, 0`)
- **Forward (Mouse 5):** `[32, 16, 0, 0]` (`0x20, 0x10, 0, 0`)

### 7.2 Disabled Action
- **Disabled:** `[255, 0, 0, 0]` (`0xFF, 0, 0, 0`)
