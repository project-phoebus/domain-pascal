{ codegen-patterns 14 ("Odd addresses", layout); bead domain-pascal-meel.
  Owner ruling on reference-clarifications item 51: with `-cpu mathchip'
  an explicit `aligned(0)' field may cross two word boundaries, in a
  packed record starting at the next free byte; the MC68010 keeps the
  reference's rule (3.10.5.5 p. 3-26; 3.15.6.3 p. 3-72).  SAU9
  io_$match_device 0x3C439032 `move.l (3,A1),D3; cmp.l (3,A0),D3': bytes
  3..6 of two 8-byte PROM device records reached through by-address
  parameters; SCSI_WAIT 0x3C4778F2 `move.l (35,A5),(-116,A6)' at
  SCSI_$CB + 0x13.
  - an 8-byte packed record with the longword at byte 3 (../source
    reproducer g231), compared whole;
  - the same unpacked, copied to a local and used as a number. }
module crossing_field;
type
  dev_t = packed record
    flags: 0..255;
    unit: 0..255;
    ctype: 0..255;
    id: [aligned(0)] integer32;
    pad: char;
  end;
  rec_t = record
    a, b, c: char;
    id: [aligned(0)] integer32;
    z: char;
  end;
var
  n: integer32;
  cb: extern ^rec_t;

procedure p(var a, b: dev_t);
begin
  if a.id = b.id then
    n := sizeof(dev_t);
end;

procedure q;
var
  t: integer32;
begin
  t := cb^.id;
  n := n - t;
end;
