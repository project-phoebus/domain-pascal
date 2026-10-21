{ Golden: record copies with a device side (ref 3.15.1-3.15.3, 3.15.7;
  codegen-patterns 5.5, 5.2; bead domain-pascal-e07e).  The sau5 WIN_
  driver's RESTORE (0xE199EC) and SEEK (0xE19AE6) save the `[device]'
  window's first 12-byte descriptor in a local, set its drive (and
  cylinder), give the command, and put the descriptor back, each copy
  the ordinary 12-byte block copy (5.2): `lea (0x80,A3),A1; lea
  (-0x10,A6),A4; move.l (A1)+,(A4)+' x3 (0xE19A1E..0xE19A2A, SEEK
  0xE19B14..0xE19B20), and `lea (-0x10,A6),A4; lea (0x80,A3),A0; move.l
  (A4)+,(A0)+' x3 (0xE19A50..0xE19A5C, 0xE19B50..0xE19B5C).  The types
  are ../source's win/win.ins.pas (reconstructed).  Here the window is a
  variable of the unit, so its address is `lea (d,A5)' (5.2, "Block
  addresses"); the image reaches it through a pointer local held in A3
  (12.5), which dpc does not hold in a register (bead domain-pascal-dsr5
  refuses the block form through it under --strict).
  - seek: the save, the field stores (`move.w', `move.b #1'), the call,
    the restore;
  - swap: the descriptors exchanged through a plain local, a copy
    between two device sides, and a 2-byte field copied out as one
    `move.w' (win_$ecc_correct 0xE19898 `move.w (0x28,A0),(-0xe,A6)',
    there declared an array of `[byte]' elements, which dpc copies byte
    by byte: bead domain-pascal-4lf1). }
module device_copies;
type
  byte_t = [byte] 0..255;
  pinteger = 0..65535;
  desc_t = record
    cyl: pinteger;
    head: byte_t;
    sector: byte_t;
    phys: integer32;
    page: pinteger;
    drive: pinteger;
  end;
  pattern_t = record
    w: pinteger;
  end;
  regs_t = [device] record
    go: byte_t;
    command: byte_t;
    status: pinteger;
    count: byte_t;
    error: byte_t;
    pad_06: array[1..17] of integer;
    ecc_pattern: pattern_t;
    pad_2a: array[1..43] of integer;
    desc: array[1..50] of desc_t;
  end;

var
  window: regs_t;
  last: pattern_t;

procedure start(in ctlr: pinteger); extern;

procedure seek(in ctlr: pinteger; in dev_unit: pinteger; in cyl: pinteger);
var
  saved_desc: desc_t;
begin
  saved_desc := window.desc[1];
  window.desc[1].drive := dev_unit;
  window.desc[1].cyl := cyl;
  window.count := 1;
  start(ctlr);
  window.desc[1] := saved_desc;
end;

procedure swap;
var
  t: desc_t;
begin
  t := window.desc[1];
  window.desc[1] := window.desc[2];
  window.desc[2] := t;
  last := window.ecc_pattern;
end;
