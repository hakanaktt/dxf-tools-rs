# ACadSharp DWG Writer — Complete Architecture Reference

> **Purpose:** Reference document for implementing a Rust DWG writer based on the C# ACadSharp library.

---

## Table of Contents

1. [Overall Writing Flow](#1-overall-writing-flow)
2. [DwgWriter Entry Point](#2-dwgwriter-entry-point)
3. [DwgStreamWriter Hierarchy](#3-dwgstreamwriter-hierarchy)
4. [File Header Writers](#4-file-header-writers)
5. [Section Writers](#5-section-writers)
6. [DwgObjectWriter — Entity & Object Serialization](#6-dwgobjectwriter--entity--object-serialization)
7. [LZ77 Compression](#7-lz77-compression)
8. [Checksums & CRC](#8-checksums--crc)
9. [Version Format Differences](#9-version-format-differences)
10. [Constants, Sentinels & Magic Numbers](#10-constants-sentinels--magic-numbers)

---

## 1. Overall Writing Flow

### High-Level Sequence

```
DwgWriter.Write()
│
├── 1. writeHeader()           → DwgHeaderWriter   → Header section stream
├── 2. writeClasses()          → DwgClassesWriter   → Classes section stream
├── 3. writeSummaryInfo()      → (empty stream)
├── 4. writePreview()          → DwgPreviewWriter   → Preview section stream
├── 5. writeAppInfo()          → DwgAppInfoWriter   → AppInfo section stream
├── 6. writeFileDepList()      → (empty stream)
├── 7. writeRevHistory()       → (empty stream)
├── 8. writeAuxHeader()        → DwgAuxHeaderWriter → AuxHeader section stream
├── 9. writeObjects()          → DwgObjectWriter    → AcDbObjects section stream
│                                                      (also builds handlesMap)
├── 10. writeObjFreeSpace()    → (empty stream)
├── 11. writeTemplate()        → (empty stream)
├── 12. writeHandles()         → DwgHandleWriter    → Handles section stream
│                                                      (uses handlesMap from step 9)
└── 13. _fileHeaderWriter.WriteFile()  → Assembles final DWG file
```

Each `write*()` method:
1. Creates a `MemoryStream`
2. Invokes the appropriate section writer to fill it
3. Calls `_fileHeaderWriter.AddSection(sectionName, stream, isCompressed?, decompressedSize?)`

The file header writer collects all sections, then `WriteFile()` assembles them into the output stream with proper headers, record locators, compression (R2004+), and checksums.

### Data Flow Diagram

```
CadDocument
    │
    ▼
DwgWriter (orchestrator)
    │
    ├─── DwgHeaderWriter ──────► MemoryStream (header variables in bit format)
    ├─── DwgClassesWriter ─────► MemoryStream (DXF class table)
    ├─── DwgObjectWriter ──────► MemoryStream (all entities + objects serialized)
    │         │
    │         └─► handlesMap: Dictionary<ulong, long>  (handle → byte offset)
    │
    ├─── DwgHandleWriter ──────► MemoryStream (handle-to-offset map)
    ├─── DwgPreviewWriter ─────► MemoryStream (thumbnail/preview image)
    ├─── DwgAppInfoWriter ─────► MemoryStream (application info)
    ├─── DwgAuxHeaderWriter ───► MemoryStream (auxiliary header)
    │
    ▼
DwgFileHeaderWriter
    │
    ├─ For R13-R2000: writes 0x61-byte file header + section records + raw sections
    └─ For R2004+:    writes 0x100-byte header + compressed pages + section maps + second header
```

---

## 2. DwgWriter Entry Point

**File:** `DwgWriter.cs` (~415 lines)

### Constructor

```csharp
DwgWriter(Stream stream, CadDocument document)
```

- Stores output `_stream` and `_document`
- Creates `DwgFileHeader` based on `document.Header.Version`
- Initializes `_handlesMap = new Dictionary<ulong, long>()`

### Key Fields

| Field | Type | Purpose |
|-------|------|---------|
| `_stream` | `Stream` | Output DWG file stream |
| `_document` | `CadDocument` | Source CAD document |
| `_fileHeader` | `DwgFileHeader` | Version-appropriate file header data model |
| `_fileHeaderWriter` | `IDwgFileHeaderWriter` | Version-specific file assembly writer |
| `_handlesMap` | `Dictionary<ulong, long>` | Handle → byte-offset map (built by DwgObjectWriter, consumed by DwgHandleWriter) |

### Writer Selection

```
getStreamWriter(stream) → DwgStreamWriterBase (version-specific)
getFileHeaderWriter()   → IDwgFileHeaderWriter (version-specific)
```

File header writer selection:
| Version | Writer |
|---------|--------|
| AC1014 (R14) | `DwgFileHeaderWriterAC15` |
| AC1015 (R2000) | `DwgFileHeaderWriterAC15` |
| AC1018 (R2004) | `DwgFileHeaderWriterAC18` |
| AC1021 (R2007) | **NOT SUPPORTED** (throws `NotSupportedException`) |
| AC1024 (R2010) | `DwgFileHeaderWriterAC18` |
| AC1027 (R2013) | `DwgFileHeaderWriterAC18` |
| AC1032 (R2018) | `DwgFileHeaderWriterAC18` |

### Section Writing Pattern

Each section follows this pattern:

```csharp
private void writeHeader()
{
    MemoryStream stream = new MemoryStream();
    DwgHeaderWriter writer = new DwgHeaderWriter(stream, _document, _encoding);
    writer.Write();
    _fileHeaderWriter.AddSection(DwgSectionDefinition.Header, stream, true, (int)stream.Length);
}
```

For `writeObjects()`, the handlesMap is populated:

```csharp
private void writeObjects()
{
    MemoryStream stream = new MemoryStream();
    DwgObjectWriter writer = new DwgObjectWriter(stream, _document, _encoding, _handlesMap);
    writer.Write();
    _fileHeaderWriter.AddSection(DwgSectionDefinition.AcDbObjects, stream, true, (int)stream.Length);
}
```

---

## 3. DwgStreamWriter Hierarchy

### Class Hierarchy

```
IDwgStreamWriter (interface, ~40 methods)
    │
    └── DwgStreamWriterBase (abstract, 688 lines)
            │
            ├── DwgStreamWriterAC12 (R13/R14 base — empty)
            │       │
            │       └── DwgStreamWriterAC15 (R2000 — extrusion/thickness optimizations)
            │               │
            │               └── DwgStreamWriterAC18 (R2004 — color encoding)
            │                       │
            │                       └── DwgStreamWriterAC21 (R2007 — Unicode text)
            │                               │
            │                               └── DwgStreamWriterAC24 (R2010 — object type encoding)
            │
            └── DwgMergedStreamWriter (delegates to Main/Text/Handle sub-writers)
                    │
                    └── DwgmMergedStreamWriterAC14 (pre-R2004: text=main, only handle separate)
```

### Factory Methods

```csharp
// Single stream writer
static DwgStreamWriterBase GetStreamWriter(ACadVersion version, Stream stream, Encoding encoding)

// Merged stream writer (for section writers that need separate text/handle streams)
static DwgStreamWriterBase GetMergedWriter(ACadVersion version, Stream stream, Encoding encoding)
```

Factory selection for `GetStreamWriter`:
| Version | Class |
|---------|-------|
| AC1012 (R13) | `DwgStreamWriterAC12` |
| AC1014 (R14) | `DwgStreamWriterAC12` |
| AC1015 (R2000) | `DwgStreamWriterAC15` |
| AC1018 (R2004) | `DwgStreamWriterAC18` |
| AC1021 (R2007) | `DwgStreamWriterAC21` |
| AC1024+ | `DwgStreamWriterAC24` |

Factory selection for `GetMergedWriter`:
| Version | Class |
|---------|-------|
| ≤ AC1015 | `DwgmMergedStreamWriterAC14` (text=main, handle separate) |
| ≥ AC1018 | `DwgMergedStreamWriter` (main/text/handle all separate) |

### Bit-Level Writing — Core Methods

All DWG data is written at the bit level. The base class tracks:

```
Fields:
  _stream: Stream          — underlying byte stream
  _lastByte: byte          — current partial byte being built
  BitShift: int            — current bit position within _lastByte (0-7)
  PositionInBits: long     — total bit position in the stream
```

#### WriteBit(bool value)

```
If value=true:
    _lastByte |= (byte)(1 << (7 - BitShift))
BitShift++
If BitShift == 8:
    flush _lastByte to stream
    _lastByte = 0
    BitShift = 0
```

#### Write2Bits(byte value)

Writes a 2-bit value (0–3). Handles byte-boundary crossing:

```
If BitShift <= 6:
    value <<= (6 - BitShift)
    _lastByte |= value
    BitShift += 2
    If BitShift == 8: flush
Else (BitShift == 7):
    // Straddles byte boundary
    high bit → _lastByte
    flush _lastByte
    low bit → new _lastByte at position 7
    BitShift = 1
```

#### WriteBitShort(short value) — BS type

```
Encoding:
  value == 0    → Write2Bits(0b10)                          // 2 bits
  value == 256  → Write2Bits(0b11)                          // 2 bits
  0<value<256   → Write2Bits(0b01) + WriteByte(value)       // 2+8 = 10 bits
  otherwise     → Write2Bits(0b00) + WriteShort(value)      // 2+16 = 18 bits
```

#### WriteBitLong(int value) — BL type

```
Encoding:
  value == 0    → Write2Bits(0b10)                          // 2 bits
  0<value<256   → Write2Bits(0b01) + WriteByte(value)       // 2+8 = 10 bits
  otherwise     → Write2Bits(0b00) + WriteInt(value)        // 2+32 = 34 bits
```

#### WriteBitDouble(double value) — BD type

```
Encoding:
  value == 0.0  → Write2Bits(0b10)                          // 2 bits
  value == 1.0  → Write2Bits(0b01)                          // 2 bits
  otherwise     → Write2Bits(0b00) + WriteRawDouble(value)  // 2+64 = 66 bits
```

#### WriteBitLongLong(long value) — BLL type

```
// Count non-zero bytes needed
size = count of bytes in value (1-8), minimum 0 if value==0
Write3Bits(size)
For i in 0..size:
    WriteByte(value & 0xFF)
    value >>= 8
```

#### WriteBitDoubleWithDefault(double value, double def) — DD type

Compares byte patterns of `value` vs `default`:

```
Get 8 bytes of each as arrays
If identical:
    Write2Bits(0b00)                                    // no data needed
Else if bytes[4..7] differ, bytes[0..3] same:
    Write2Bits(0b01) + Write4Bytes(value[4..7])         // 2+32 = 34 bits
Else if bytes[0..5] differ, bytes[6..7] same:
    Write2Bits(0b10) + Write6Bytes(value[0..5])         // 2+48 = 50 bits
Else:
    Write2Bits(0b11) + WriteRawDouble(value)            // 2+64 = 66 bits
```

#### HandleReference — Handle encoding

```csharp
void HandleReference(ulong handle)     // absolute
void HandleReference(DwgReferenceType type, ulong handle) // reference type only
void HandleReference(IHandledCadObject handledObject)     // from object handle
void HandleReference(DwgReferenceType type, IHandledCadObject cadObject) // full
```

Handle byte format:

```
First byte: (type << 4) | size
   type = DwgReferenceType enum (0-5):
     0x00 = SoftPointer
     0x02 = HardPointer  
     0x03 = SoftOwnership
     0x04 = HardOwnership
     0x05 = Declaration
   size = number of handle bytes (0-8)

Following bytes: handle value in big-endian

Size calculation:
  Starting from value bytes[7] down to bytes[0],
  skip leading zero bytes, count remaining = size
```

**DwgReferenceType values for relative references:**
| Code | Description |
|------|-------------|
| 0x02 | Offset +1 from ref handle |
| 0x03 | Offset -1 from ref handle |
| 0x04 | SoftOwnership |
| 0x05 | Declaration / HardOwnership |
| 0x06 | Plus 1 |
| 0x08 | Minus 1 |
| 0x0A | Plus offset |
| 0x0C | Minus offset |

#### WriteRawLong / WriteRawShort / WriteRawDouble / WriteByte

These write full-width values at the current bit position, handling bit-shifting when `BitShift != 0`.

For `WriteByte(byte value)` at non-aligned position:

```
_lastByte |= (value >> BitShift)
flush _lastByte
_lastByte = (byte)(value << (8 - BitShift))
```

#### SavePositonForSize / SetPositionInBits / SetPositionByFlag

Used for size fields that need to be patched after content is written:

```csharp
long SavePositonForSize()
    // Returns current PositionInBits, advances by 32 bits (reserves space for a RL)

void SetPositionInBits(long posInBits)
    // Seeks to a specific bit position (flushes current byte, sets stream position)

void SetPositionByFlag(long pos)
    // Seeks to saved position, writes PositionInBits - pos - 32 as a RL (raw long)
    // Then restores original position
```

### DwgStreamWriterAC15 Overrides

**WriteBitExtrusion(XYZ value):**
```
If value == (0, 0, 1):  // AxisZ
    WriteBit(true)       // 1 bit: compressed
Else:
    WriteBit(false)      // 1 bit: full data follows
    WriteBitDouble(value.X)
    WriteBitDouble(value.Y)
    WriteBitDouble(value.Z)
```

**WriteBitThickness(double value):**
```
If value == 0.0:
    WriteBit(true)       // compressed: zero thickness
Else:
    WriteBit(false)
    WriteBitDouble(value)
```

### DwgStreamWriterAC18 Overrides

**WriteCmColor(Color value):**
```
WriteBitShort(value.Index)   // color index, hardcoded 0 in source
WriteBitLong(value.GetTrueColor())   // RGB as BL
WriteByte(value.GetTrueColorFlag())  // flag byte
```

**WriteEnColor(Color color, Transparency transparency):**
```
flags = color.Index (ushort)

If transparency is not ByLayer:
    flags |= 0x2000

If color has TrueColor:
    flags |= 0x8000

If color.BookName is not empty:
    flags |= 0x4000

WriteBitShort(flags)

If 0x8000 set:
    WriteBitLong(RGB value)

If 0x4000 set:
    WriteVariableText(color.BookName)

If 0x2000 set:
    WriteBitLong(transparency as BL)
```

### DwgStreamWriterAC21 Overrides

**WriteVariableText(string value):**
```
// Write as Unicode
ushort length = value.Length
WriteBitShort(length)
For each char:
    WriteRawShort(char)    // 16-bit Unicode code point
```

**WriteTextUnicode(string value):**
```
// Different from above — writes byte length + 2 null bytes
bytes = Encoding.Unicode.GetBytes(value)
WriteRawShort((short)(bytes.Length + 2))    // byte count including null terminator
WriteBytes(bytes)
WriteRawShort(0)   // null terminator (2 bytes)
```

### DwgStreamWriterAC24 Overrides

**WriteObjectType(ObjectType type):**
```
ushort index = (ushort)type

If index < 0x1F0:
    Write2Bits(0b00)
    WriteByte((byte)index)
Elif index < (0x1F0 + 0xFF):
    Write2Bits(0b01)
    WriteByte((byte)(index - 0x1F0))
Else:
    Write2Bits(0b10)
    WriteRawShort(index)
```

### DwgMergedStreamWriter — Multi-Stream Architecture

For R2004+, each object's data is split into three sub-streams:

| Stream | Content |
|--------|---------|
| **Main** | All data except text and handles |
| **Text** | All `WriteVariableText()` and `WriteTextUnicode()` calls |
| **Handle** | All `HandleReference()` calls |

The `DwgMergedStreamWriter` delegates calls:
- Default → Main stream writer
- `WriteVariableText` / `WriteTextUnicode` → Text stream writer
- `HandleReference` → Handle stream writer

**WriteSpearShift() — Merging the three streams:**

```
1. Main stream: flush and finalize
2. If TextWriter has data:
     Write size flag at saved position in main stream
     Copy text bytes to main stream with bit shifting
3. Copy handle bytes to main stream with bit shifting
4. Calculate total size in bits = main bits + text bits + handle bits
5. Patch the reserved size field at the saved position
```

For pre-R2004 (`DwgmMergedStreamWriterAC14`):
- Text and Main are the SAME writer (text is interleaved)
- Only Handle is a separate stream
- Simpler merging: just appends handle data to main + writes handle section size

---

## 4. File Header Writers

### IDwgFileHeaderWriter Interface

```csharp
interface IDwgFileHeaderWriter
{
    long HandleSectionOffset { get; set; }
    void AddSection(string name, MemoryStream stream, bool isCompressed, int decompressedSize);
    void WriteFile();
}
```

### R13–R2000: DwgFileHeaderWriterAC15

**File header: 0x61 bytes (97 bytes)**

```
Offset  Size  Description
──────  ────  ───────────
0x00    6     Version string: "AC1014" or "AC1015" (ASCII)
0x06    5     0x00, 0x00, 0x00, ACADMAINTVER(byte), 0x01
0x0B    4     Preview image seeker (4-byte LE offset, or -1)
0x0F    2     0x1B, 0x19
0x11    2     Code page (ushort LE)
0x13    4     Section record count = 6 (int32 LE)
0x17    N     Section records (6 records × 9 bytes each = 54 bytes)
0x4D    2     CRC (ushort LE, seed=0xC0C1)
0x4F    16    "Packed sentinel": 0x95,0xA0,0x4E,0x28,0x99,0x82,0x1A,0x E5,0x5E,0x41,0xE0,0x5F,0x9D,0x3A,0x4D,0x00
0x5F    2     Terminal bytes: 0x00, 0x00
```

**Section records** (each 9 bytes):

```
Byte 0:    Section number (byte)
Bytes 1-4: Seeker / file offset (int32 LE)
Bytes 5-8: Size in bytes (int32 LE)
```

Section numbers for R13-R2000:
| Number | Section |
|--------|---------|
| 0 | Header |
| 1 | Classes |
| 2 | Handles (Object Map) |
| 3 | ObjFreeSpace |
| 4 | Template |
| 5 | AuxHeader |

Sections without record numbers (Preview, AcDbObjects) are referenced via the file header directly (Preview via seeker, AcDbObjects implicitly positioned).

**Section order in file:**

```
[File Header: 0x61 bytes]
[Section 0: Header]
[Section 1: Classes]
[Section 3: ObjFreeSpace]
[Section 4: Template]
[Section 5: AuxHeader]
[AcDbObjects section (no record number)]
[Section 2: Handles]
[Preview section (referenced by seeker)]
```

Each section's seeker is the absolute file offset where its data starts. The writer computes these by summing accumulated sizes.

### R2004+: DwgFileHeaderWriterAC18

**File header: 0x100 bytes (256 bytes)**

Much more complex page-based architecture.

#### Overall file layout for R2004+:

```
[File Header: 0x100 bytes]
[Data Section Pages (compressed)]
  ├── Header section page(s)
  ├── Classes section page(s)
  ├── AcDbObjects section page(s)
  ├── Handles section page(s)
  ├── Preview section page(s)
  ├── AppInfo section page(s)
  ├── AuxHeader section page(s)
  ├── SummaryInfo section page(s)
  ├── ... other sections ...
[Section Map (descriptor page)]
[Section Page Map]
[Second File Header (0x100 bytes, at end)]
```

#### Data Section Page Format

Each data page has a 32-byte header:

```
Offset  Size  Description
──────  ────  ───────────
0x00    4     Section page type = 0x4163043B (int32 LE)
0x04    4     Section number (int32 LE, sequential within section)
0x08    4     Compressed data size (int32 LE)
0x0C    4     Decompressed data size (int32 LE)
0x10    4     Start offset within section (int32 LE)
0x14    4     Data checksum (Adler32, uint32 LE)
0x18    4     ODA value (= decompressed_size XOR start_offset) //unknown purpose
0x1C    4     Padding (filled with MagicSequence bytes)
```

After the header: LZ77-compressed data, padded to 0x20 alignment using MagicSequence bytes.

#### Section Map Descriptor Page

```
Section page type = 0x4163003B

Per section descriptor (each 32 bytes):
  Offset  Size  Description
  0x00    4     Size of section (compressed total)
  0x04    4     Number of pages
  0x08    4     Max decompressed page size (0x7400)
  0x0C    4     Unknown (written as 1)
  0x10    4     Compressed flag (1=compressed, 2=not)
  0x14    4     Section ID
  0x18    4     Encrypted flag (0)
  0x1C    8     Section name (null-padded ASCII)
```

Then per-page locator (each 8 bytes):
```
  0x00    4     Page number (1-based)
  0x04    4     Data size in page
```

#### Section Page Map

```
Section page type = 0x41630E3B

Contains sequential 8-byte entries:
  0x00    4     Page size (in file, including header)
  0x04    4     Negative sequential index (-1, -2, -3, ...)
```

These entries map file positions to pages. The page map records the size and order of every page in the file.

#### File Header (0x100 bytes)

```
Offset   Size  Description
──────   ────  ───────────
0x00     6     Version string (e.g., "AC1018" or "AC1032")
0x06     5     Zeros + ACADMAINTVER + 0x01
0x0B     4     Preview seeker (int32 LE)
0x0F     1     0x1B
0x10     1     0x19
0x11     2     Code page (ushort LE)
0x13     4     Section count = 0 (R2004+ uses page system)
0x17     2     CRC (ushort, seed 0xC0C1)
0x19     N     Padding with 0x00 to offset 0x20
0x20     0xE0  "Inner file header" (encrypted, see below)
```

#### Inner File Header (at offset 0x20, length 0xE0 = 224 bytes)

Written into a buffer, then encrypted by `applyMask()`:

```
Offset  Size   Description
──────  ────   ───────────
0x00    12     Signature: "AcFssFcAJMB\0"
0x0C    4      0x00 (unknown)
0x10    4      0x6C (unknown)
0x14    4      0x04 (unknown)
0x18    4      Root tree node gap (int32)
0x1C    4      Left tree node gap (int32)
0x20    4      Right tree node gap (int32)
0x24    4      Unknown (0)
0x28    4      Last section page ID (int32)
0x2C    8      Last section page address (int64 LE)
0x34    8      Second header data address (for 2nd copy)
0x3C    4      Gap amount (uint32)
0x40    4      Section page amount (uint32)
0x44    4      0x20 (unknown, constant)
0x48    4      0x80 (unknown, constant)
0x4C    4      0x40 (unknown, constant)
0x50    4      Section page map ID (uint32)
0x54    8      Section page map address (uint64 LE, but only lower 32 bits set)
0x5C    4      Section map ID (uint32)
0x60    4      Section page array size (uint32)
0x64    4      Gap array size (uint32, = 0)
0x68    4      CRC32 of bytes [0x00..0x68)
0x6C    ...    MagicSequence-XOR'd padding (0x6C to 0xE0)
```

**Encryption: `applyMask(buffer, offset, length)`**

```
For each 4-byte block at position i:
    mask = 0x4164536B XOR (i + streamPosition)
    buffer[i..i+3] ^= mask (as uint32 LE)
```

**Encryption: `applyMagicSequence(buffer, offset, length)`**

```
For each byte at position i:
    buffer[offset + i] ^= MagicSequence[i]
```

Where `MagicSequence` is 256 bytes generated by:
```
seed = 1
For i in 0..256:
    seed = seed * 0x343FD + 0x269EC3
    MagicSequence[i] = (byte)(seed >> 16)
```

#### CRC32 for File Headers

The CRC32 checksum at offset 0x68 in the inner file header uses a custom Adler32-like algorithm:

```
sum1 = seed & 0xFFFF
sum2 = seed >> 16
For each byte b in buffer:
    sum1 = (sum1 + b) % 0xFFF1
    sum2 = (sum2 + sum1) % 0xFFF1
result = (sum2 << 16) | (sum1 & 0xFFFF)
```

This is NOT standard CRC32 — it's an **Adler-32** variant with modulus `0xFFF1`.

### R2007: DwgFileHeaderWriterAC21

Inherits AC18 but with:
- `fileHeaderSize = 0x480` (instead of 0x100)
- Uses `DwgLZ77AC21Compressor` (which is **NOT IMPLEMENTED** — throws `NotImplementedException`)
- The file header writing is incomplete/non-functional

**⚠️ R2007 DWG writing is NOT supported.**

---

## 5. Section Writers

### 5.1 DwgHeaderWriter

**File:** `DwgHeaderWriter.cs` (1095 lines)

Writes all `CadHeader` variables in DWG bit format.

#### Structure

```
[Start Sentinel: 16 bytes]
    0xCF,0x7B,0x1F,0x23,0xFD,0xDE,0x38,0xA9,0x5F,0x7C,0x68,0xB8,0x4E,0x6D,0x33,0x5F
[Size field: RL (raw long, 32 bits)]
[Header variables data — bit-packed]
[CRC: RS (raw short, 16 bits), seed 0xC0C1]
[End Sentinel: 16 bytes]
    0x30,0x84,0xE0,0xDC,0x02,0x21,0xC7,0x56,0xA0,0x83,0x97,0x47,0xB1,0x92,0xCC,0xA0
```

For R2004+ (≥ AC1018), uses a **merged writer** (separate main/text/handle streams).

#### Header Variables — Writing Order

The variables are written in a specific order. Key groups:

**1. Core variables (all versions):**
```
BD  Unknown (0.0)               // first
BD  Unknown (0.0)
BD  Unknown (0.0)
BD  Unknown (0.0)
TV  Unknown ("")                // empty string
TV  Unknown ("")
TV  Unknown ("")
TV  Unknown ("")
BL  Unknown (0x00)
BL  Unknown (0x00)
--- Conditionals for R13-R14 ---
BS  CurrentEntityLinetypeScale  (short, R13-R14 only)
--- End R13-R14 ---
--- R2004+ only ---
B   Unknown                     
--- End R2004+ ---
BS  InsUnits
BS  CELTScale (lindist)
B   EXTNAMES
BS  SplineType (PSLinetypeScale)
--- R2004+ additions ---
B   unknown
B   unknown
--- End R2004+ ---
```

**2. Measurement/drawing variables:**
```
BD  LTSCALE, TEXTSIZE, TRACEWID, SKETCHINC, FILLETRAD, THICKNESS, ANGBASE
BS  AUNITS
BS  AUPREC
TV  MENU
BD  DIMSCALE (and ~50 dimension variables)
```

**3. Dimension variables — version-dependent:**

For R13–R14, dimension variables are written inline as individual fields.
For R2000+, many dimension variables are grouped differently with handles.

**4. Table control handles (critical for cross-referencing):**
```
BLOCK_CONTROL_OBJECT handle (H)
LAYER_CONTROL_OBJECT handle (H)
STYLE_CONTROL_OBJECT handle (H)
LINETYPE_CONTROL_OBJECT handle (H)
VIEW_CONTROL_OBJECT handle (H)
UCS_CONTROL_OBJECT handle (H)
VPORT_CONTROL_OBJECT handle (H)
APPID_CONTROL_OBJECT handle (H)
DIMSTYLE_CONTROL_OBJECT handle (H)
VIEWPORT_ENTITY_HEADER_CONTROL (R13-R14 only)
```

**5. Named handles (R2000+):**
```
DICTIONARY_ACAD_GROUP handle
DICTIONARY_ACAD_MLINESTYLE handle
DICTIONARY_NAMED_OBJECTS handle
...
```

**6. R2004+ extended header variables (AC1018+):**
```
BS  SORTENTS
BS  INDEXCTL  
BS  HIDETEXT
...many more...
```

**7. R2007+ variables (AC1021+):**
```
B   CameraDisplay
BL  Unknown
BL  Unknown  
BD  StepsPerSec
BD  StepSize
BD  SwdashPercent (3DDWFPREC)
BD  LensLength
BD  CameraHeight
RC  SolidsRetainHistory
RC  ShowSolidsHistory
BD  SwdashPercent
BD  LensLength
```

### 5.2 DwgClassesWriter

**File:** `DwgClassesWriter.cs`

#### Structure

```
[Start Sentinel: 16 bytes]
[Size field: RL]
[Classes data — bit-packed]
[CRC: RS, seed 0xC0C1]
[End Sentinel: 16 bytes]
```

For R2004+: additional fields after sentinel size:
```
BS  Max class number (BL for R2004+, should be max class number)
RC  0x00
RC  0x00
B   true
```

#### Per-Class Data

```
BS  classNumber
BS  proxyFlags
TV  applicationName
TV  cppClassName
TV  dxfName
B   wasZombie
BS  itemClassId
```

R2004+ additions per class:
```
BL  numberOfObjects (instance count)
BL  dwgVersion
BL  maintenanceVersion
BL  unknown1 (0)
BL  unknown2 (0)
```

### 5.3 DwgHandleWriter

**File:** `DwgHandleWriter.cs`

Writes the handle-to-offset map (Object Map / Handles section).

#### Structure

The section consists of chunks, each with:
```
[2 bytes: chunk data size (ushort BE? or LE)]
[chunk data: handle-offset pairs using modular encoding]
[2 bytes: CRC of chunk data, seed 0xC0C1]
```

Last chunk is empty:
```
[2 bytes: 0x0000 (zero size)]
[2 bytes: CRC]
```

#### Chunk Data Format

Handles are sorted by handle value. Each entry encodes:
- **Handle delta** (current handle – previous handle): encoded as **modular short**
- **Offset delta** (current offset – previous offset): encoded as **signed modular short**

Maximum chunk data size: 2032 bytes.

#### Modular Short Encoding

Used for handle deltas (unsigned):

```
do:
    byte = value & 0x7F
    value >>= 7
    if value != 0:
        byte |= 0x80   // continuation bit
    write byte
while value != 0
```

#### Signed Modular Short Encoding

Used for offset deltas (signed):

```
// WriteSmodularChar
byte lowByte = (byte)(value & 0x3F)
int remainder = value >> 6

If value is negative:
    lowByte |= 0x40    // sign bit
    // Special handling for negative numbers

lowByte |= (remainder != 0) ? 0x80 : 0x00  // continuation bit
write lowByte

While remainder != 0:
    byte nextByte = (byte)(remainder & 0x7F)
    remainder >>= 7
    nextByte |= (remainder != 0) ? 0x80 : 0x00
    write nextByte
```

### 5.4 DwgAppInfoWriter

**File:** `DwgAppInfoWriter.cs`

```
Offset  Type   Description
──────  ────   ───────────
0x00    RC     Class version = 3
0x01    2+N    App info name (length-prefixed Unicode string)
        16     Checksum placeholder (16 zero bytes)
        2+N    Version string (Unicode)
        16     Checksum placeholder (16 zero bytes)
        2+N    Comment string (Unicode)
        16     Checksum placeholder (16 zero bytes)
        2+N    Product string (Unicode)
        16     Checksum placeholder (16 zero bytes)
```

### 5.5 DwgAuxHeaderWriter

**File:** `DwgAuxHeaderWriter.cs`

```
Offset  Type   Description
──────  ────   ───────────
0x00    3      Magic bytes: 0xFF, 0x77, 0x01
0x03    RS     DWG version (as short, e.g., 0x17 for AC1015)
0x05    RS     Maintenance version
0x07    RL     Save count (num_saves + 1)
0x0B    RL     Save count again (-1)
0x0F    RS     0x0000
0x11    RS     DWG version again
0x13    RL     Maintenance version (as int)
0x17    RS     DWG version again
0x19    RS     Maintenance version
0x1B    RS×6   TDCREATE as 6 raw shorts
0x27    RS×6   TDUPDATE as 6 raw shorts
0x33    RL     Handseed (if ≤ int range) or -1
```

### 5.6 DwgPreviewWriter

**File:** `DwgPreviewWriter.cs`

```
[Start Sentinel: 16 bytes]
    0x1F,0x25,0x6D,0x07,0xD4,0x36,0x28,0x28,0x9D,0x57,0xCA,0x3F,0x9D,0x44,0x10,0x2B
[Overall size: RL]
[Image present counter: RC (0-2)]
If images present:
    [Per image (up to 2): byte code, int start, int size]
    [Header data bytes]
    [Image data bytes]  
[End Sentinel: 16 bytes]
    0xE0,0xDA,0x92,0xF8,0x2B,0xC9,0xD7,0xD7,0x62,0xA8,0x35,0xC0,0x62,0xBB,0xEF,0xD4
```

Image types: `HeaderData = 1`, `BmpImage = 2`, `WmfImage = 3`

---

## 6. DwgObjectWriter — Entity & Object Serialization

### Architecture

**Files:**
- `DwgObjectWriter.cs` (1397 lines) — Main orchestrator + dispatch
- `DwgObjectWriter.Common.cs` (512 lines) — Common entity/object data patterns
- `DwgObjectWriter.Entities.cs` (2634 lines) — Entity-specific serialization
- `DwgObjectWriter.Objects.cs` (1005 lines) — Non-graphical object serialization

### 6.1 Main Orchestrator (`DwgObjectWriter.cs`)

#### Constructor

```csharp
DwgObjectWriter(Stream stream, CadDocument document, Encoding encoding, 
                Dictionary<ulong, long> handlesMap)
```

#### Write() Method — Object Serialization Order

```
1. If R2004+: write 0x0DCA prefix (BL)
2. Add RootDictionary to processing queue
3. Write Block Control + Block Records (writeBlockControl)
4. Write Tables:
   - Layers
   - TextStyles  
   - LineTypes
   - Views
   - UCSs
   - VPorts
   - AppIds
   - DimensionStyles
5. Write Block Entities (writeBlockEntities)
   - For each BlockRecord: write all contained entities
   - Child entities (Insert→Attributes, Polyline→Vertices) via writeChildEntities
6. Write Objects (writeObjects)
   - Process queue iteratively
   - Each CadObject → writeObject() dispatcher → type-specific writer
```

#### `writeTable<T>()` — Table Entry Writing Pattern

```
For each entry in table:
    writeCommonNonEntityData(entry)
    [table-specific fields]
    registerObject(entry)
```

#### Entity Object Layout (Per-Object Byte Stream)

Each serialized object in the AcDbObjects section:

```
[Modular Short: object data size in bytes]
[Object data — bit-packed]:
    [Object type: BS or encoded type (AC24+)]
    [Reserved size: RL placeholder (32 bits, patched later)]
    [Handle: HandleReference]
    [Extended Entity Data (EED)]
    [--- type-specific common data ---]
    [--- type-specific fields ---]
    [--- handle references (in handle stream for R2004+) ---]
    [Merged streams: main + text + handles (WriteSpearShift)]
[R2010+: Modular Char — handle stream bit size]
[CRC: RS, seed 0xC0C1]
```

#### `registerObject()` — Finalizing Each Object

```csharp
void registerObject(CadObject cadObject)
{
    // 1. Flush bit writer, finalize merged streams
    _writer.WriteSpearShift();

    // 2. Get serialized data as byte array
    byte[] data = _msmain.GetBuffer();
    int dataLength = (int)_msmain.Length;

    // 3. Write to output stream:
    //    a. Modular short: data size
    writeSizeModularShort((int)_msmain.Length);
    //    b. Raw data bytes
    _stream.Write(data, 0, dataLength);
    //    c. R2010+: modular char for handle stream size in bits
    //    d. CRC (2 bytes)

    // 4. Record in handles map
    Map[cadObject.Handle] = objectFilePosition;

    // 5. Reset streams for next object
    _msmain.SetLength(0);
    _msmain.Position = 0;
}
```

### 6.2 Common Data Patterns (`DwgObjectWriter.Common.cs`)

#### `writeCommonData(CadObject)` — First Part of Every Object

```
1. WriteObjectType(type)         // BS (pre-AC24) or encoded (AC24+)
2. SavePositonForSize()          // Reserve 32 bits for size
3. HandleReference(obj.Handle)   // Object's own handle
4. writeExtendedData(obj.EED)    // Extended Entity Data
```

#### Extended Entity Data (EED) Format

```
For each entry in EED:
    BS  appHandle reference length
    RC  bytes of app handle
    [EED data items]:
        RC  type code
        Followed by type-specific data:
          0: TV string (RC length + chars + RC closing tag)
          1: (invalid, skipped)
          2: RC '{' open brace
          3: RL layer table ref
          4: RC chunk (binary data)
          5: BL entity handle
         10: 3RD (3 raw doubles)
         11: 3RD (3 raw doubles)  
         12: 3RD (3 raw doubles)
         13: 3RD (3 raw doubles)
         40: RD (raw double)
         41: RD
         42: RD
         70: RS (raw short)
         71: RL (raw long)
    BS  0  (terminator — zero length for next app)
```

#### `writeCommonNonEntityData(CadObject)` — For Table Entries & Objects

```
writeCommonData(obj)
HandleReference(HardOwnership, owner)

// Reactors
BL  reactorCount
For each reactor:
    HandleReference(HardPointer, reactor)

// XDictionary
HandleReference(HardOwnership, xDictionary) // or NullHandle
```

#### `writeCommonEntityData(Entity)` — For Graphical Entities

```
writeCommonData(entity)

// Graphic data presence
B   hasGraphicData (always false in this implementation)

// R2004+
BL  objectSize (total bits, patched later)

// Entity mode (2 bits)
//   00 = explicit owner handle follows
//   01 = entity is in Paper Space
//   10 = entity is in Model Space (most common)
2B  entityMode

// Reactors
BL  numReactors

// R2004+
B   xDictMissing
B   hasDs (always false)

// R13-R14: isbylayerlt, nolinks flags
// Pre-R2004: prev/next entity handles

// Color
WriteEnColor(color, transparency)

// Linetype scale
BD  linetypeScale

// R2000+
2B  linetypeFlags   // 00=by layer, 01=by block, 10=continuous, 11=handle follows
2B  plotStyleFlags  // 00=by layer, 01=by block, 10=handle follows

// R2007+
2B  materialFlags
RC  shadowFlags

// Invisibility
BS  invisible (1 = invisible, 0 = visible)

// R2000+ lineweight
RC  lineweight

// --- Handle references (in handle stream) ---
// R2004+:
HandleReference(SoftPointer, subentityRefHandle)    // only if entityMode==0
HandleReference(SoftPointer, layer)

// Conditional handles:
If linetypeFlags == 0b11:
    HandleReference(SoftPointer, linetypeHandle)
If R2000+ and plotStyleFlags != 0b11:
    HandleReference(SoftPointer, plotStyleHandle)
If R2007+ and materialFlags == 0b11:
    HandleReference(SoftPointer, materialHandle)
If R2007+ and shadowFlags == 0b11:
    HandleReference(SoftPointer, shadowHandle)

// R13-R2000: prev/next entity handles  
If nolinks == false:
    HandleReference(HardPointer, prevEntity)
    HandleReference(HardPointer, nextEntity)
```

### 6.3 Entity Serialization Details (`DwgObjectWriter.Entities.cs`)

#### Entity Type Dispatch

```csharp
switch (entity) {
    case Arc:            writeEntity(arc);           break;
    case Circle:         writeEntity(circle);        break;
    case DimensionAligned:    writeEntity(dim);      break;
    case DimensionAngular2Line: ...
    case DimensionAngular3Pt: ...
    case DimensionDiameter: ...
    case DimensionLinear: ...
    case DimensionOrdinate: ...
    case DimensionRadius: ...
    case Ellipse:        writeEntity(ellipse);       break;
    case Insert:         writeEntity(insert);        break;
    case Face3D:         writeEntity(face);          break;
    case Hatch:          writeEntity(hatch);         break;
    case Leader:         writeEntity(leader);        break;
    case Line:           writeEntity(line);          break;
    case LwPolyline:     writeEntity(poly);          break;
    case Mesh:           writeEntity(mesh);          break;
    case MLine:          writeEntity(mline);         break;
    case MText:          writeEntity(mtext);         break;
    case MultiLeader:    writeEntity(mleader);       break;
    case Ole2Frame:      writeEntity(frame);         break;
    case PdfUnderlay:    writeEntity(underlay);      break;
    case Point:          writeEntity(point);         break;
    case Polyline2D:     writeEntity(poly);          break;
    case Polyline3D:     writeEntity(poly);          break;
    case PolyfaceMesh:   writeEntity(poly);          break;
    case PolygonMesh:    writeEntity(poly);          break;
    case Ray:            writeEntity(ray);           break;
    case Shape:          writeEntity(shape);         break;
    case Solid:          writeEntity(solid);         break;
    case Solid3D:        writeEntity(solid3d);       break; // empty
    case Spline:         writeEntity(spline);        break;
    case CadWipeoutBase: writeEntity(wipeout);       break;
    case TextEntity:     writeEntity(text, insert?); break;
    case Tolerance:      writeEntity(tolerance);     break;
    case Vertex2D:       writeEntity(vertex);        break;
    case Vertex3D:       writeEntity(vertex3d);      break;
    case VertexFaceMesh: writeEntity(vertex);        break;
    case VertexFaceRecord: writeEntity(record);      break;
    case Viewport:       writeEntity(viewport);      break;
    case XLine:          writeEntity(xline);         break;
}
```

#### Selected Entity Formats

**Line:**
```
writeCommonEntityData()
R13-R14:
    3BD  startPoint
    3BD  endPoint
R2000+:
    B    zIsZero (start.Z==0 && end.Z==0)
    RD   start.X
    DD   end.X (default=start.X)
    RD   start.Y
    DD   end.Y (default=start.Y)
    If !zIsZero:
        BD  start.Z
        BD  end.Z
WriteBitThickness(thickness)
WriteBitExtrusion(normal)
// Handles written by writeCommonEntityData
```

**Circle:**
```
writeCommonEntityData()
3BD  center
BD   radius
WriteBitThickness(thickness)
WriteBitExtrusion(normal)
```

**Arc:**
```
writeCommonEntityData()
3BD  center
BD   radius
WriteBitThickness(thickness)
WriteBitExtrusion(normal)
BD   startAngle
BD   endAngle
```

**Ellipse:**
```
writeCommonEntityData()
3BD  center
3BD  endPoint (relative to center — semi-major axis endpoint)
3BD  normal (extrusion)
BD   radiusRatio (minor/major)
BD   startAngle
BD   endAngle
```

**Point:**
```
writeCommonEntityData()
3BD  point
WriteBitThickness(thickness)
WriteBitExtrusion(normal)
BD   xAxisAngle
```

**3DFace:**
```
writeCommonEntityData()
R13-R14:
    3BD  corner1, corner2, corner3, corner4
R2000+:
    B    hasNoFlags (flags==0)
    B    zIsZero (all corners Z==0)
    RD   corner1.X
    DD   corner2.X (default=corner1.X)
    DD   corner3.X (default=corner2.X)
    DD   corner4.X (default=corner3.X)
    RD   corner1.Y
    DD   corner2.Y (default=corner1.Y)
    DD   corner3.Y (default=corner2.Y)
    DD   corner4.Y (default=corner3.Y)
    If !zIsZero:
        RD  corner1.Z
        DD  corner2.Z...corner4.Z
    If !hasNoFlags:
        BS  flags (invisibleEdge flags)
```

**Text (also Attribute / AttDef base):**
```
writeCommonEntityData()
R13-R14:
    BD   elevation (Z of insertion)
    2RD  insertion (X, Y)
    2RD  alignment (X, Y)
    WriteBitExtrusion(normal)
    WriteBitThickness(thickness)
    BD   obliqueAngle
    BD   rotation
    BD   height
    BD   widthFactor
    TV   value
    BS   generation (mirror flags)
    BS   horizontalAlignment
    BS   verticalAlignment
R2000+:
    RC   dataFlags
    If !(flags&0x01): RD  elevation
    2RD  insertion
    If !(flags&0x02): 2DD alignment (default=insertion)
    WriteBitExtrusion(normal)
    WriteBitThickness(thickness)
    If !(flags&0x04): BD obliqueAngle
    If !(flags&0x08): BD rotation
    BD   height
    If !(flags&0x10): BD widthFactor
    TV   value
    If !(flags&0x20): BS generation
    If !(flags&0x40): BS horizontalAlignment
    If !(flags&0x80): BS verticalAlignment

Handles:
    HandleReference(SoftPointer, textStyle)
```

For **Attribute**: same as Text + tag, flags, R2010+ version/type.
For **AttDef**: same as Text + tag, prompt, flags, R2010+ version/type, lockPosition.

**Insert (Block Reference):**
```
writeCommonEntityData()
3BD  insertionPoint
R13-R14:
    3BD  scale
R2000+:
    // Scale compression
    41 = scaleX, 42 = scaleY, 43 = scaleZ
    dataFlags:
        If all three == 1.0: write 2Bits(0b11) — no data
        Elif all equal and != 1.0: write DD for one axis
        Else: individual DDs for each
BD   rotation
WriteBitExtrusion(normal)
B    hasAttributes
If hasAttributes:
    BL   ownedObjectCount

Handles:
    HandleReference(SoftPointer, blockHeader)
    If hasAttributes:
        HandleReference(SoftPointer, firstAttribute)   // R13-R2000
        HandleReference(SoftPointer, lastAttribute)     // R13-R2000
        — or for R2004+:
        HandleReference(HardOwnership, each attribute)  // per attribute
        HandleReference(HardPointer, seqend)
```

**LwPolyline (Lightweight Polyline):**
```
writeCommonEntityData()
BS   flags (closed, plinegen, const width)
If hasConstWidth:
    BD   constWidth
If hasElevation:
    BD   elevation
If hasThickness:
    BD   thickness
If hasNormal:
    3BD  normal
BL   numPoints
BL   numBulges (if any)
// R2010+:
BL   vertexIdCount

// Vertex coordinates — differential encoding:
2RD  firstPoint (X, Y)
For remaining points:
    2DD  point (default = previous point)

For each bulge:
    BD   bulge

// Width pairs (if not const width):
For each width:
    BD   startWidth
    BD   endWidth

// R2010+: vertex IDs
For each vertex:
    BL   vertexId
```

**Spline:**
```
writeCommonEntityData()
// R2013+:
BS   splineType (1 or 2)
If type == 1:
    BL   degree
    B    periodic
    BL   numKnots
    BL   numControlPoints
    B    weighted
    For each knot: BD
    For each ctrlPt:
        3BD  point
        If weighted: BD weight
Elif type == 2:
    BD   fitTolerance
    3BD  startTangent
    3BD  endTangent
    BL   numFitPoints
    For each fitPt: 3BD point

// Pre-R2013:
BS   scenario (1=hasfit data, 2=has control data)
If hasfit:
    BS   degree
    BD   fitTolerance  
    3BD  startTangent
    3BD  endTangent
    BL   numFitPoints
    For each: 3BD point
If hascontrol:
    BS   degree
    BD   ctrlTolerance
    BD   knotTolerance
    BL   numKnots
    BL   numControlPoints
    B    weighted
    For each knot: BD
    For each ctrlPt: 3BD + optional BD weight
```

**Hatch:**
```
writeCommonEntityData()
// Hatch is the most complex entity, heavy use of nested loops
// Series of: gradientData (R2004+), elevation, extrusion, patternName, 
// solidFill flag, associativity, loopCount
// Then per loop: loop type, edges or polyline data
// Then pattern lines, seed points, boundary handles
// [Very large — see source for full details]
```

**MText:**
```
writeCommonEntityData()
3BD  insertionPoint
3BD  direction  
WriteBitExtrusion(normal)
BD   width (reference rect width)
// R2007+: BD height (reference rect height)
BS   attachmentPoint
// R2018+: 
//   RC  columnType
//   If columnType:
//     BL  columnCount, B  reversed, B  autoHeight, BD  width, BD  gutter
//     If !autoHeight: BD per column height
BS   drawingDirection
// R2007+: BD  annotationHeight
BD   extentsWidth
BD   extentsHeight
TV   value (RTF text content)
BS   lineSpacingStyle
BD   lineSpacingFactor
B    unknown
// R2004+:
BL   backgroundFlags
If flags > 0:
    BL  background scale factor
    BL  background color
    BL  background transparency
Handles:
    HandleReference(SoftPointer, style)
```

**Dimension (common base):**
```
writeCommonEntityData()
// R2010+:
RC   dimensionVersion
WriteBitExtrusion(extrusion)
2RD  textMidpoint (X, Y)
BD   elevation
RC   flags (various dimension flags)
TV   text (user text override)
BD   rotation
BD   horizontalDirection
3BD  insScale (insertion scale for block rep)
BD   insRotation (insertion rotation)
// R2000+:
BS   attachmentPoint
BS   lineSpacingStyle
BD   lineSpacingFactor
BD   measurement (actual dimension value)
// R2007+:
B    unknown
B    flipArrow1
B    flipArrow2

Handles:
    HandleReference(SoftPointer, dimStyle)
    HandleReference(SoftPointer, blockReference)  // anonymous block
```

Then subtype-specific data for Linear/Aligned/Angular/Radial/Diameter/Ordinate.

**Viewport:**
```
writeCommonEntityData()
3BD  center
BD   width
BD   height
// R2000+:
3BD  viewTarget
3BD  viewDirection
BD   viewTwistAngle
BD   viewHeight
BD   lensLength
BD   frontClipZ
BD   backClipZ
BD   snapAngle
2BD  snapBase
2BD  snapSpacing
2BD  gridSpacing
BS   circleZoomPercent
// R2007+:
BS   gridMajor
BL   frozenLayerCount  
For each: HandleReference(SoftPointer, frozenLayer)
BL   styleSheet (from string)
RC   renderMode
B    ucsFollowMode
// etc. (many more fields)
```

### 6.4 Non-Graphical Object Serialization (`DwgObjectWriter.Objects.cs`)

#### Object Type Dispatch

```csharp
switch (cadObject) {
    case AcdbPlaceHolder:          writeObject(holder); break;
    case BookColor:                writeObject(color); break;
    case CadDictionaryWithDefault: writeObject(dict); break;
    case CadDictionary:            writeObject(dict); break;
    case DictionaryVariable:       writeObject(var); break;
    case GeoData:                  writeObject(geo); break;
    case Group:                    writeObject(group); break;
    case ImageDefinitionReactor:   writeObject(reactor); break;
    case ImageDefinition:          writeObject(def); break;
    case Layout:                   writeObject(layout); break;
    case MLineStyle:               writeObject(style); break;
    case MultiLeaderStyle:         writeObject(style); break;
    case PdfUnderlayDefinition:    writeObject(def); break;
    case PlotSettings:             writeObject(settings); break;
    case RasterVariables:          writeObject(vars); break;
    case Scale:                    writeObject(scale); break;
    case SortEntitiesTable:        writeObject(table); break;
    case SpatialFilter:            writeObject(filter); break;
    case XRecord:                  writeObject(xrec); break;
}
```

#### Selected Object Formats

**CadDictionary:**
```
writeCommonNonEntityData()
BL   numEntries
// R14+:
RC   cloningFlag
RC   hardOwnerFlag
For each entry:
    TV   name (entry key)
For each entry:
    HandleReference(type based on hardOwner flag, entry value)
```

**Layout:**
```
writeCommonNonEntityData()
// Inherits from PlotSettings — writes all PlotSettings data first:
TV   pageName
TV   printerName
TV   paperSize
// ... (extensive plot configuration: margins, offsets, scale, etc.)
// Then Layout-specific:
TV   layoutName
BS   tabOrder
BS   flags
3BD  insBase
2BD  min/max extents (minLimit, maxLimit)
3BD  origin (insertionBasePoint)
3BD  xAxis
3BD  yAxis
BD   elevation
BS   orthoViewType
3BD  min/maxExtents

Handles:
    HandleReference(SoftPointer, viewport)          // active viewport
    HandleReference(SoftPointer, blockRecord)        // associated block
    HandleReference(SoftPointer, lastActiveViewport)
    HandleReference(SoftPointer, baseUCS)
    HandleReference(SoftPointer, namedUCS)
```

**XRecord:**
```
writeCommonNonEntityData()
BL   numDataBytes (total byte size of all records)
For each record:
    // Written as raw DXF group code + value
    RS   groupCode
    [value based on group code range]:
        1-9:   TV (string)
       10-59:  RD (double)
       60-79:  RS (short)
       90-99:  RL (int)
      100-109: TV (string)
      110-119: RD (double)
      ... etc.
RC   cloningFlag
```

### 6.5 Table Entry Writing

Tables are written in `writeTable<T>()` pattern. Key tables:

**Layer:**
```
writeCommonNonEntityData()
TV   name
B    flags64 (has 64-bit flags)
BS   flags
WriteCmColor(color)
Handles:
    HandleReference(SoftPointer, layerControlObject)
    // R2000+:
    HandleReference(SoftPointer, plotStyle)
    // R2007+:
    HandleReference(SoftPointer, material)
    HandleReference(SoftPointer, linetype)
    // R2013+:
    HandleReference(SoftPointer, unknown)
```

**TextStyle:**
```
writeCommonNonEntityData()
TV   name
B    flags64
BS   flags
BD   fixedHeight
BD   widthFactor
BD   obliqueAngle
RC   generation
BD   lastHeight
TV   fontName
TV   bigFontName
Handles:
    HandleReference(SoftPointer, styleControlObject)
```

**LineType:**
```
writeCommonNonEntityData()
TV   name
B    flags64
BS   flags
TV   description
BD   patternLength
RC   alignment (always 'A' = 65)
RC   numDashes
For each dash:
    BD   length
    BS   complexShapeCode
    RD   xOffset
    RD   yOffset
    BD   scale
    BD   rotation
    BS   shapeFlag

Handles:
    HandleReference(SoftPointer, lineTypeControlObject)
    For each dash with shape handle:
        HandleReference(SoftPointer, shapeFile/textStyle)
```

---

## 7. LZ77 Compression

### DwgLZ77AC18Compressor

**File:** `DwgLZ77AC18Compressor.cs` (236 lines)

Used for R2004 (AC1018), R2010 (AC1024), R2013 (AC1027), R2018 (AC1032).

#### Interface

```csharp
interface ICompressor
{
    byte[] Compress(byte[] source, int decompressedSize);
}
```

#### Algorithm Overview

Standard LZ77 with 4-byte hash matching and sliding window.

**Constants:**
- Minimum match length: 3 bytes
- Maximum offset: 0xBFFF (49151)
- Hash table size: 0x8000 (32768)
- Hash is computed from 4 bytes

#### Opcode Encoding

Three match opcode types based on offset and length:

**Type 1 — Short Match (offset ≤ 0x0400, length < 15):**
```
byte1 = ((offset >> 2) - 1) & 0xFF
byte2_bits:
   bits[7:6] = (offset - 1) & 0x03
   bits[5:2] = (length - 2) & 0x0F
Output: byte1, byte2
```

**Type 2 — Medium Match (offset ≤ 0x4000):**
```
byte1_bits:
   bit[7] = 1
   bit[6] = 0
   bits[5:0] = ((length - 2) >> 0) & 0x3F  (if length ≤ 33)
   — OR if length > 33:
   bits[5:0] = 0
   + extra byte: (length - 34) & 0xFF

byte2 = ((offset >> 2) - 1) & 0xFF
byte3_bits:
   bits[7:2] = high bits of offset
   bits[1:0] = (offset - 1) & 0x03
```

**Type 3 — Long Match:**
```
byte1 = 0xFF
byte2 = (length - 3) & 0xFF
byte3 = (offset >> 2) low byte
byte4:
   bits[7:2] = offset high bits
   bits[1:0] = (offset - 1) & 0x03
```

**Literal Run:**
```
If literal_count ≤ 3:
    — Encoded in the previous opcode's low 2 bits
    (handled by copyCompressedBytes)
If literal_count > 3 and ≤ 18:
    byte = (literal_count - 1) & 0x0F  (with high nibble = 0)
If literal_count > 18:
    byte = 0x00
    then: (literal_count - 18) as bytes (with continuation)
    
Followed by: raw literal bytes
```

**Terminator:**
```
0x11, 0x00, 0x00
(Indicates end of compressed data)
```

#### Hash Function

```
For 4 bytes [a, b, c, d]:
    hash = ((a ^ (b << 5) ^ (c << (5+5))) >> 2) & 0x7FFF
```

Each hash bucket stores the most recent position of that hash in the input, forming a chain for match searching.

### DwgLZ77AC21Compressor

**NOT IMPLEMENTED.** Throws `NotImplementedException`. R2007 writing is not supported.

### Compression Integration in File Header Writer

In `DwgFileHeaderWriterAC18.createLocalSection()`:

```
1. Split section data into pages of max 0x7400 bytes
2. For each page:
   a. If compressed: compress = compressor.Compress(pageData, decompressedSize)
   b. Write 32-byte page header (type, section#, sizes, checksum)
   c. Write compressed data
   d. Pad to 0x20 alignment with MagicSequence bytes
```

Compression padding calculation:
```
padBytes = 0x1F - (compressedLength + 0x20 - 1) % 0x20
// pad with MagicSequence bytes starting from dataLength % 256
```

---

## 8. Checksums & CRC

### CRC8 (Section-level)

**Used for:** Header section, Classes section, Object Map chunks, individual object data.

**Algorithm:** Lookup-table-based CRC-16 with seed `0xC0C1`.

```
CRC = seed (0xC0C1)
For each byte b:
    index = b XOR (CRC & 0xFF)
    CRC = (CRC >> 8) XOR CrcTable[index]
Result: CRC (16-bit)
```

The CRC table is the standard CRC-CCITT table (256 entries, 16-bit each). Stored in `CRC.CrcTable`.

**Written as:** RS (raw short, 16 bits) after section data.

### CRC32 / Adler32 (File Header level)

**Used for:** Inner file header checksum in R2004+ files, data section page checksums.

```
sum1 = seed & 0xFFFF
sum2 = seed >> 16
For each byte b in buffer:
    sum1 = (sum1 + b) % 0xFFF1
    sum2 = (sum2 + sum1) % 0xFFF1
Result = (sum2 << 16) | (sum1 & 0xFFFF)
```

**Note:** Despite being called "CRC32" in the codebase, this is actually an **Adler-32** checksum variant.

### MagicSequence Generation

```
seed = 1
For i in 0..256:
    seed = seed * 0x343FD + 0x269EC3   // Linear congruential generator
    MagicSequence[i] = (byte)(seed >> 16)
```

Used for:
- XOR masking of inner file header
- Padding compressed data pages
- Encryption of section data headers

---

## 9. Version Format Differences

### Summary Table

| Feature | R13 (AC1012) | R14 (AC1014) | R2000 (AC1015) | R2004 (AC1018) | R2007 (AC1021) | R2010+ (AC1024+) |
|---------|:---:|:---:|:---:|:---:|:---:|:---:|
| File header size | 0x61 | 0x61 | 0x61 | 0x100 | 0x480 | 0x100 |
| Section storage | Flat | Flat | Flat | Paged+Compressed | Paged+Compressed | Paged+Compressed |
| LZ77 compression | No | No | No | AC18 | AC21 (N/I) | AC18 |
| Text encoding | CP/ANSI | CP/ANSI | CP/ANSI | CP/ANSI | Unicode | Unicode |
| Object streams | Single | Text+Handle | Text+Handle | Main+Text+Handle | Main+Text+Handle | Main+Text+Handle |
| Color format | Index only | Index only | Index only | TrueColor+Transparency | TrueColor+Transparency | TrueColor+Transparency |
| Object type encoding | BS | BS | BS | BS | BS | 2-bit+byte |
| Extrusion/Thickness | Full write | Full write | Compressed (1-bit flag) | Compressed | Compressed | Compressed |
| Handle map CRC | CRC8 | CRC8 | CRC8 | CRC8 | CRC8 | CRC8 |

### Key Version Transition Points

#### R13/R14 → R2000 (AC1015)

- **Extrusion optimization:** `WriteBitExtrusion` — 1 bit for default Z-axis, otherwise full 3BD
- **Thickness optimization:** `WriteBitThickness` — 1 bit for zero, otherwise BD
- **Entity links:** `nolinks` flag controls prev/next entity handle chain
- **Dimension style overrides** move from inline to handle-based
- **New entities:** Support for entity mode 2-bit encoding

#### R2000 → R2004 (AC1018)

**Biggest change: Paged section architecture**
- File header grows from 0x61 to 0x100 bytes
- All sections are compressed with LZ77
- Inner file header with encryption (XOR masks)
- Data pages with 32-byte headers, checksums, section maps
- Page map and section map descriptor pages

**Color encoding:**
- `WriteCmColor`: BS index + BL RGB + RC flag
- `WriteEnColor`: Flags in BS (0x2000=transparency, 0x8000=RGB, 0x4000=book color)

**Object streams split into three:**
- Main, Text, Handle all become separate streams per object (`DwgMergedStreamWriter`)

**Entity data changes:**
- `hasDs` field added (always false)
- `xDictMissing` flag added
- R2000 prev/next entity links removed; explicit ownership handles used instead

#### R2004 → R2007 (AC1021)

**Text encoding → Unicode:**
- `WriteVariableText`: BS length + RS[] chars (16-bit Unicode per char)
- `WriteTextUnicode`: RS byte-length + bytes + RS null terminator
- File header size: 0x480 (much larger)
- LZ77 compression variant: AC21 (NOT IMPLEMENTED in ACadSharp)

**New entity fields:**
- Material handle, shadow flags
- Dimension flip arrows
- Camera, lighting variables in header

#### R2007 → R2010 (AC1024)

**Object type encoding:**
- Pre-R2010: BS (BitShort)
- R2010+: 2-bit+byte encoding:
  - `00` + byte: type < 0x1F0
  - `01` + byte: type = byte + 0x1F0
  - `10` + short: raw 16-bit type

**Handle stream size tracking:**
- After each object's data, a **modular char** encodes the handle stream bit size

**New dimension features:**
- `dimensionVersion` byte
- Various new conditional fields

---

## 10. Constants, Sentinels & Magic Numbers

### Section Sentinels (16 bytes each)

| Section | Start Sentinel | End Sentinel |
|---------|---------------|-------------|
| Header | `CF 7B 1F 23 FD DE 38 A9 5F 7C 68 B8 4E 6D 33 5F` | `30 84 E0 DC 02 21 C7 56 A0 83 97 47 B1 92 CC A0` |
| Classes | `8D A1 C4 B8 C4 A9 F8 C5 C0 DC F4 5F E7 CF B6 8A` | `72 5E 3B 47 3B 56 07 3A 3F 23 0B A0 18 30 49 75` |
| Preview | `1F 25 6D 07 D4 36 28 28 9D 57 CA 3F 9D 44 10 2B` | `E0 DA 92 F8 2B C9 D7 D7 62 A8 35 C0 62 BB EF D4` |

### File Header Magic

| Constant | Value | Usage |
|----------|-------|-------|
| File header sentinel | `0x95 A0 4E 28 99 82 1A E5 5E 41 E0 5F 9D 3A 4D 00` | End of R13-R2000 file header |
| Inner header signature | `"AcFssFcAJMB\0"` | Start of R2004+ inner file header |
| XOR mask base | `0x4164536B` | For `applyMask()` encryption |
| Data page type | `0x4163043B` | Normal data section page |
| Section map type | `0x4163003B` | Section map descriptor page |
| Page map type | `0x41630E3B` | Section page map |
| CRC8 seed | `0xC0C1` | All CRC8 calculations |
| Max page size | `0x7400` | Maximum decompressed page size |
| Object prefix (R2004+) | `0x0DCA` | Written before all objects |
| LZ77 terminator | `0x11 0x00 0x00` | End of compressed stream |

### Section Name Strings

```
"AcDb:Header"
"AcDb:Classes"
"AcDb:Handles"
"AcDb:AcDbObjects"
"AcDb:ObjFreeSpace"
"AcDb:Template"
"AcDb:AuxHeader"
"AcDb:AppInfo"
"AcDb:FileDepList"
"AcDb:SummaryInfo"
"AcDb:Preview"
"AcDb:RevHistory"
```

### Section Locator Numbers (R13-R2000)

| Number | Section |
|--------|---------|
| 0 | Header |
| 1 | Classes |
| 2 | Handles |
| 3 | ObjFreeSpace |
| 4 | Template |
| 5 | AuxHeader |

### DWG Version Bytes

| Version String | Numeric Code | AutoCAD Version |
|---------------|-------------|-----------------|
| `"AC1012"` | 13 | R13 |
| `"AC1014"` | 14 | R14 |
| `"AC1015"` | 15 | R2000 |
| `"AC1018"` | 21 | R2004 |
| `"AC1021"` | 24 | R2007 |
| `"AC1024"` | 27 | R2010 |
| `"AC1027"` | 30 | R2013 |
| `"AC1032"` | 33 | R2018 |

---

## Appendix A: Type Abbreviation Reference

| Abbrev | Full Name | Bits | Description |
|--------|-----------|------|-------------|
| B | Bit | 1 | Boolean bit |
| 2B | 2-Bit code | 2 | Value 0-3 |
| 3B | 3-Bit code | 3 | Value 0-7 |
| BS | BitShort | 2+0/8/16 | Compact short encoding |
| BL | BitLong | 2+0/8/32 | Compact int encoding |
| BLL | BitLongLong | 3+N×8 | Compact long encoding |
| BD | BitDouble | 2+0/64 | Compact double encoding |
| DD | BitDoubleWithDefault | 2+0/32/48/64 | Delta from default |
| RC | Raw Char | 8 | Unsigned byte |
| RS | Raw Short | 16 | 16-bit value |
| RL | Raw Long | 32 | 32-bit value |
| RD | Raw Double | 64 | 64-bit IEEE double |
| TV | Text Variable | BS+N×8 or BS+N×16 | Version-dependent text |
| H | Handle | 8+N×8 | Handle reference |
| 2RD | 2 Raw Doubles | 128 | X,Y point |
| 3BD | 3 BitDoubles | 6-198 | X,Y,Z point |
| 2BD | 2 BitDoubles | 4-132 | X,Y point (compact) |
| 2DD | 2 DD | 4-132 | X,Y with defaults |
| CMC | CmColor | varies | Color (version-specific) |

---

## Appendix B: Recommended Rust Implementation Strategy

### Core Traits

```rust
trait DwgStreamWriter {
    fn write_bit(&mut self, value: bool);
    fn write_2bits(&mut self, value: u8);
    fn write_bit_short(&mut self, value: i16);
    fn write_bit_long(&mut self, value: i32);
    fn write_bit_long_long(&mut self, value: i64);
    fn write_bit_double(&mut self, value: f64);
    fn write_bit_double_with_default(&mut self, value: f64, default: f64);
    fn write_raw_char(&mut self, value: u8);
    fn write_raw_short(&mut self, value: i16);
    fn write_raw_long(&mut self, value: i32);
    fn write_raw_double(&mut self, value: f64);
    fn write_bytes(&mut self, data: &[u8]);
    fn write_variable_text(&mut self, value: &str);
    fn write_text_unicode(&mut self, value: &str);
    fn write_handle_reference(&mut self, ref_type: DwgReferenceType, handle: u64);
    fn write_bit_thickness(&mut self, value: f64);
    fn write_bit_extrusion(&mut self, x: f64, y: f64, z: f64);
    fn write_cm_color(&mut self, color: &Color);
    fn write_en_color(&mut self, color: &Color, transparency: &Transparency);
    fn write_object_type(&mut self, obj_type: ObjectType);
    fn save_position_for_size(&mut self) -> u64;
    fn set_position_by_flag(&mut self, pos: u64);
    fn position_in_bits(&self) -> u64;
}
```

### Version-Specific Behavior via Enum + Match

Rather than deep inheritance, use enum dispatch:

```rust
enum DwgVersion {
    R13, R14, R2000, R2004, R2007, R2010, R2013, R2018
}

// Version-specific behavior in match arms rather than virtual dispatch
fn write_bit_extrusion(writer: &mut BitWriter, version: DwgVersion, xyz: XYZ) {
    match version {
        DwgVersion::R13 | DwgVersion::R14 => {
            writer.write_bit_double(xyz.x);
            writer.write_bit_double(xyz.y);
            writer.write_bit_double(xyz.z);
        }
        _ => {
            // R2000+ compressed form
            if xyz == XYZ::AXIS_Z {
                writer.write_bit(true);
            } else {
                writer.write_bit(false);
                writer.write_bit_double(xyz.x);
                writer.write_bit_double(xyz.y);
                writer.write_bit_double(xyz.z);
            }
        }
    }
}
```

### Suggested Module Structure

```
dwg/
├── mod.rs                    // DwgWriter entry point
├── bit_writer.rs             // BitWriter struct (replaces DwgStreamWriterBase)
├── merged_writer.rs          // MergedWriter (main+text+handle streams)
├── version.rs                // Version enum + utilities
├── crc.rs                    // CRC8 + Adler32
├── compress/
│   ├── mod.rs
│   └── lz77_ac18.rs          // LZ77 compressor
├── file_header/
│   ├── mod.rs
│   ├── ac15.rs               // R13-R2000 file header
│   └── ac18.rs               // R2004+ file header
├── sections/
│   ├── header.rs             // Header variables
│   ├── classes.rs            // DXF classes
│   ├── handles.rs            // Object map
│   ├── app_info.rs
│   ├── aux_header.rs
│   └── preview.rs
└── objects/
    ├── mod.rs                // Object writer + dispatch
    ├── common.rs             // Common entity/object data
    ├── entities.rs           // Entity serialization
    └── objects.rs            // Non-graphical objects
```

---

*Document generated from analysis of ACadSharp commit at `ACadSharp/src/ACadSharp/IO/DWG/`*
