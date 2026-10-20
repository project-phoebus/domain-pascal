//! Parameter-passing and function-result conventions (ref 5.2 Table 5-1,
//! p. 5-5; 5.5 routine options, pp. 5-15..5-21).  Pure functions, so
//! Table 5-1 can be tested row by row (`conv_tests.rs`).
//!
//! Table 5-1 (read from the PDF, p. 5-5; the OCR scrambles it):
//!
//! | mode       | default caller / callee                    | > 4 bytes     | <= 4 bytes |
//! |------------|--------------------------------------------|---------------|------------|
//! | IN         | passes address / indirect references      | no effect     | val_param or internal: by value, direct references; UNIV cancels |
//! | IN OUT, VAR| passes address / indirect references      | no effect     | no effect |
//! | no mode    | passes address / local copy, direct refs  | no effect     | val_param or internal: by value; UNIV cancels (callee copies as described by the parameter) |
//!
//! OUT is not in the table; 5.5.7 (p. 5-19) passes by value only
//! parameters "declared as value parameters or as in parameters", so OUT
//! goes by address like VAR.

use dpc_syntax::ast::ParamMode;
use dpc_target::Target;

use crate::types::{TypeId, TypeKind, TypeTable};

/// The routine options that affect the interface (ref 5.5, p. 5-15).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoutineOptions {
    pub forward: bool,
    pub extern_: bool,
    pub internal: bool,
    pub variable: bool,
    pub abnormal: bool,
    pub val_param: bool,
    pub nosave: bool,
    pub noreturn: bool,
    pub d0_return: bool,
    pub a0_return: bool,
    pub c_param: bool,
}

impl RoutineOptions {
    /// Small value and `in` parameters go by value: `val_param` (5.5.7,
    /// p. 5-19); `internal` ("Currently, however, internal routines are
    /// treated the same as val_param routines", 5.2 p. 5-4); `c_param`
    /// ("equivalent to specifying d0_return and val_param", 5.5.12
    /// p. 5-21).
    pub fn small_by_value(&self) -> bool {
        self.val_param || self.internal || self.c_param
    }

    /// Set the option called `name` (folded).  Returns false for a name
    /// that is not a predeclared routine option.
    pub fn set(&mut self, name: &str) -> bool {
        match name {
            "forward" => self.forward = true,
            "extern" => self.extern_ = true,
            "internal" => self.internal = true,
            "variable" => self.variable = true,
            "abnormal" => self.abnormal = true,
            "val_param" => self.val_param = true,
            "nosave" => self.nosave = true,
            "noreturn" => self.noreturn = true,
            "d0_return" => self.d0_return = true,
            "a0_return" => self.a0_return = true,
            "c_param" => self.c_param = true,
            _ => return false,
        }
        true
    }
}

/// How an argument travels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Passing {
    /// The caller pushes the argument's address.
    ByAddress,
    /// The caller pushes the value itself.
    ByValue,
}

/// The convention of one parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParamConv {
    pub passing: Passing,
    /// Bytes the argument occupies in the caller's push.  A by-value
    /// argument smaller than the target's slot is widened to a slot (1-byte
    /// values take a 2-byte slot on m68k, codegen-patterns 1.3).
    pub slot_size: u32,
    /// The callee copies the argument into its frame on entry: a value
    /// parameter passed by address (Table 5-1, "no mode", default callee:
    /// "Creates local copies of arguments and makes direct references to
    /// them"; 5.2 p. 5-4 "by making a local copy of the argument").
    pub local_copy: bool,
}

