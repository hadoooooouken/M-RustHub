# Technical Specification: MCHOSE G3 V2 Hardware Button Remapping

## 1. Overview and Architecture

Button remapping for the **MCHOSE G3 V2** mouse (both wired `PID 0x418C` and wireless 2.4G dongle `PID 0x4245`) is implemented entirely at the **hardware level** in on-board flash memory.

Once written to flash, bindings persist across power cycles, device reboots, and operating systems without requiring any background resident daemon, hook, or driver software (Set & Forget).

> [!IMPORTANT]
> **Controller Architecture Note:**
> An earlier draft of this documentation incorrectly assumed a generic Nordic/Telink `0x4D` report format. Live hardware validation and reverse-engineering of the official MCHOSE web driver (`https://www.mchose.com.cn/gmouse/`) confirmed that the MCHOSE G3 V2 uses a **YiJiaXun (YJX)** OEM microcontroller.
> 
> The YJX protocol uses **Interface 2** with the standard `0x55` magic frame, Command `0x08` (Read Keymap), and Command `0x09` (Write Keymap), identical to the rest of the mouse configuration protocol (DPI, polling rate, debounce, sleep).

---

## 2. USB HID Interface and Hardware Characteristics

- **Vendor ID (VID):** `0x3837` (MCHOSE) — used by both the wired mouse and the 2.4G dongle
- **Product ID (PID):**
  - Wired Mode: `0x418C`
  - 2.4G Wireless Dongle: `0x4245`
- **HID Interfaces:**
  - **Interface 0:** Standard HID Mouse (Motion, LMB, RMB, MMB, Wheel)
  - **Interface 1:** Standard HID Keyboard (Emitted hotkeys and keystrokes)
  - **Interface 2:** Vendor Configuration Interface
    - Usage Page: `0xFF01` (Vendor-defined)
    - Usage: `0x0010`
- **Packet Sizing:** 64-byte payload. On Windows `hidapi`, transmitted as 65 bytes with leading `0x00` Report ID.
- **Composite Re-enumeration:** When Command `0x09` writes new key mappings to flash, the dongle firmware performs an internal descriptor refresh. This causes a momentary (~100–300 ms) pause on composite mouse and keyboard endpoints before resuming normal operation.

---

## 3. Command Protocol Framing (`0x55` Header)

All configuration packets adhere to the standard MCHOSE YJX binary frame structure:

```text
+---------+---------+---------+---------+---------+---------+---------+---------+--------------------+
| Byte 0  | Byte 1  | Byte 2  | Byte 3  | Byte 4  | Byte 5  | Byte 6  | Byte 7  | Bytes 8 .. 63      |
| 0x55    | Cmd ID  | 0xA5    | Length  | SubLen  | 0x00    | 0x00    | 0x00    | Data Payload       |
+---------+---------+---------+---------+---------+---------+---------+---------+--------------------+
```

### Protocol Header Fields:
1. **Byte 0 — Magic Byte:** `0x55` (85)
2. **Byte 1 — Command ID:**
   - `0x08` (8) — **Read** Button Mappings (Get Keymap)
   - `0x09` (9) — **Write** Button Mappings (Set Keymap)
3. **Byte 2 — Magic Marker:** `0xA5` (165)
4. **Byte 3 — Length Parameter:**
   - `0x0B` (11) for Read (Cmd 8)
   - `0x22` (34) for Write (Cmd 9)
5. **Byte 4 — Payload Byte Count:** `0x20` (32 bytes = 8 slots × 4 bytes)
6. **Bytes 5..7 — Reserved:** `0x00, 0x00, 0x00`
7. **Bytes 8..39 — Slot Table:** 8 slots, 4 bytes each (total 32 bytes)
8. **Bytes 40..63 — Padding:** `0x00`

---

## 4. Hardware Slot Layout

The microcontroller firmware manages 8 consecutive button slots:

| Slot Index | Byte Offset | Firmware Function | Factory Default Bytes (4B) | M-RustHub Key Index |
| :---: | :---: | :--- | :--- | :---: |
| **`Slot 0`** | `8..12` | Left Mouse Button (LMB) | `[32, 1, 0, 0]` (`20 01 00 00`) | `0` |
| **`Slot 1`** | `12..16` | Right Mouse Button (RMB) | `[32, 2, 0, 0]` (`20 02 00 00`) | `1` |
| **`Slot 2`** | `16..20` | Middle Mouse Button (MMB) | `[32, 4, 0, 0]` (`20 04 00 00`) | `2` |
| **`Slot 3`** | `20..24` | Side Back (Mouse 4) | `[32, 8, 0, 0]` (`20 08 00 00`) | `4` |
| **`Slot 4`** | `24..28` | Side Forward (Mouse 5) | `[32, 16, 0, 0]` (`20 10 00 00`) | `3` |
| **`Slot 5`** | `28..32` | Top DPI Button | `[33, 85, 0, 0]` (`21 55 00 00`) | — |
| **`Slot 6`** | `32..36` | Scroll Up | `[33, 56, 1, 0]` (`21 38 01 00`) | — |
| **`Slot 7`** | `36..40` | Scroll Down | `[33, 56, 255, 0]` (`21 38 FF 00`) | — |

