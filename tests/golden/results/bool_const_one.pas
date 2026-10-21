{ codegen-patterns 3 ("Boolean constants"); bead domain-pascal-u9vq.
  A boolean function whose whole body is one constant assignment:

  - yes: `f := true', `st D0' (DUMMY_INTERRUPT_HANDLER 0xE6D46C,
    `link #-4; st D0; unlk; rts'; the same body in SAU5);
  - no: `f := false', `clr.b D0' (IIC_$EXISTS 0xE70AB0, `link #-4;
    clr.b D0; unlk; rts').

  No byte function of the SAU2 image sets D0 with `moveq #-1' or
  `move.b #-1'. }
module bool_const_one;

function yes: boolean;
begin
  yes := true;
end;

function no: boolean;
begin
  no := false;
end;
