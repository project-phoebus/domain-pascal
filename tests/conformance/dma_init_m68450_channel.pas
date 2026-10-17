{ Module-local routine at 0xE0A328 (the first routine of the map's DMA_
  module), 0x3A bytes: 0xE0A328..0xE0A362 (tools/imgdis.sh E0A328 3A),
  and DMA_$INIT after it, 0x44 bytes: 0xE0A362..0xE0A3A6.
  Semantics: ../source/domain_os/dma/init_m68450_channel.c, where it is
  called DMA_$INIT_M68450_CHANNEL (a C name; the map does not name the
  routine): put one channel of the MC68450 DMA controller into its idle
  state, five byte stores to its registers.

  The unit is a fragment of DMA_ (its first 0x3A bytes), so
  tools/conform.py compiles it with `dpc --fragment-origin 0xE0A328'.  No
  loops, no module end: no loop-alignment option.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  internal: both parameters by value, the pointer at (8,A6), the
         channel number word at (0xC,A6) (DMA_$INIT pushes the pointer
         with `pea 0xffa0c0' / `move.l #0xffa000,-(SP)' and the word with
         `move.w #3,-(SP)' / `clr.w -(SP)', 0xE0A366..0xE0A3A0);
  - 2    link.w A6,#-4 (the slot of a routine that is not a global
         symbol), no register save;
  - 12.2 the word parameter loaded at entry into D0, stored from it at the
         end as a byte (`move.b D0,(0x2d,A0)', 0xE0A35A);
  - 13   device registers: no `clr'/`st', a constant byte stored with
         `move.b #n' (`move.b #-1,(A0)' as `10bc 00ff');
  - 1.5  DMA_$INIT: `addr(x)' by value of another unit's element at a
         constant index, the push of the element by address: `pea
         (DMA+0xc0).l' (0xE0A36C), `move.l #DMA,-(SP)' for the element at
         0 (0xE0A38A), after the pad and the word (bead
         domain-pascal-hs92); `bsr.s' to the module-local, the last pop
         left to `unlk';
  - 3.15.3 device: the register block's pointer is read from its frame
         slot before each reference (`movea.l (8,A6),A0' five times,
         0xE0A330..0xE0A356), not loaded at entry and held as a
         parameter used more than once is (codegen-patterns 12.2,
         "device").

  Reconstruction notes: the routine is `internal' here because the map
  does not name it and every call reaches it with `bsr' (DMA_$INIT).  The
  register block is declared `[device]' (ref 3.15.3): what the original
  declared is not known; the device reading is what explains both the
  reloads and the `move.b' stores (codegen-patterns 13).  The registers
  are `char' with chr() for their byte values (../source: uint8_t); the
  MC68450 register names and offsets are the C's (dma/dma.h: CSR 0x00,
  CER 0x01, DCR 0x04, OCR 0x05, SCR 0x06, CCR 0x07, CPR 0x2D); the other
  bytes are filler (single chars: an array of chars is word aligned in a
  record, ref 3.10.5.5).  The parameter names are the C's. }
module dma_;
type
  m68450_channel_t = [device] record
    csr: char;                        { 0x00 channel status }
    cer: char;                        { 0x01 channel error }
    pad02, pad03: char;
    dcr: char;                        { 0x04 device control }
    ocr: char;                        { 0x05 operation control }
    scr: char;                        { 0x06 sequence control }
    ccr: char;                        { 0x07 channel control }
    pad08: array[8..44] of char;
    cpr: char;                        { 0x2D channel priority }
    pad2e: array[46..63] of char;     { the channels are 0x40 apart }
  end;
  m68450_channel_ptr_t = ^m68450_channel_t;
var
  { DMA 0xFFA000 (IODEFS, a map symbol): the four channels. }
  dma: extern array[0..3] of m68450_channel_t;

procedure dma_$init_m68450_channel(chan_virtual_address: m68450_channel_ptr_t;
                                   channel_number: integer); internal;
begin
  chan_virtual_address^.ccr := chr(16#10);           { 0xE0A334 SAB }
  chan_virtual_address^.csr := chr(16#ff);           { 0xE0A33E }
  chan_virtual_address^.dcr := chr(16#28);           { 0xE0A346 }
  chan_virtual_address^.scr := chr(4);               { 0xE0A350 }
  chan_virtual_address^.cpr := chr(channel_number);  { 0xE0A35A }
end;

{ DMA_$INIT 0xE0A362 (../source dma/init.pas, dma/init.c): channels 3, 2,
  0 and 1, in that order. }
procedure dma_$init;
begin
  dma_$init_m68450_channel(addr(dma[3]), 3);         { 0xE0A366 }
  dma_$init_m68450_channel(addr(dma[2]), 2);         { 0xE0A376 }
  dma_$init_m68450_channel(addr(dma[0]), 0);         { 0xE0A386 }
  dma_$init_m68450_channel(addr(dma[1]), 1);         { 0xE0A394 }
end;
