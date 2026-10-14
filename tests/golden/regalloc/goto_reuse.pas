{ Golden: codegen-patterns 7.5 and 12.4 R7 / 12.5 (bead
  domain-pascal-o4q): a label a `goto' reaches is a join for the
  analyses that reuse a value held from earlier code.
  - fw_*: the spin-lock token stored with `move.w D0,(-2,A6)' and pushed
    from D0 (12.5, "Stored locals pushed from the register"; PKT_$NEXT_ID
    0xE124A8 / 0xE124C4): reloaded from the slot when a branch from
    before the store reaches the push (fw_fwd) or the push is in a loop
    a later `goto' closes (fw_back, whose back edge calls TICK), from D0
    when the `goto' comes after the store (fw_after);
  - cse_*: the scaled index `w * 4' (12.4 R7, FIM_$ACKNOWLEDGE 0xE0A96C)
    computed again when a later `goto' loops back over a store to `w'
    (cse_back) or a branch from before the first computation reaches
    the second (cse_fwd), reused when the `goto' comes after it and
    skips only a store to another variable (cse_after). }
module goto_reuse;
var
  lock: extern integer32;
  counter: extern integer;
  b: extern boolean;
  w: integer;
  tab: array[0..9] of integer32;
  last, last2: integer32;

function ml_$spin_lock(var l: integer32): integer; extern;
procedure ml_$spin_unlock(var l: integer32; token: integer); options(extern, val_param);
procedure tick; extern;

{ forwarding: a branch from before the store reaches the push }
procedure fw_fwd;
label l;
var token: integer;
begin
  if b then goto l;
  token := ml_$spin_lock(lock);
  counter := 1;
l:
  ml_$spin_unlock(lock, token);
end;

{ forwarding: the push is in a loop a later goto closes }
procedure fw_back;
label l;
var token: integer;
begin
  token := ml_$spin_lock(lock);
l:
  ml_$spin_unlock(lock, token);
  if b then begin
    tick;
    goto l;
  end;
  counter := 2;
end;

{ forwarding with a forward goto after the store: still valid }
procedure fw_after;
label l;
var token: integer;
begin
  token := ml_$spin_lock(lock);
  if b then goto l;
  counter := 1;
l:
  ml_$spin_unlock(lock, token);
end;

{ reuse of a scaled index: a backward goto changes the variable }
procedure cse_back;
label l;
begin
  last := tab[w];
l:
  last2 := tab[w];
  w := w + 1;
  if w < 5 then goto l;
end;

{ reuse: a forward goto from before the computation }
procedure cse_fwd;
label l;
begin
  if b then goto l;
  last := tab[w];
l:
  last2 := tab[w];
end;

{ reuse: a forward goto after the computation, skipping a store to
  another variable: the value is still valid }
procedure cse_after;
label l;
begin
  last := tab[w];
  if b then goto l;
  counter := 3;
l:
  last2 := tab[w];
end;

{ a local held in a register (12.5) across a `goto' that skips its
  assignment, and across a backward one }
procedure rl_skip;
label l;
var x: integer;
begin
  if b then goto l;
  x := counter;
  tick;
l:
  counter := x + 1;
  tick;
  counter := x;
end;

procedure rl_loop;
label l;
var x: integer;
begin
  x := counter;
l:
  tick;
  counter := x;
  if b then goto l;
end;
