//! PROC1_$TST_LOCK (0xE148CA, 0x1C bytes), the first conformance target
//! (CLAUDE.md 6.3).  The image:
//!
//! ```text
//! e148ca  link.w  A6,#-4
//! e148ce  movea.l 0xE1EAC8,A0         ; PROC1_$CURRENT_PCB (extern)
//! e148d4  move.w  (0x8,A6),D0w        ; lock_id by value, 2-byte slot at +8
//! e148d8  move.l  (0x40,A0),D1        ; resource_locks_held, a 4-byte set
//! e148dc  btst    D0,D1
//! e148de  sne     D1
//! e148e0  move.b  D1,D0               ; boolean result in D0.b
//! e148e2  unlk    A6
//! e148e4  rts
//! ```

use super::*;
use crate::conv::{Passing, ResultReg};
use crate::program::{ExprKind, StmtKind, Storage};
use dpc_syntax::ast::BinOp;

/// Same text as the parser's whole-module test
/// (crates/dpc-syntax/src/parser_tests.rs).
const TST_LOCK: &str = "module proc1_tst;
type
  proc1_$pcb_t = record
    pad: array[1..16] of integer32;      { 0x00..0x3F }
    resource_locks_held: set of 0..31;   { 0x40 }
  end;
var
  proc1_$current_pcb: extern ^proc1_$pcb_t;
function proc1_$tst_lock(lock_id: integer): boolean; options(val_param);
begin
  proc1_$tst_lock := lock_id in proc1_$current_pcb^.resource_locks_held;
end;
";

#[test]
fn tst_lock_analyses() {
    let p = ok(TST_LOCK);
    assert_eq!(p.name, "proc1_tst");
    assert!(!p.is_program);
    assert_eq!(
        (p.code_section.as_str(), p.data_section.as_str()),
        (".text", ".data")
    );

    // The PCB layout.
    assert_eq!(offset(&p, "proc1_$pcb_t", "resource_locks_held"), 0x40);
    let set_ty = field(&p, "proc1_$pcb_t", "resource_locks_held").ty;
    assert_eq!(p.types.size(set_ty), 4);

    // The extern variable, reached by symbol.
    let pcb = var(&p, "proc1_$current_pcb");
    assert_eq!(
        pcb.storage,
        Storage::Extern {
            symbol: "PROC1_$CURRENT_PCB".into()
        }
    );
    assert!(
        p.data_sections.is_empty(),
        "nothing allocated: {:?}",
        p.data_sections
    );

    // The routine: exported, val_param, lock_id by value in a 2-byte slot
    // at +8, no pad issue for the callee (2 bytes + 2 pad for callers).
    let r = routine(&p, "proc1_$tst_lock");
    assert_eq!(r.symbol, "PROC1_$TST_LOCK");
    assert!(r.global && !r.external);
    assert!(r.options.val_param);
    assert_eq!(r.params.len(), 1);
    let lock_id = &r.params[0];
    assert_eq!(lock_id.conv.passing, Passing::ByValue);
    assert_eq!(lock_id.conv.slot_size, 2);
    assert!(!lock_id.conv.local_copy);
    assert_eq!((lock_id.slot_offset, lock_id.value_offset), (8, 8));
    assert_eq!((r.arg_bytes, r.arg_pad), (2, 2));

    // Boolean result in the data register.
    let res = r.result.as_ref().unwrap();
    assert_eq!(res.reg, ResultReg::Data);
    assert_eq!(p.types.size(res.ty), 1);
    // The result variable has a frame slot (open question, see
    // codegen-patterns 2: the image has `link #-4`).
    let rv = p.var(res.var.unwrap());
    assert_eq!(rv.storage, Storage::Frame { offset: -1 });
    assert_eq!(r.frame_size, 1);

    // The body: result := lock_id in pcb^.resource_locks_held.
    let body = r.body.as_ref().unwrap();
    // The `;` before `end` leaves an empty statement.
    assert_eq!(body.stmts.len(), 2);
    assert!(matches!(body.stmts[1].kind, StmtKind::Empty));
    let StmtKind::Assign { target, value } = &body.stmts[0].kind else {
        panic!("{:?}", body.stmts[0].kind)
    };
    assert!(matches!(target.kind, ExprKind::Result { uplevel: 0, .. }));
    let ExprKind::Binary { op, lhs, rhs } = &value.kind else {
        panic!("{:?}", value.kind)
    };
    assert_eq!(*op, BinOp::In);
    assert!(p.types.is_boolean(value.ty));
    assert!(matches!(lhs.kind, ExprKind::Var { var, uplevel: 0 } if var == lock_id.var));
    let ExprKind::Field { base, offset, .. } = &rhs.kind else {
        panic!("{:?}", rhs.kind)
    };
    assert_eq!(*offset, 0x40);
    assert_eq!(rhs.ty, set_ty);
    let ExprKind::Deref(ptr) = &base.kind else {
        panic!("{:?}", base.kind)
    };
    assert!(matches!(ptr.kind, ExprKind::Var { uplevel: 0, .. }));
}
