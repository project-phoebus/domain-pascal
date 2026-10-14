{ Golden: codegen-patterns 11 (fragments of several code sections), 1.6
  (calls of module routines outside the unit), 7 (fragment pad, module-end
  padding).
  - A unit with routines in two code sections, compiled as a fragment of
    each (`dpc --fragment-origin=SECTION=0xADDR', .args): each section has
    its own origin symbol `__dpc_fragment_origin<output section>', its own
    `.space' pad (WIRED_PROC at 2 mod 4) and its own module-end padding
    (only .text ends its module's segment, `--fragment-ends-module=.text').
  - Routines of the module outside the unit (`--module-routine=
    SECTION:NAME=0xADDR') are called with `bsr' from their own code section
    and with `jsr abs.l' from another, as the whole module calls them (TTY:
    TTY_$K_RESET 0xE6730E `jsr' to the WIRED_PROC module-local 0xE1AED0,
    the WIRED_PROC routines' `bsr's to it).  SFRAG_$LOCK at 0xFF0: `bsr.s'
    from 0x1006 (written as its encoding, `.short 0x61e8'), `jsr' from
    .text.  SFRAG_$GET_DESC at 0x1F000: `bsr.w' from .text, `jsr' from
    WIRED_PROC.  SFRAG_$INIT in OS_INIT_PROC, a section the unit has no
    routine in: `jsr' from both.
  - A call between the unit's own sections is `jsr abs.l' (sections/
    named_sections). }
module sfrag;

procedure sfrag_$lock; internal; extern;          { WIRED_PROC }
procedure sfrag_$get_desc; extern;                { .text }
procedure sfrag_$init; extern;                    { OS_INIT_PROC }

[section(wired_proc, wired_data)]
procedure sfrag_$put;
begin
  sfrag_$lock;
  sfrag_$get_desc;
  sfrag_$init;
end;

procedure sfrag_$reset;
begin
  sfrag_$lock;
  sfrag_$get_desc;
  sfrag_$init;
  sfrag_$put;
end;
