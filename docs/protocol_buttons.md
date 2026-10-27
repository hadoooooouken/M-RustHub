# Technical Specification: MCHOSE G3 V2 Hardware Button Remapping

## 1. Overview and Architecture

Button remapping for the **MCHOSE G3 V2** mouse (both wired `PID 0x418C` and wireless 2.4G `PID 0x4245`) is implemented entirely at the **hardware level** using on-board flash memory.

All custom button bindings are persisted directly into the mouse's microcontroller. Once written, **no background daemon or system hooks are required** (Set & Forget). The mouse autonomously emits the configured HID signals (mouse clicks, keyboard strokes, hotkey combinations, and media actions) across all operating systems (Windows, Linux, macOS).

---

## 2. USB HID Interface and Transport

- **Vendor ID (VID):** `0x3837` (14391) / `0x0BDA` (Realtek dongle)
- **Product ID (PID):**
  - Wired Mode: `0x418C` (16780)
  - 2.4G Wireless Dongle: `0x4245` (16965)
- **HID Configuration Endpoint:**
  - **Usage Page:** `0xFF01` (Vendor-defined)
  - **Usage:** `0x0010`
- **Packet Sizing:** 64-byte payload + 1-byte Report ID prefix (total 65 bytes in Windows `hidapi`).
- **HID Report ID:** `0x4D` (decimal `77`, ASCII `'M'`).

---

## 3. Packet Framing (Unified M-Protocol)

Configuration packets follow the unified binary framing structure:

```text
+---------+---------+-------+--------+---------+---------+---------+----------+--------------------+----------+
| Byte 0  | Byte 1  | Byte 2| Byte 3 | Byte 4  | Byte 5  | Byte 6  | Byte 7   | Bytes 8 .. 8+N-1   | Byte 8+N |
| 0x4D    | 0x01    | Flags | Length | Cmd Low | Cmd High| BizCode | Sequence | Data Payload       | Checksum |
+---------+---------+-------+--------+---------+---------+---------+----------+--------------------+----------+
```

### Header Fields
1. **Byte 0 — Report ID:** `0x4D` (77 / ASCII `'M'`).
2. **Byte 1 — Protocol Version:** `0x01`.
3. **Byte 2 — Flags:** `0x01` (enables XOR checksum verification).
4. **Byte 3 — Payload Length ($N$):** Number of bytes in the Data Payload.
5. **Bytes 4..5 — Command ID (Little-Endian):**
   - `0x0001` (Low: `0x01`, High: `0x00`) — **Read** button mapping table (Get Keys).
   - `0x0101` (Low: `0x01`, High: `0x01`) — **Write** button binding (Set Key).
6. **Byte 6 — BizCode:** `0x00`.
7. **Byte 7 — Sequence:** `0x00`.
8. **Bytes 8..(8+N-1) — Data Payload:** Command-specific arguments.
9. **Byte (8+N) — Checksum (XOR):**
   Calculated as a cumulative bitwise XOR (`^`) across all bytes from index 2 up to the last payload byte:
   $$\text{Checksum} = \bigoplus_{i=2}^{7 + N} \text{packet}[i]$$

---

## 4. Physical Button Indexing (`keyIndex`)

The MCHOSE G3 V2 defines 6 hardware-configurable physical buttons:

| Index (`keyIndex`) | Physical Button | Factory Default Action | Internal Action Code |
| :---: | :--- | :--- | :--- |
| **`0`** | Left Mouse Button (LMB) | Left Click | `0x000001` |
| **`1`** | Right Mouse Button (RMB) | Right Click | `0x000002` |
| **`2`** | Middle Mouse Button (MMB / Scroll Click) | Middle Click | `0x000004` |
| **`3`** | Side Forward Button | Mouse 5 (Forward) | `0x000010` |
| **`4`** | Side Back Button (Retreat) | Mouse 4 (Back) | `0x000008` |
| **`5`** | Top DPI Button | DPI Stage Cycle | `0x01000000` |

---

## 5. Read Button Map Command: `0x0001` (Get Keymap)

### Request Format:
- `profileIndex` — On-board profile index (`0` = Profile 1, `1` = Profile 2, `2` = Profile 3).
- `offset` — Starting button index (`0`).
- `count` — Number of buttons to query (`6`).

```text
4D 01 01 03 01 00 00 00 [profileIndex] [offset] [count] [checksum] 00 00 ...
```

For Profile 0:
```hex
4D 01 01 03 01 00 00 00 00 00 06 05 00 00 00 00 ... (zero-padded to 64 bytes)
```

### Response Format:
The mouse returns an array of entries for all buttons. Each button entry is serialized as a typed descriptor (3 or 4 bytes per button) containing the action family prefix and value.

---

## 6. Write Button Map Command: `0x0101` (Set Keymap)