> [!NOTE]
> **Side Button Order in Firmware:**
> The G3 V2 firmware assigns `Slot 3` to Backward (`0x08`) and `Slot 4` to Forward (`0x10`).
> In M-RustHub UI and CLI indexing, buttons are ordered as `[LMB, RMB, MMB, Forward, Backward]`, so:
> - `mappings[3]` (Forward) maps to hardware `Slot 4`.
> - `mappings[4]` (Backward) maps to hardware `Slot 3`.

> [!IMPORTANT]
> **Slot 0 (LMB) is locked:**
> The firmware ignores remaps written to `Slot 0`: the left button keeps acting as left click (verified on hardware).
> The official web driver blocks it too: every assignment to key index `0` is rejected with the `leftBtnNoChange` toast ("Left button modification not allowed").
> M-RustHub always writes `[32, 1, 0, 0]` to `Slot 0` and reports it as Left Click.

> [!NOTE]
> **Factory Unprogrammed State:**
> If a button has never been reprogrammed or is set to factory defaults, reading flash may return all zeroes (`[0, 0, 0, 0]`). The driver interprets `[0, 0, 0, 0]` as the default physical action for that slot.

---

## 5. Command 8: Read Button Mappings (Get Keymap)

### Request Packet (64 bytes):
```hex
55 08 A5 0B 20 00 00 00 00 00 00 00 00 00 00 00 ... (padded with 00)
```

### Verified Hardware Response:
```hex
AA 08 A5 0B 20 00 00 00 [Slot 0] [Slot 1] [Slot 2] [Slot 3] [Slot 4] [Slot 5] [Slot 6] [Slot 7] ...
```
Example response with default mappings:
```hex
AA 08 A5 0B 20 00 00 00 20 01 00 00 20 02 00 00 20 04 00 00 20 08 00 00 20 10 00 00 21 55 00 00 21 38 01 00 21 38 FF 00 00 00 ...
```

---

## 6. Command 9: Write Button Mappings (Set Keymap)

### Request Packet (64 bytes):
Transmits all 8 slots (32 bytes) at offset 8:
```hex
55 09 A5 22 20 00 00 00 [Slot 0 (4B)] [Slot 1 (4B)] [Slot 2 (4B)] [Slot 3 (4B)] [Slot 4 (4B)] [Slot 5 (4B)] [Slot 6 (4B)] [Slot 7 (4B)] 00 00 ...
```

Example packet writing factory default bindings:
```hex
55 09 A5 22 20 00 00 00 20 01 00 00 20 02 00 00 20 04 00 00 20 08 00 00 20 10 00 00 21 55 00 00 21 38 01 00 21 38 FF 00 00 00 ...
```

### Verified Hardware ACK Response:
```hex
AA 09 A5 41 20 00 00 00 ...
```

---

## 7. Action Encoding Values

### 7.1 Mouse Actions (Prefix `0x20` / 32)
Format: `[0x20, button_mask, 0x00, 0x00]`
- **Left Click (LMB):** `[32, 1, 0, 0]` (`20 01 00 00`)
- **Right Click (RMB):** `[32, 2, 0, 0]` (`20 02 00 00`)
- **Middle Click (MMB):** `[32, 4, 0, 0]` (`20 04 00 00`)
- **Backward (Mouse 4):** `[32, 8, 0, 0]` (`20 08 00 00`)
- **Forward (Mouse 5):** `[32, 16, 0, 0]` (`20 10 00 00`)

### 7.2 Disabled Button (Prefix `0xFF` / 255)
- **Disable:** `[255, 0, 0, 0]` (`FF 00 00 00`)

### 7.3 Sensor & Wheel Actions (Prefix `0x21` / 33)
- **DPI Cycle (Loop):** `[33, 85, 0, 0]` (`21 55 00 00`)
- **Scroll Up:** `[33, 56, 1, 0]` (`21 38 01 00`)
- **Scroll Down:** `[33, 56, 255, 0]` (`21 38 FF 00`)
