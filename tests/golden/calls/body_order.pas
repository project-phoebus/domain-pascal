{ Golden: codegen-patterns 6 ("Order") and 1.6.
  - routines are emitted in the order of their bodies: a routine declared
    `forward' (ref 5.5.2 p. 5-16) or `extern' and defined later (ref 7.3.4
    p. 7-11, Method 2: `define', the `extern' heading, a bare re-heading)
    is emitted where its body is, so calls to it are forward `bsr's
    (AUDIT_$SHUTDOWN 0xE70D2E `bsr.s' to the module-local 0xE70D38;
    PROC2_$AWAKEN_GUARDIAN 0xE3E9AC `bsr.w' to the module-local
    0xE3EB8C);
  - a routine left `extern' is not emitted and is called with `jsr'. }
module body_order;
var
  x: extern integer;
define
  body_$late;
procedure body_$late; extern;
procedure body_$elsewhere; extern;
procedure worker; forward; internal;
procedure body_$first;
begin
  worker;
  body_$late;
  body_$elsewhere;
end;
procedure body_$late;
begin
  x := 1;
end;
procedure worker;
begin
  x := 2;
end;
