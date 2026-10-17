{ Golden: a field or element of a type transfer of a variable (ref "Type
  Transfer Functions", p. 4-197: "the type transfer function does not
  change any bits in the argument.  Domain Pascal just 'sees' the
  argument as a value of the new type"; codegen-patterns 5.2, "Fields of
  a type-transferred variable"; bead domain-pascal-w0rj).  The field is
  read in place, in the variable's own storage, with no copy:
  - PROC1_$CREATE_P 0xE15156 `move.w (0xe,A6),D2' and 0xE15162 `move.w
    (0xc,A6),-(SP)': both words of its longword `val_param' parameter,
    which is never loaded whole (tf_$create);
  - FILE_$SET_ACL 0xE5E08A `and.w (-0x44,A6),D0': the third word of the
    callee's copy of a UID value parameter (tf_$word);
  - AREA_$COPY 0xE092DE `move.w (-0xa,A6),-(SP)': the low word of an
    integer32 local passed by value to the module-local 0xE09A6A
    (tf_$word's argument). }
module tf;
type
  args_t = record
    stack_size: integer;
    proc_type: integer;
  end;
  uid_$t = record
    high, low: integer32;
  end;
  view_t = record
    case integer of
      1: (uid: uid_$t);
      2: (w0, w1: integer;
          w2: 0..65535;
          w3: integer);
      3: (b0, b1: char);
  end;
  handle_t = record
    seq: integer;
    area_id: integer;
  end;
  words_t = array[1..4] of integer;
var
  g: integer;
  c: char;
  tbl: array[1..8] of uid_$t;

function tf_$alloc(size: integer; out st: integer32): integer32; val_param; extern;
procedure tf_$use(t: integer; s: integer32); val_param; extern;
procedure tf_$take(w: integer); val_param; extern;

procedure tf_$create(proc: univ_ptr; proc_type: integer32;
                     out status: integer32); val_param;
var
  stack: integer32;
  t: integer;
begin
  t := args_t(proc_type).proc_type;
  stack := tf_$alloc(args_t(proc_type).stack_size, status);
  case t of
    3, 4, 5: g := 5;
    8: g := 6;
    otherwise g := 0;
  end;
  tf_$use(t, stack);
end;

procedure tf_$word(u: uid_$t; var r: uid_$t; i: integer);
var
  h: integer32;
begin
  g := view_t(u).w2;
  c := view_t(r).b1;
  h := 16#10002;
  tf_$take(handle_t(h).area_id);
  g := words_t(tbl[i])[3];
end;
