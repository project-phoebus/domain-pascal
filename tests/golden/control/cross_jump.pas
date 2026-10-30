{ Cross-jumping (codegen-patterns 7.1 "Identical ends of the two
  parts", 1.1 "Shared pop at a join"; bead domain-pascal-k1da).  The
  identical tails of the arms reaching one join are merged: the arm
  falling through to the join keeps its copy and the other's suffix
  becomes a `bra' into it (RIP_$SEND_UPDATES 0xE688B8 `st -(SP); bra.s
  0xE688BE' into the other part's `jsr'; the shared pop, 0xE06156); with
  no arm falling through, MC68010 code keeps the first arm's copy and
  later arms branch back (TERM_$CONTROL 0xE66C24 `moveq #2,D0; bra.s
  0xE66C10'), MC68020 code the last's (golden m68020/cross_jump).  An
  arm that is nothing but the tail goes, its branches retargeted (LOG_$ADD
  0xE176C8 `beq.s 0xE176D2'). }
module cross_jump;
type
  rec_t = record a: integer32; b: integer; end;
  rec_ptr_t = ^rec_t;
var
  count: integer32;
procedure log_it(in n: integer); extern;
procedure p_$tails(in r: rec_ptr_t; in sel: boolean);
begin
  if sel then begin
    log_it(1);
    count := count + 1;
  end
  else begin
    log_it(2);
    count := count + 1;
  end;
  log_it(3);
end;
function p_$arms(in r: rec_ptr_t; in k: integer): integer;
begin
  case k of
    1: p_$arms := 10;
    2: begin r^.b := 5; p_$arms := 20; end;
    3: begin r^.b := 5; p_$arms := 20; end;
    otherwise p_$arms := 0;
  end;
end;
