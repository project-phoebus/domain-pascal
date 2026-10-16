{ codegen-patterns 11 ("Initial values", addresses); bead
  domain-pascal-i5mf, owner's ruling (docs/reference-clarifications.md
  "A variable's own address in its initializer"): a variable is in scope
  in its own initial value, so `addr(itself)' is a relocated long to its
  own place.  ../source domain_os/network/network.pas declares
  network_$rqst_done_ec, an eventcount whose waiter list points at
  itself: `[value := 0, waiter_list_head := addr(network_$rqst_done_ec),
  waiter_list_tail := addr(network_$rqst_done_ec)]'; the image's NETWORK
  D segment 0xE248FC holds at +0x258 (0xE24B54) `0000 0000 00e2 4b54
  00e2 4b54', and network_$rqst_quit_ec at +0x268 the same with
  00e2 4b64.  Here: a unit-level variable without a clause, a `static'
  and a `define'd one, a routine's static, and a variable of an overlay
  section (ref 3.14), each pointing at itself. }
module selfa;
type
  ec_t = record
    value: integer32;
    waiter_list_head: univ_ptr;
    waiter_list_tail: univ_ptr;
  end;
var
  first: integer32;
  done_ec: ec_t := [value := 0, waiter_list_head := addr(done_ec),
                    waiter_list_tail := addr(done_ec)];
  pad: integer32;
  quit_ec: static ec_t := [value := 0, waiter_list_head := addr(quit_ec),
                           waiter_list_tail := addr(quit_ec)];
  glob_ec: define ec_t := [value := 1, waiter_list_head := addr(glob_ec.value),
                           waiter_list_tail := addr(glob_ec.waiter_list_tail)];
  self_p: univ_ptr := addr(self_p);
var (selfa_ov)
  ov_first: integer32;
  ov_ec: ec_t := [value := 0, waiter_list_head := addr(ov_ec),
                  waiter_list_tail := addr(ov_ec)];

procedure selfa_$touch;
var
  inner_ec: static ec_t := [value := 0, waiter_list_head := addr(inner_ec),
                            waiter_list_tail := addr(inner_ec)];
begin
  first := 0;
end;
