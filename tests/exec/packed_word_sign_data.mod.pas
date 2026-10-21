{ The other unit of packed_word_sign.pas: a word packed record that
  packed_word_sign.pas reaches as another unit's object (an absolute
  operand, as PBU_$MCB in sau5; bead domain-pascal-g9kx). }
module pws_data;

type
  mcb_t = packed record case integer of
      1:  (present: boolean;
           wired: boolean;
           unused: 0..16383;
           setups: integer);
      2:  (word0: integer;
           word1: integer);
      3:  (all: integer32);
      end;
var
  ext_mcb: mcb_t;
define ext_mcb;
