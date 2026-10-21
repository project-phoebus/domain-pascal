{ The other unit of packed_bool_unreduced.pas: a packed record that
  packed_bool_unreduced.pas reaches as another unit's object (absolute
  operands, bead domain-pascal-6zdt). }
module pbu_data;

type
  hdr_t = packed record
    top: boolean;
    b6: boolean;
    pad0: 0..63;
    flags: 0..255;
    in_use: boolean;
    mode: 0..3;
    pad2: 0..31;
    pad3: 0..63;
    read_only: boolean;
    b0: boolean;
  end;
  wrap_t = record
    lo: integer;
    hdr: hdr_t;
    hi: integer;
  end;
var
  pbu_$wrap: wrap_t;
define pbu_$wrap;
