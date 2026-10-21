{ Golden: one constant member added to or removed from a `[device]'
  set (ref 3.15.3 pp. 3-57..3-59; codegen-patterns 5.5, 5.1; bead
  domain-pascal-791y): `ori'/`andi' of the mask at the set's own width,
  in place, never `bset'/`bclr #k' on one of its bytes.  The sau5
  image's hardware control words (`sau5/domain_os', map
  `sau5/domain_os.map'):
  - enable_gpu_int: COLOR2_$ENABLE_GPU_INT 0xE7355C `andi.w
    #-5,(0xf9700c).l', 0xE73564 `ori.w #0x80,(0xf9700c).l' (KOLOR_CTL's
    ctl); disable_gpu_int: COLOR2_$DISABLE_GPU_INT 0xE7357A `andi.w
    #-0x81,(0xf9700c).l';
  - busy_wait: COLOR2_$BUSY_WAIT 0xE734DE `ori.w
    #1,(COLOR3_$CR_CONTENTS).l', 0xE734FE `andi.w #-2' (and
    COLOR2_$VIDEO_CTL 0xE1E6FA, 0xE1E704 by 8);
  - inq_done: COLOR_$INQ_DONE_WAIT 0xE730C8 `ori.w #2,(0xa0,A3)',
    0xE73128 `andi.w #-3,(0xa0,A3)' (COLOR_SUP's int_ctl; here by its
    absolute address);
  - byte and longword sets of a device: `ori.b'/`andi.b',
    `ori.l'/`andi.l' (no site; the same rule);
  - plain: a set that is not a device keeps `bset #k' on the member's
    byte (TTY_$K_SET_FLAG 0xE6745C, codegen-patterns 5.1), the member
    number unreduced on an absolute operand (sau5 MT_$WAIT 0xE76A1A
    `bclr #13,(0xe90cc0).l'; bead domain-pascal-fozq). }
module device_set_members;
type
  kolor_ctl_t = [device] record
    unused_00: array[1..6] of integer;
    ctl: set of 0..15;                    { 0x0C }
  end;
  color_sup_t = [device] record
    unused_00: array[1..80] of integer;
    int_ctl: set of 0..15;                { 0xA0 }
  end;
  other_t = [device] record
    b: [byte] set of 0..7;
    pad: [byte] 0..255;
    l: set of 0..31;
  end;

var
  kolor_ctl: extern kolor_ctl_t;
  color3_$cr_contents: extern [device] set of 0..15;
  color_sup: extern color_sup_t;
  other: extern other_t;
  plain: extern set of 0..15;

procedure enable_gpu_int;
begin
  kolor_ctl.ctl := kolor_ctl.ctl - [2];
  kolor_ctl.ctl := kolor_ctl.ctl + [7];
end;

procedure disable_gpu_int;
begin
  kolor_ctl.ctl := kolor_ctl.ctl - [7];
end;

procedure busy_wait;
begin
  color3_$cr_contents := color3_$cr_contents + [0];
  color3_$cr_contents := color3_$cr_contents - [0];
  color3_$cr_contents := color3_$cr_contents + [3];
end;

procedure inq_done;
begin
  color_sup.int_ctl := color_sup.int_ctl + [1];
  color_sup.int_ctl := color_sup.int_ctl - [1];
end;

procedure widths;
begin
  other.b := other.b + [3];
  other.b := other.b - [7];
  other.l := other.l + [20];
  other.l := other.l - [1];
end;

procedure not_device;
begin
  plain := plain + [7];
  plain := plain - [12];
end;