/// Decide the convention of one parameter.
///
/// `small_by_value` is [`RoutineOptions::small_by_value`] (or the nested
/// routine rule, see `check.rs`); `c_param` additionally passes records by
/// value whatever their size (5.5.12, p. 5-21).
pub fn param_conv(
    mode: ParamMode,
    univ: bool,
    ty: TypeId,
    small_by_value: bool,
    c_param: bool,
    types: &TypeTable,
    target: &dyn Target,
) -> ParamConv {
    let by_address = ParamConv {
        passing: Passing::ByAddress,
        slot_size: target.pointer_size(),
        local_copy: false,
    };
    let value_like = matches!(mode, ParamMode::Value | ParamMode::In);
    if !value_like {
        // VAR, IN OUT: Table 5-1 row 2, every modifier "No effect".  OUT:
        // see the module comment.
        return by_address;
    }
    let size = types.size(ty);
    let is_record = matches!(types.kind(ty), TypeKind::Record(_));
    // "All character arrays are treated like arguments with fewer than four
    // bytes, regardless of their size" (5.2 NOTE, p. 5-4) contradicts 5.5.7
    // p. 5-19 "(except for character arrays)".  TODO(ref-question): the
    // conservative reading (5.5.7, and codegen-patterns 1.2) is used:
    // character arrays always go by address.
    let char_array = types.char_array_len(ty).is_some();
    // TODO(ref-question): 5.5.7 says "all arguments four bytes or less" go
    // by value under val_param; whether that includes small records and
    // non-char arrays, or whether only c_param passes records by value
    // (5.5.12), is not stated.  The literal reading of 5.5.7 is used.
    let small_enough = size <= target.val_param_max_size() && !char_array;
    // UNIV "Cancels the effect of val_param and internal" (Table 5-1).
    let by_value = !univ && ((small_by_value && small_enough) || (c_param && is_record));
    if by_value {
        ParamConv {
            passing: Passing::ByValue,
            slot_size: crate::types::round_up(size.max(1), target.arg_slot_size()),
            local_copy: false,
        }
    } else {
        ParamConv {
            local_copy: mode == ParamMode::Value,
            ..by_address
        }
    }
}

/// Which result register class a function uses.  The target maps the
/// classes to registers (m68k: data = D0, address = A0; codegen-patterns 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultReg {
    /// The data result register (D0), sized to the type.
    Data,
    /// The address result register (A0): pointer results by default
    /// (5.5.10 p. 5-20), every register result under `a0_return` (5.5.11).
    Address,
    /// `d0_return` on a pointer result: the callee puts the pointer in both
    /// registers; callers read the data register (5.5.10, p. 5-20).
    AddressAndData,
    /// A 4-byte record (`status_$t`, base.ins.pas): in the data register
    /// as a longword, which callers read like a [`ResultReg::Data`]
    /// result, with no caller-provided storage (codegen-patterns 3,
    /// "Record results": the 23 compiled calls of FIM_$CLEANUP, Apollo's
    /// `pfm_$cleanup ...: status_$t`, `pea; jsr; addq.w #4,SP; move.l
    /// D0,<dst>`).  The callee's result variable is a frame object like
    /// any record: `f := r` copies into its slot, which is loaded into
    /// the register at the end (PEB_$GET_STATUS 0xE5AE52, VOLX_$SHUTDOWN
    /// 0xE6B5B2).
    Record,
    /// A record of more than 4 bytes: through a hidden first argument, the
    /// address of a place for the result that the caller pushes last
    /// (after the arguments, with no pad) and the callee stores through;
    /// no register carries it back (codegen-patterns 3, "Record results
    /// over 4 bytes": ACL_$DEFAULT_ACL 0xE4787E and NAME_$GET_NODE_UID
    /// 0xE589DE store a `uid_$t` through `(8,A6)`, the module-local
    /// 0xE58C3A a 6-byte `time_$clock_t`; their 21 compiled calls push a
    /// frame temporary's address last and copy the record from it, or
    /// pass the temporary on by address).  The callee's result variable is
    /// that place ([`crate::Storage::HiddenResult`]), so it has no frame
    /// slot (ACL_$DEFAULT_ACL `link #0`).
    Hidden,
    /// Not known yet: sets, reals, records of fewer than 4 bytes, and
    /// records under `a0_return`, `d0_return` or `c_param`
    /// (codegen-patterns 3, Uncertain).  The declaration is accepted, so that Apollo's inserts
    /// that declare such functions check (bead domain-pascal-qia); a
    /// definition or a call is refused when lowered (`dpc-ir`), with
    /// [`unknown_result_message`].
    Unknown,
}

