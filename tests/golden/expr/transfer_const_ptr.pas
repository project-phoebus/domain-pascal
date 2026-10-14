{ Golden: an integer constant transferred to a pointer type of another
  size (owner's ruling, docs/reference-clarifications.md "Type Transfer
  Functions" (p. 4-197): integer constant to a pointer type; bead
  domain-pascal-gqp).  Apollo's streams/sasc misc.pas writes
  `ecp := ec2_$ptr_t( ec2_$always_ready_ec )' with ec2.ins.pas
  `ec2_$always_ready_ec = 1'.  No image site identified: the constant
  takes the paths of a 4-byte integer constant: codegen-patterns 13
  (`move.l #n' stores, as golden stores/const_stores) and 1.5 (a
  code-section cell passed by address; `pea (n).w' for a long by
  value). }
module tp;
type
  ec2_$ptr_t = ^integer32;
const
  ec2_$always_ready_ec = 1;
var
  e: ec2_$ptr_t;

procedure tp_$take(in ecp: ec2_$ptr_t); extern;
procedure tp_$take_v(in ecp: ec2_$ptr_t); val_param; extern;

procedure tp_$get(out ecp: ec2_$ptr_t);
begin
  ecp := ec2_$ptr_t(ec2_$always_ready_ec);
end;

procedure tp_$pass;
begin
  e := ec2_$ptr_t(ec2_$always_ready_ec);
  tp_$take(ec2_$ptr_t(ec2_$always_ready_ec));
  tp_$take_v(ec2_$ptr_t(ec2_$always_ready_ec));
end;
