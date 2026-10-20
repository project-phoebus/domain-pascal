{ Execution test (bead domain-pascal-g6l7): sets of more than a
  longword in memory (ref 3.9.3, Figure 3-6: member k is bit k mod 8 of the
  byte (bits-1-k) div 8; "Set Operations", pp. 4-173..4-176), of 6, 8, 16
  and 32 bytes (codegen-patterns 5.1, "Sets of more than a longword"):
  constants stored by longwords (MSG_$INIT 0xE31BD4), `[k]', `s + [k]',
  `[k] + t', `s - [k]' with run-time members (`bset'/`bclr' on the byte,
  MSG_$OPENI 0xE59260), `= []' and `<> []' in a condition and as a value
  (`tst.l (An)+' per longword, MSG_$OPENI 0xE5920A), `=' of two sets, `in'
  with a run-time member as a value, copies, `+'/`-' of constant members,
  `+', `*', `-' of two sets into a third, `s := s + t' in place, a
  constructor of constant and run-time members.  Members at both ends and
  at word and longword boundaries.  The kernel's forms: an array of
  records of one 8-byte set by a run-time index and through `with'
  (msg/fork.pas, msg/close.pas, msg/open.pas), a 16-byte frame set
  through a pointer, a `set of char' with a char member.  Also `s := s *
  t' in place, `=' and `<>' against a constant and `<='/`>=' as values;
  a char `in' a constant set (a cell in the code, the guarded test of
  UNMAP_CASE 0xE54166), below, at and above both ends and the cell's
  last bit.  A destination that is an operand under another address (a
  `var' parameter): `d := x + y', `d := (x + y) + [k]', `d := x + [k]',
  `d := x - [k]', `d := [k] + x' with d the same object as y or x; a
  `for' storing a large-set array element.
  The expected output is worked out from the set semantics and the bit
  layout of Figure 3-6, not from dpc.

  call: LARGE_SETS_$S6 w:0 w:47
  call: LARGE_SETS_$S6 w:15 w:16
  call: LARGE_SETS_$S6 w:31 w:32
  call: LARGE_SETS_$S8 w:0 w:63
  call: LARGE_SETS_$S8 w:31 w:32
  call: LARGE_SETS_$S8 w:47 w:48
  call: LARGE_SETS_$S16 w:0 w:127
  call: LARGE_SETS_$S16 w:63 w:64
  call: LARGE_SETS_$S16 w:95 w:96
  call: LARGE_SETS_$S32 w:0 w:255
  call: LARGE_SETS_$S32 w:127 w:128
  call: LARGE_SETS_$S32 w:7 w:8
  call: LARGE_SETS_$TAB1 w:1 w:0 w:63
  call: LARGE_SETS_$TEST w:1
  call: LARGE_SETS_$SHOWB
  call: LARGE_SETS_$TAB2 w:1 w:0 w:63
  call: LARGE_SETS_$TEST w:1
  call: LARGE_SETS_$SHOWB
  call: LARGE_SETS_$TAB3 w:1
  call: LARGE_SETS_$PTR w:0 w:63
  call: LARGE_SETS_$SHOWB
  call: LARGE_SETS_$CHARS w:0 w:63
  call: LARGE_SETS_$TAB1 w:3 w:58 w:5
  call: LARGE_SETS_$TEST w:3
  call: LARGE_SETS_$SHOWB
  call: LARGE_SETS_$TAB2 w:3 w:58 w:5
  call: LARGE_SETS_$TEST w:3
  call: LARGE_SETS_$SHOWB
  call: LARGE_SETS_$TAB3 w:3
  call: LARGE_SETS_$PTR w:58 w:5
  call: LARGE_SETS_$SHOWB
  call: LARGE_SETS_$CHARS w:58 w:5
  call: LARGE_SETS_$TAB1 w:223 w:31 w:32
  call: LARGE_SETS_$TEST w:223
  call: LARGE_SETS_$SHOWB
  call: LARGE_SETS_$TAB2 w:223 w:31 w:32
  call: LARGE_SETS_$TEST w:223
  call: LARGE_SETS_$SHOWB
  call: LARGE_SETS_$TAB3 w:223
  call: LARGE_SETS_$PTR w:31 w:32
  call: LARGE_SETS_$SHOWB
  call: LARGE_SETS_$CHARS w:31 w:32
  call: LARGE_SETS_$WITH w:1 w:0 w:63
  call: LARGE_SETS_$SHOW w:1
  call: LARGE_SETS_$WITH w:3 w:58 w:5
  call: LARGE_SETS_$SHOW w:3
  call: LARGE_SETS_$WITH w:223 w:31 w:32
  call: LARGE_SETS_$SHOW w:223
  call: LARGE_SETS_$CIN b:0
  call: LARGE_SETS_$CIN b:64
  call: LARGE_SETS_$CIN b:65
  call: LARGE_SETS_$CIN b:90
  call: LARGE_SETS_$CIN b:91
  call: LARGE_SETS_$CIN b:95
  call: LARGE_SETS_$CIN b:96
  call: LARGE_SETS_$CIN b:97
  call: LARGE_SETS_$CIN b:102
  call: LARGE_SETS_$CIN b:103
  call: LARGE_SETS_$CIN b:255
  call: LARGE_SETS_$ALIAS w:5
  call: LARGE_SETS_$ALIAS w:63
  call: LARGE_SETS_$ALIAS w:0
  expect:
    mem 00 00 00 00 00 00
    mem 80 01 80 01 80 01
    mem 00 00 00 00 00 01
    mem 80 00 00 00 00 01
    false
    true
    false
    mem 80 00 00 00 00 01
    true
    mem 80 00 00 00 00 00
    false
    mem 00 00 00 00 00 00
    true
    false
    true
    mem 80 00 00 00 00 01
    mem 80 00 00 00 00 00
    mem 80 00 00 00 00 01
    mem 80 00 00 00 00 00
    mem 00 00 00 00 00 00
    mem 80 00 00 00 00 00
    mem 80 00 00 00 00 01
    mem 80 00 00 00 00 01
    mem 00 00 00 00 00 00
    mem 80 00 00 00 00 01
    true
    true
    false
    true
    mem 80 00 00 00 00 00
    false
    false
    true
    mem 00 00 00 00 00 00
    mem 80 01 80 01 80 01
    mem 00 00 00 00 80 00
    mem 00 00 00 01 80 00
    false
    true
    true
    mem 00 00 00 01 80 00
    true
    mem 00 00 00 01 00 00
    false
    mem 00 00 00 00 00 00
    true
    false
    true
    mem 00 00 00 01 80 00
    mem 00 00 00 01 00 00
    mem 80 00 00 01 00 01
    mem 80 00 00 00 00 00
    mem 80 00 00 00 00 00
    mem 00 00 00 00 00 00
    mem 80 00 00 01 80 00
    mem 80 00 00 01 80 00
    mem 80 00 00 00 00 00
    mem 80 00 00 01 80 00
    true
    true
    false
    true
    mem 80 00 00 00 00 00
    false
    false
    false
    mem 00 00 00 00 00 00
    mem 80 01 80 01 80 01
    mem 00 00 80 00 00 00
    mem 00 01 80 00 00 00
    false
    true
    false
    mem 00 01 80 00 00 00
    true
    mem 00 01 00 00 00 00
    false
    mem 00 00 00 00 00 00
    true
    false
    true
    mem 00 01 80 00 00 00
    mem 00 01 00 00 00 00
    mem 80 01 00 00 00 01
    mem 80 01 00 00 00 00
    mem 80 00 00 00 00 00
    mem 00 01 00 00 00 00
    mem 80 01 80 00 00 00
    mem 80 01 80 00 00 00
    mem 80 00 00 00 00 00
    mem 80 01 80 00 00 00
    true
    true
    false
    true
    mem 80 01 00 00 00 00
    false
    true
    true
    mem 00 00 00 00 00 00 00 00
    mem 80 00 00 01 80 01 80 01
    mem 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 01
    false
    true
    false
    mem 80 00 00 00 00 00 00 01
    true
    mem 80 00 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00
    true
    false
    true
    mem 80 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 01
    mem 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 01
    true
    true
    false
    true
    mem 80 00 00 00 00 00 00 00
    false
    false
    true
    mem 00 00 00 00 00 00 00 00
    mem 80 00 00 01 80 01 80 01
    mem 00 00 00 00 80 00 00 00
    mem 00 00 00 01 80 00 00 00
    false
    true
    false
    mem 00 00 00 01 80 00 00 00
    true
    mem 00 00 00 01 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00
    true
    false
    true
    mem 00 00 00 01 80 00 00 00
    mem 00 00 00 01 00 00 00 00
    mem 80 00 00 01 00 00 00 01
    mem 80 00 00 01 00 00 00 00
    mem 80 00 00 00 00 00 00 00
    mem 00 00 00 01 00 00 00 00
    mem 80 00 00 01 80 00 00 00
    mem 80 00 00 01 80 00 00 00
    mem 80 00 00 00 00 00 00 00
    mem 80 00 00 01 80 00 00 00
    true
    true
    false
    true
    mem 80 00 00 01 00 00 00 00
    false
    true
    true
    mem 00 00 00 00 00 00 00 00
    mem 80 00 00 01 80 01 80 01
    mem 00 00 80 00 00 00 00 00
    mem 00 01 80 00 00 00 00 00
    false
    true
    false
    mem 00 01 80 00 00 00 00 00
    true
    mem 00 01 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00
    true
    false
    true
    mem 00 01 80 00 00 00 00 00
    mem 00 01 00 00 00 00 00 00
    mem 80 01 00 00 00 00 00 01
    mem 80 01 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00
    mem 00 01 00 00 00 00 00 00
    mem 80 01 80 00 00 00 00 00
    mem 80 01 80 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00
    mem 80 01 80 00 00 00 00 00
    true
    true
    false
    true
    mem 80 01 00 00 00 00 00 00
    false
    true
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 01 80 01 80 01
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    false
    true
    false
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    true
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    false
    true
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    true
    true
    false
    true
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    false
    false
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 01 80 01 80 01
    mem 00 00 00 00 00 00 00 00 80 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00
    false
    true
    false
    mem 00 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00
    true
    mem 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    false
    true
    mem 00 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 01 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00
    true
    true
    false
    true
    mem 80 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00
    false
    true
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 01 80 01 80 01
    mem 00 00 00 00 80 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00
    false
    true
    false
    mem 00 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00
    true
    mem 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    false
    true
    mem 00 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 01 00 00 00 00 00 00 00 00 00 00 00 01
    mem 80 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00
    true
    true
    false
    true
    mem 80 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00
    false
    true
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80 01 80 01
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    false
    true
    false
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    true
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    false
    true
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    true
    true
    false
    true
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    false
    false
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80 01 80 01
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    false
    true
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    false
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    true
    false
    true
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    false
    true
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80 01 80 01
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 80
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80
    false
    true
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    false
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 80
    true
    true
    false
    true
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00
    false
    true
    true
    mem 00 00 00 00 00 00 00 01
    mem 80 00 00 00 00 00 00 01
    true
    false
    mem 80 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00
    true
    mem 04 00 00 00 00 00 00 00
    mem 04 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 01
    mem 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 02 00 00 00 00 00 00 00 00 00 00 01 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 02 00 00 00 00 00 00 00 00 00 00 00 00
    mem 04 00 00 00 00 00 00 00
    mem 04 00 00 00 00 00 00 20
    true
    false
    mem 00 00 00 00 00 00 00 20
    mem 00 00 00 00 00 00 00 00
    true
    mem 04 00 00 00 00 00 00 00
    mem 04 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00
    mem 04 00 00 00 00 00 00 00 04 00 00 00 00 00 00 00
    mem 04 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 02 00 00 00 04 00 00 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 02 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 00 80 00 00 00
    mem 00 00 00 01 80 00 00 00
    true
    false
    mem 00 00 00 01 00 00 00 00
    mem 00 00 00 00 00 00 00 00
    true
    mem 04 00 00 00 00 00 00 00
    mem 04 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00
    mem 00 00 00 00 80 00 00 00 00 00 00 00 80 00 00 00
    mem 00 00 00 00 80 00 00 00 00 00 00 00 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 02 00 00 00 00 00 00 00 80 00 00 00 00
    false
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 02 00 00 00 00 00 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 00
    true
    mem 00 00 00 00 00 00 00 20
    true
    mem 00 00 00 01 00 00 00 00
    true
    false
    false
    false
    false
    true
    false
    true
    false
    false
    false
    false
    true
    false
    false
    false
    true
    false
    true
    false
    false
    false
    false
    mem 80 00 00 00 00 00 00 06
    mem 80 00 00 00 00 00 00 06
    mem 80 00 00 00 00 00 00 26
    mem 00 00 00 00 00 00 00 22
    mem 00 00 00 00 00 00 00 02
    mem 00 00 00 00 00 00 00 28
    mem 00 00 00 00 00 00 00 20 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 20
    mem 80 00 00 00 00 00 00 06
    mem 80 00 00 00 00 00 00 06
    mem 80 00 00 00 00 00 00 06
    mem 80 00 00 00 00 00 00 02
    mem 00 00 00 00 00 00 00 02
    mem 80 00 00 00 00 00 00 08
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 80 00 00 00 00 00 00 00
    mem 80 00 00 00 00 00 00 06
    mem 80 00 00 00 00 00 00 06
    mem 80 00 00 00 00 00 00 07
    mem 00 00 00 00 00 00 00 03
    mem 00 00 00 00 00 00 00 02
    mem 00 00 00 00 00 00 00 09
    mem 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
}
module large_sets;
%include 'exec.ins.pas';

type
  set6 = set of 0..47;
  set8 = set of 0..63;
  set16 = set of 0..127;
  set32 = set of 0..255;
  owner_t = record
    owners: set8;
  end;
  set16_ptr = ^set16;
var
  s6, t6, u6: set6;
  s8, t8, u8: set8;
  s16, t16, u16: set16;
  s32, t32, u32: set32;
  cs: set of char;
  own: array[1..223] of owner_t;
  bl: boolean;
  arr: array[0..3] of set8;

procedure large_sets_$s6(a: integer; b: integer); options(val_param);
begin
  s6 := [];
  exec_$log_mem(s6, 6);
  s6 := [0, 15, 16, 31, 32, 47];
  exec_$log_mem(s6, 6);
  s6 := [a];
  exec_$log_mem(s6, 6);
  s6 := s6 + [b];
  exec_$log_mem(s6, 6);
  if s6 = [] then bl := true else bl := false;
  exec_$log_bool(bl);
  bl := a in s6;
  exec_$log_bool(bl);
  bl := 16 in s6;
  exec_$log_bool(bl);
  t6 := s6;
  exec_$log_mem(t6, 6);
  if t6 = s6 then bl := true else bl := false;
  exec_$log_bool(bl);
  s6 := s6 - [a];
  exec_$log_mem(s6, 6);
  if t6 = s6 then bl := true else bl := false;
  exec_$log_bool(bl);
  s6 := s6 - [b];
  exec_$log_mem(s6, 6);
  if s6 = [] then bl := true else bl := false;
  exec_$log_bool(bl);
  bl := s6 <> [];
  exec_$log_bool(bl);
  if t6 <> [] then bl := true else bl := false;
  exec_$log_bool(bl);
  s6 := [b] + t6;
  exec_$log_mem(s6, 6);
  s6 := t6 - [a];
  exec_$log_mem(s6, 6);
  s6 := s6 + [0, 47];
  exec_$log_mem(s6, 6);
  s6 := s6 - [0, 16];
  exec_$log_mem(s6, 6);
  u6 := s6 - t6;
  exec_$log_mem(u6, 6);
  u6 := s6 * t6;
  exec_$log_mem(u6, 6);
  u6 := s6 + t6;
  exec_$log_mem(u6, 6);
  t6 := t6 + s6;
  exec_$log_mem(t6, 6);
  t6 := t6 - [a, b];
  exec_$log_mem(t6, 6);
  u6 := [a, 47, b];
  exec_$log_mem(u6, 6);
  bl := t6 <= s6;
  exec_$log_bool(bl);
  bl := s6 >= t6;
  exec_$log_bool(bl);
  bl := u6 <= s6;
  exec_$log_bool(bl);
  bl := s6 >= [47];
  exec_$log_bool(bl);
  s6 := s6 * u6;
  exec_$log_mem(s6, 6);
  bl := s6 = [0, 47];
  exec_$log_bool(bl);
  bl := s6 <> [47];
  exec_$log_bool(bl);
  u6 := [b, 47];
  bl := s6 = u6;
  exec_$log_bool(bl);
end;

procedure large_sets_$s8(a: integer; b: integer); options(val_param);
begin
  s8 := [];
  exec_$log_mem(s8, 8);
  s8 := [0, 15, 16, 31, 32, 63];
  exec_$log_mem(s8, 8);
  s8 := [a];
  exec_$log_mem(s8, 8);
  s8 := s8 + [b];
  exec_$log_mem(s8, 8);
  if s8 = [] then bl := true else bl := false;
  exec_$log_bool(bl);
  bl := a in s8;
  exec_$log_bool(bl);
  bl := 16 in s8;
  exec_$log_bool(bl);
  t8 := s8;
  exec_$log_mem(t8, 8);
  if t8 = s8 then bl := true else bl := false;
  exec_$log_bool(bl);
  s8 := s8 - [a];
  exec_$log_mem(s8, 8);
  if t8 = s8 then bl := true else bl := false;
  exec_$log_bool(bl);
  s8 := s8 - [b];
  exec_$log_mem(s8, 8);
  if s8 = [] then bl := true else bl := false;
  exec_$log_bool(bl);
  bl := s8 <> [];
  exec_$log_bool(bl);
  if t8 <> [] then bl := true else bl := false;
  exec_$log_bool(bl);
  s8 := [b] + t8;
  exec_$log_mem(s8, 8);
  s8 := t8 - [a];
  exec_$log_mem(s8, 8);
  s8 := s8 + [0, 63];
  exec_$log_mem(s8, 8);
  s8 := s8 - [0, 16];
  exec_$log_mem(s8, 8);
  u8 := s8 - t8;
  exec_$log_mem(u8, 8);
  u8 := s8 * t8;
  exec_$log_mem(u8, 8);
  u8 := s8 + t8;
  exec_$log_mem(u8, 8);
  t8 := t8 + s8;
  exec_$log_mem(t8, 8);
  t8 := t8 - [a, b];
  exec_$log_mem(t8, 8);
  u8 := [a, 63, b];
  exec_$log_mem(u8, 8);
  bl := t8 <= s8;
  exec_$log_bool(bl);
  bl := s8 >= t8;
  exec_$log_bool(bl);
  bl := u8 <= s8;
  exec_$log_bool(bl);
  bl := s8 >= [63];
  exec_$log_bool(bl);
  s8 := s8 * u8;
  exec_$log_mem(s8, 8);
  bl := s8 = [0, 63];
  exec_$log_bool(bl);
  bl := s8 <> [63];
  exec_$log_bool(bl);
  u8 := [b, 63];
  bl := s8 = u8;
  exec_$log_bool(bl);
end;

procedure large_sets_$s16(a: integer; b: integer); options(val_param);
begin
  s16 := [];
  exec_$log_mem(s16, 16);
  s16 := [0, 15, 16, 31, 32, 127];
  exec_$log_mem(s16, 16);
  s16 := [a];
  exec_$log_mem(s16, 16);
  s16 := s16 + [b];
  exec_$log_mem(s16, 16);
  if s16 = [] then bl := true else bl := false;
  exec_$log_bool(bl);
  bl := a in s16;
  exec_$log_bool(bl);
  bl := 16 in s16;
  exec_$log_bool(bl);
  t16 := s16;
  exec_$log_mem(t16, 16);
  if t16 = s16 then bl := true else bl := false;
  exec_$log_bool(bl);
  s16 := s16 - [a];
  exec_$log_mem(s16, 16);
  if t16 = s16 then bl := true else bl := false;
  exec_$log_bool(bl);
  s16 := s16 - [b];
  exec_$log_mem(s16, 16);
  if s16 = [] then bl := true else bl := false;
  exec_$log_bool(bl);
  bl := s16 <> [];
  exec_$log_bool(bl);
  if t16 <> [] then bl := true else bl := false;
  exec_$log_bool(bl);
  s16 := [b] + t16;
  exec_$log_mem(s16, 16);
  s16 := t16 - [a];
  exec_$log_mem(s16, 16);
  s16 := s16 + [0, 127];
  exec_$log_mem(s16, 16);
  s16 := s16 - [0, 16];
  exec_$log_mem(s16, 16);
  u16 := s16 - t16;
  exec_$log_mem(u16, 16);
  u16 := s16 * t16;
  exec_$log_mem(u16, 16);
  u16 := s16 + t16;
  exec_$log_mem(u16, 16);
  t16 := t16 + s16;
  exec_$log_mem(t16, 16);
  t16 := t16 - [a, b];
  exec_$log_mem(t16, 16);
  u16 := [a, 127, b];
  exec_$log_mem(u16, 16);
  bl := t16 <= s16;
  exec_$log_bool(bl);
  bl := s16 >= t16;
  exec_$log_bool(bl);
  bl := u16 <= s16;
  exec_$log_bool(bl);
  bl := s16 >= [127];
  exec_$log_bool(bl);
  s16 := s16 * u16;
  exec_$log_mem(s16, 16);
  bl := s16 = [0, 127];
  exec_$log_bool(bl);
  bl := s16 <> [127];
  exec_$log_bool(bl);
  u16 := [b, 127];
  bl := s16 = u16;
  exec_$log_bool(bl);
end;

procedure large_sets_$s32(a: integer; b: integer); options(val_param);
begin
  s32 := [];
  exec_$log_mem(s32, 32);
  s32 := [0, 15, 16, 31, 32, 255];
  exec_$log_mem(s32, 32);
  s32 := [a];
  exec_$log_mem(s32, 32);
  s32 := s32 + [b];
  exec_$log_mem(s32, 32);
  if s32 = [] then bl := true else bl := false;
  exec_$log_bool(bl);
  bl := a in s32;
  exec_$log_bool(bl);
  bl := 16 in s32;
  exec_$log_bool(bl);
  t32 := s32;
  exec_$log_mem(t32, 32);
  if t32 = s32 then bl := true else bl := false;
  exec_$log_bool(bl);
  s32 := s32 - [a];
  exec_$log_mem(s32, 32);
  if t32 = s32 then bl := true else bl := false;
  exec_$log_bool(bl);
  s32 := s32 - [b];
  exec_$log_mem(s32, 32);
  if s32 = [] then bl := true else bl := false;
  exec_$log_bool(bl);
  bl := s32 <> [];
  exec_$log_bool(bl);
  if t32 <> [] then bl := true else bl := false;
  exec_$log_bool(bl);
  s32 := [b] + t32;
  exec_$log_mem(s32, 32);
  s32 := t32 - [a];
  exec_$log_mem(s32, 32);
  s32 := s32 + [0, 255];
  exec_$log_mem(s32, 32);
  s32 := s32 - [0, 16];
  exec_$log_mem(s32, 32);
  u32 := s32 - t32;
  exec_$log_mem(u32, 32);
  u32 := s32 * t32;
  exec_$log_mem(u32, 32);
  u32 := s32 + t32;
  exec_$log_mem(u32, 32);
  t32 := t32 + s32;
  exec_$log_mem(t32, 32);
  t32 := t32 - [a, b];
  exec_$log_mem(t32, 32);
  u32 := [a, 255, b];
  exec_$log_mem(u32, 32);
  bl := t32 <= s32;
  exec_$log_bool(bl);
  bl := s32 >= t32;
  exec_$log_bool(bl);
  bl := u32 <= s32;
  exec_$log_bool(bl);
  bl := s32 >= [255];
  exec_$log_bool(bl);
  s32 := s32 * u32;
  exec_$log_mem(s32, 32);
  bl := s32 = [0, 255];
  exec_$log_bool(bl);
  bl := s32 <> [255];
  exec_$log_bool(bl);
  u32 := [b, 255];
  bl := s32 = u32;
  exec_$log_bool(bl);
end;

procedure large_sets_$tab1(i: integer; a: integer; b: integer); options(val_param);
begin
  own[i].owners := [a];
  exec_$log_mem(own[i].owners, 8);
  own[i].owners := own[i].owners + [b];
  exec_$log_mem(own[i].owners, 8);
  bl := b in own[i].owners;
  exec_$log_bool(bl);
end;

procedure large_sets_$test(i: integer); options(val_param);
begin
  bl := own[i].owners = [];
end;

procedure large_sets_$showb;
begin
  exec_$log_bool(bl);
end;

procedure large_sets_$tab2(i: integer; a: integer; b: integer); options(val_param);
begin
  own[i].owners := own[i].owners - [a];
  exec_$log_mem(own[i].owners, 8);
  own[i].owners := [b] + own[i].owners;
  own[i].owners := own[i].owners - [b];
  exec_$log_mem(own[i].owners, 8);
end;

procedure large_sets_$tab3(i: integer); options(val_param);
begin
  own[i].owners := [58];
  exec_$log_mem(own[i].owners, 8);
  exec_$log_mem(own[1].owners, 8);
  exec_$log_mem(own[2].owners, 8);
  exec_$log_mem(own[222].owners, 8);
end;

procedure large_sets_$ptr(a: integer; b: integer); options(val_param);
var
  l: set16;
  p: set16_ptr;
begin
  p := exec_$id_p(addr(l));
  p^ := [a];
  p^ := p^ + [127 - b];
  exec_$log_mem(l, 16);
  p^ := p^ - [a];
  exec_$log_mem(l, 16);
  bl := p^ = [];
end;

procedure large_sets_$chars(a: integer; b: integer); options(val_param);
var
  c: char;
begin
  c := chr(a + 8);
  cs := ['a'];
  cs := cs + [c];
  exec_$log_mem(cs, 32);
  c := chr(b + 8);
  bl := c in cs;
  exec_$log_bool(bl);
  c := chr(a + 8);
  cs := cs - [c];
  exec_$log_mem(cs, 32);
end;

procedure large_sets_$with(i: integer; a: integer; b: integer); options(val_param);
begin
  with own[i] do begin
    owners := [a];
    owners := owners + [b];
    bl := owners <> [];
    if a in owners then
      owners := owners - [a];
  end;
end;

procedure large_sets_$cin(c: char); options(val_param);
begin
  bl := c in ['A'..'Z'];
  exec_$log_bool(bl);
  if c in ['a'..'f', '_'] then bl := true else bl := false;
  exec_$log_bool(bl);
end;

procedure ls_plus(var d: set8; var x: set8; var y: set8); internal;
begin
  d := x + y;
end;

procedure ls_plusk(var d: set8; var x: set8; var y: set8; k: integer); internal;
begin
  d := (x + y) + [k];
end;

procedure ls_addk(var d: set8; var x: set8; k: integer); internal;
begin
  d := x + [k];
end;

procedure ls_subk(var d: set8; var x: set8; k: integer); internal;
begin
  d := x - [k];
end;

procedure ls_kplus(var d: set8; var x: set8; k: integer); internal;
begin
  d := [k] + x;
end;

procedure ls_fill(k: integer); internal;
var
  i: integer;
begin
  for i := 0 to 3 do
    arr[i] := [k];
end;

procedure ls_clear;
var
  i: integer;
begin
  for i := 1 to 2 do
    arr[i] := [];
end;

procedure large_sets_$alias(k: integer); options(val_param);
begin
  s8 := [1];
  t8 := [2, 63];
  ls_plus(t8, s8, t8);
  exec_$log_mem(t8, 8);
  s8 := [1];
  t8 := [2, 63];
  ls_plus(s8, t8, s8);
  exec_$log_mem(s8, 8);
  s8 := [1];
  t8 := [2, 63];
  ls_plusk(t8, s8, t8, k);
  exec_$log_mem(t8, 8);
  s8 := [1];
  ls_addk(s8, s8, k);
  exec_$log_mem(s8, 8);
  s8 := [1, k];
  ls_subk(s8, s8, k);
  exec_$log_mem(s8, 8);
  t8 := [3];
  ls_kplus(t8, t8, k);
  exec_$log_mem(t8, 8);
  ls_fill(k);
  ls_clear;
  exec_$log_mem(arr, 32);
end;

procedure large_sets_$show(i: integer); options(val_param);
begin
  exec_$log_mem(own[i].owners, 8);
  exec_$log_bool(bl);
end;