/// The refusal of a definition of, or a call to, a function whose result
/// convention is [`ResultReg::Unknown`].
pub fn unknown_result_message(ty: TypeId, types: &TypeTable) -> String {
    format!(
        "functions returning {} are not supported yet (codegen-patterns 3: result convention unknown)",
        types.display(ty)
    )
}

/// Decide the result register of a function returning `ty`.  `Err` names
/// results that cannot be returned in a register;
/// [`ResultReg::Unknown`] is a result whose convention is not known yet.
pub fn result_reg(
    ty: TypeId,
    opts: &RoutineOptions,
    types: &TypeTable,
    target: &dyn Target,
) -> Result<ResultReg, String> {
    let pointer = matches!(
        types.kind(ty),
        TypeKind::Pointer { .. } | TypeKind::UnivPtr | TypeKind::RoutinePointer(_)
    );
    if pointer {
        // 5.5.10: d0_return puts a pointer in A0 and D0, callers expect D0;
        // c_param implies d0_return (5.5.12).
        if opts.d0_return || opts.c_param {
            return Ok(ResultReg::AddressAndData);
        }
        return Ok(ResultReg::Address);
    }
    if types.is_ordinal(ty) || types.is_error(ty) {
        if types.size(ty) > target.val_param_max_size() {
            return Err("result too large for a register".into());
        }
        // 5.5.11: a0_return "Puts into A0 any values returned from that
        // routine that should go into a register".
        return Ok(if opts.a0_return {
            ResultReg::Address
        } else {
            ResultReg::Data
        });
    }
    // A set of one word or one longword: in D0, sized to the type, like
    // an ordinal (codegen-patterns 3, "Set results"): ACL_$MIN_RIGHTS's
    // `set of 0..15` `moveq #0xf,D0; and.w D2w,D0w` (0xE469F2), read by
    // its caller from D0 (`move.w D0w,D2w`, DIR's 0xE4CF6x); PROC2_$
    // SIGBLOCK's `set of 0..31` `move.l D2,D0` (0xE3F6B4), with no hidden
    // argument (its two slots are its two parameters).  Under `a0_return`:
    // no site, left unknown.
    if matches!(types.kind(ty), TypeKind::Set { .. })
        && matches!(types.size(ty), 2 | 4)
        && !opts.a0_return
    {
        return Ok(ResultReg::Data);
    }
    // A 4-byte record: in D0 (codegen-patterns 3, "Record results"; seen
    // for `status_$t` only, a packed record with a variant, base.ins.pas).
    // `a0_return` ("Puts into A0 any values returned from that routine
    // that should go into a register", 5.5.11 p. 5-20) on one: no site,
    // left unknown.  `d0_return` and `c_param` speak of pointer results
    // only (5.5.10, 5.5.12), so they change nothing here.
    if matches!(types.kind(ty), TypeKind::Record(_))
        && types.size(ty) == target.val_param_max_size()
        && !opts.a0_return
    {
        return Ok(ResultReg::Record);
    }
    // A larger record: through a hidden address (codegen-patterns 3,
    // "Record results over 4 bytes"; seen for 6- and 8-byte records,
    // `time_$clock_t` and `uid_$t`, and taken for every record over 4
    // bytes, Uncertain for other sizes).  Under `a0_return` ("Puts into
    // A0 any values returned from that routine that should go into a
    // register", 5.5.11 p. 5-20), `d0_return` (5.5.10) or `c_param`
    // ("returns results in D0", 5.2 p. 5-3): no site, left unknown.
    // TODO(target): an ABI decision of Apollo's m68k compiler, to move
    // into the target description (bead domain-pascal-wiue).
    if matches!(types.kind(ty), TypeKind::Record(_))
        && types.size(ty) > target.val_param_max_size()
        && !opts.a0_return
        && !opts.d0_return
        && !opts.c_param
    {
        return Ok(ResultReg::Hidden);
    }
    // Sets, reals, other records: how they are returned is not yet
    // observed (codegen-patterns 3, Uncertain).
    Ok(ResultReg::Unknown)
}