### Data Payload Layout:
1. **Byte 0 (`packet[8]`):** `profileIndex` (`0` for the active profile).
2. **Byte 1 (`packet[9]`):** `keyIndex` (`0..5`).
3. **Byte 2 (`packet[10]`):** `writeControl` — Control byte calculated as `(KeyNumber << 3) | 3`. For a single-key binding (`KeyNumber = 1`), this evaluates to `(1 << 3) | 3 = 0x0B` (11).
4. **Bytes 3..5/6 (`packet[11..]`):** Encoded action descriptor (see Section 7).

---

## 7. Action Code Definitions

### 7.1. Mouse Actions (Prefix `0x00`)
- **Left Click (LMB):** `0x000001` $\rightarrow$ payload bytes: `00 01 00`
- **Right Click (RMB):** `0x000002` $\rightarrow$ payload bytes: `00 02 00`
- **Middle Click (MMB):** `0x000004` $\rightarrow$ payload bytes: `00 04 00`
- **Back Click (Mouse 4):** `0x000008` $\rightarrow$ payload bytes: `00 08 00`
- **Forward Click (Mouse 5):** `0x000010` $\rightarrow$ payload bytes: `00 10 00`
- **Scroll Up:** `0x050000` $\rightarrow$ payload bytes: `05 00 00`
- **Scroll Down:** `0x050001` $\rightarrow$ payload bytes: `05 01 00`

### 7.2. DPI Functions (Prefix `0x01`)
- **DPI Cycle (Loop):** `0x01000000` $\rightarrow$ payload bytes: `01 00 00 00`
- **DPI + (Step Up):** `0x01000002` $\rightarrow$ payload bytes: `01 02 00 00`
- **DPI - (Step Down):** `0x01000003` $\rightarrow$ payload bytes: `01 03 00 00`

### 7.3. Keyboard Keys (Prefix `0x11`)
Format: `0x11` + HID Usage ID (1 byte) + `0x00`.
- Letters `A`–`Z`: `0x110004` – `0x11001D` (e.g. `'A'` = `11 04 00`)
- Digits `1`–`0`: `0x11001E` – `0x110027`
- `Enter`: `0x110028`
- `Escape`: `0x110029`
- `Backspace`: `0x11002A`
- `Tab`: `0x11002B`
- `Space`: `0x11002C`
- `F1`–`F12`: `0x11003A` – `0x110045`
- Arrow Keys (`Right`, `Left`, `Down`, `Up`): `0x11004F` – `0x110052`

### 7.4. Key Combinations / Shortcuts (Prefix `0x13`)
Format: `0x13` + Modifier Byte + Key Usage ID:
- **Modifier Bitmask:**
  - `0x01` = `Ctrl`
  - `0x02` = `Shift`
  - `0x04` = `Alt`
  - `0x08` = `Win / Meta`
- **Common Examples:**
  - `Ctrl + C`: `0x130106` $\rightarrow$ payload bytes: `13 01 06`
  - `Ctrl + V`: `0x130119` $\rightarrow$ payload bytes: `13 01 19`
  - `Ctrl + Z`: `0x13011D` $\rightarrow$ payload bytes: `13 01 1D`
  - `Alt + Tab`: `0x13042B` $\rightarrow$ payload bytes: `13 04 2B`

### 7.5. Multimedia / Consumer Control (Prefix `0x14`)
Format: `0x14` + Consumer Control Usage Code (1 byte) + `0x00`:
- **Volume Up:** `0x1400E9` $\rightarrow$ payload bytes: `14 E9 00`
- **Volume Down:** `0x1400EA` $\rightarrow$ payload bytes: `14 EA 00`
- **Mute:** `0x1400E2` $\rightarrow$ payload bytes: `14 E2 00`
- **Play / Pause:** `0x1400CD` $\rightarrow$ payload bytes: `14 CD 00`
- **Next Track:** `0x1400B5` $\rightarrow$ payload bytes: `14 B5 00`
- **Previous Track:** `0x1400B6` $\rightarrow$ payload bytes: `14 B6 00`

### 7.6. Disabled Button
- **Disable:** `0xFE0000` $\rightarrow$ payload bytes: `FE 00 00`

---

## 8. Complete Example: Remapping "Forward" to "Ctrl + C"

- **Goal:** Remap the side Forward button (`keyIndex = 3`) to `Ctrl + C`.
- **Command ID:** `0x0101`
- **Data Payload:** `[00, 03, 0B, 13, 01, 06]` (length: 6 bytes).
- **USB HID Output Report (Report ID 77):**
```hex
4D 01 01 06 01 01 00 00 00 03 0B 13 01 06 1B 00 ... (zero-padded to 64 bytes)
```
- **Checksum Calculation:**
  $$\text{Checksum} = \text{0x01} \oplus \text{0x06} \oplus \text{0x01} \oplus \text{0x01} \oplus \text{0x00} \oplus \text{0x00} \oplus \text{0x00} \oplus \text{0x03} \oplus \text{0x0B} \oplus \text{0x13} \oplus \text{0x01} \oplus \text{0x06} = \text{0x1B}$$
- The microcontroller commits the new binding into flash memory and returns an ACK response.
