{ Golden: codegen-patterns 4, "Main programs" (bead domain-pascal-vq0h): a
  program with no data has no base to load (/com/chhdir has an empty .data and
  no `lea' at its entry). }
program no_data_t;

procedure pgm_$exit; extern;

begin
  pgm_$exit;
end.
