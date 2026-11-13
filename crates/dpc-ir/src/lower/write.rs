//! `write` and `writeln` to the standard output ("Write, Writeln", pp.
//! 4-215..4-220), best effort (not byte-identical; bead
//! domain-pascal-338r, domain-c's gap 4; no kernel site): calls of the
//! Pascal run time's PAS_$WRITE, one per item, and PAS_$WRITELN, as the
//! DOMAIN C compiler (Pascal Rev 8.89) compiles them (reported by
//! domain-c: c_macro_utl's messages, objio's check_init).  The run time's
//! headings are ../source's lib/ftnlib/pasio.pas (SR10.2 ftnlib):
//!
//! ```text
//! procedure pas_$write(fmt: fmt_t; var f: univ char; in v: univ value_t;
//!     width: integer; digits: integer; in names: univ names_t);
//!     options(val_param);
//! procedure pas_$writeln(var f: univ char);
//! ```
//!
//! `fmt` is a packed word (../source evidence/lib/ftnlib/pasio.codegen.md):
//! the high byte the default width, bits 7..6 the width's form (0 none, 1
//! a width, 2 a width and digits), 5..4 the size (1 byte, 2 word, 3
//! long), 3..0 the kind (0 integer, 1 integer32, 2 char, 3 character
//! array, 4 boolean, 5 enumeration, 6 real, 7 double, 8 unsigned, 9
//! unsigned32, 10 varying string).  The default widths are the DOMAIN C
//! compiler's words where seen (domain-c's rev889-patterns 170: integers
//! 10, booleans and enumerations 15, doubles 24, chars 1, character
//! arrays 0), the reference's 13 for a single (p. 4-218; Uncertain).
//! For a character array `width` is the array's length and `digits` the
//! field width.  The file is `PAS_$OUTPUT`, the run time's standard
//! output.

use super::*;

const KIND_INTEGER: u16 = 0;
const KIND_INTEGER32: u16 = 1;
const KIND_CHAR: u16 = 2;
const KIND_CHARS: u16 = 3;
const KIND_BOOLEAN: u16 = 4;
const KIND_ENUM: u16 = 5;
const KIND_REAL: u16 = 6;
const KIND_DOUBLE: u16 = 7;
const KIND_UNSIGNED: u16 = 8;
const KIND_UNSIGNED32: u16 = 9;
const KIND_VARYING: u16 = 10;

fn output() -> Addr {
    Addr::Symbol {
        name: "PAS_$OUTPUT".into(),
        offset: 0,
    }
}

/// A field width or digits: `None` for [`dpc_sema::NO_FIELD`].
fn field(e: &SExpr) -> Option<&SExpr> {
    match &e.value {
        Some(ConstValue::Int(dpc_sema::NO_FIELD)) => None,
        _ => Some(e),
    }
}

/// A word value argument (`width`, `digits`).
fn word(p: &Program, e: Option<&SExpr>) -> Res<Arg> {
    let ty = Scalar::signed(Width::Word);
    let value = match e {
        Some(e) => match &e.value {
            Some(v) => const_of(v, ty, e.span)?,
            None => convert(value_of(p, e)?, ty),
        },
        None => Expr::Const { value: 0, ty },
    };
    Ok(Arg::Value { value, slot: 2 })
}

/// An enumeration's name table: a word count, then the names as counted
/// strings in upper case (pasio.codegen.md; domain-c's rev889-patterns
/// 65: the IL node kinds' names).
fn names_table(consts: &[String]) -> Vec<u8> {
    let mut b = super::word_bytes(consts.len() as u16).to_vec();
    for c in consts {
        b.push(c.len() as u8);
        b.extend(c.to_ascii_uppercase().bytes());
    }
    if b.len() % 2 == 1 {
        b.push(0);
    }
    b
}

/// The item `v`'s address: a variable's, a string constant's cell, or a
/// frame temporary holding its value.
fn item(p: &Program, v: &SExpr) -> Res<Arg> {
    if let Some(ConstValue::Str(_)) = &v.value {
        return Ok(Arg::Address {
            addr: Addr::Const(pool_const(p, v.value.as_ref().unwrap(), v.ty, v.span)?),
            slot: 4,
            writable: false,
        });
    }
    if let Some(q) = real::prec_of(p, v.ty) {
        if let Some(a) = real::arg(p, v, q)? {
            return Ok(a);
        }
    }
    if v.value.is_none() && v.is_designator() && bit_field(p, v)?.is_none() {
        return Ok(Arg::Address {
            addr: addr_of(p, v)?,
            slot: 4,
            writable: false,
        });
    }
    Ok(Arg::Temp {
        value: value_of(p, v)?,
        offset: 0,
        slot: 4,
    })
}

/// `write(...)` or `writeln(...)`: `args` the items as (value, width,
/// digits) triples ([`Builtin::Write`]).
pub(super) fn write(p: &Program, st: &SStmt, newline: bool, args: &[SExpr]) -> Res<Vec<Stmt>> {
    best_effort(
        st.span,
        "`write`/`writeln` (calls of the run time's PAS_$WRITE and PAS_$WRITELN; no kernel \
         site) is",
    )?;
    let mut out = Vec::new();
    for t in args.chunks(3) {
        let (v, w, d) = (&t[0], field(&t[1]), field(&t[2]));
        let ty = p.types.canon(v.ty);
        let size = p.types.size(ty);
        let size_code = match size {
            1 => 1u16,
            2 => 2,
            _ => 3,
        };
        let mut names = None;
        let (default, kind, mut width_arg, mut digits_arg) = if let Some(n) =
            p.types.char_array_len(ty)
        {
            // A character array: `width` its length, `digits` the field.
            let n = Expr::Const {
                value: n as i64,
                ty: Scalar::signed(Width::Word),
            };
            (0u16, KIND_CHARS, Arg::Value { value: n, slot: 2 }, word(p, w)?)
        } else if let TypeKind::StringLit { len } = p.types.kind(ty) {
            let n = Expr::Const {
                value: *len as i64,
                ty: Scalar::signed(Width::Word),
            };
            (0, KIND_CHARS, Arg::Value { value: n, slot: 2 }, word(p, w)?)
        } else {
            let (default, kind) = match p.types.kind(ty) {
                TypeKind::Varying { .. } => (0, KIND_VARYING),
                TypeKind::Real { size: 4 } => (13, KIND_REAL),
                TypeKind::Real { .. } => (24, KIND_DOUBLE),
                TypeKind::Boolean => (15, KIND_BOOLEAN),
                TypeKind::Char => (1, KIND_CHAR),
                TypeKind::Enum { consts } => {
                    names = Some(names_table(consts));
                    (15, KIND_ENUM)
                }
                _ if p.types.is_char(ty) => (1, KIND_CHAR),
                _ if matches!(p.types.kind(ty), TypeKind::Subrange { .. })
                    && p
                        .types
                        .ordinal_base(ty)
                        .is_some_and(|b| matches!(p.types.kind(b), TypeKind::Enum { .. })) =>
                {
                    let b = p.types.ordinal_base(ty).unwrap();
                    if let TypeKind::Enum { consts } = p.types.kind(b) {
                        names = Some(names_table(consts));
                    }
                    (15, KIND_ENUM)
                }
                // A pointer: as an integer32 (reported: 0x0A31).
                _ if p.types.is_pointer(ty) => (10, KIND_INTEGER32),
                _ if p.types.is_integer(ty) => {
                    let unsigned = sign_of(&p.types, ty) == Sign::Unsigned;
                    let kind = match (size, unsigned) {
                        (4, false) => KIND_INTEGER32,
                        (4, true) => KIND_UNSIGNED32,
                        (_, false) => KIND_INTEGER,
                        (_, true) => KIND_UNSIGNED,
                    };
                    (10, kind)
                }
                _ => return Err(unsupported(v.span, "writing a value of this type is")),
            };
            (default, kind, word(p, w)?, word(p, d)?)
        };
        let form = match (w, d) {
            (None, _) => 0u16,
            (Some(_), None) => 1,
            (Some(_), Some(_)) => 2,
        };
        // A character array's field is its `digits`: form 1 when given.
        if kind == KIND_CHARS {
            digits_arg = word(p, w)?;
        }
        if matches!(kind, KIND_CHARS | KIND_VARYING) && w.is_none() {
            digits_arg = word(p, None)?;
        }
        if kind == KIND_VARYING {
            width_arg = word(p, w)?;
        }
        let fmt = (default << 8) | (form << 6) | (size_code << 4) | kind;
        let fmt = Arg::Value {
            value: Expr::Const {
                value: fmt as i64,
                ty: Scalar::unsigned(Width::Word),
            },
            slot: 2,
        };
        let names = match names {
            Some(bytes) => Arg::Address {
                addr: Addr::Const(PoolConst {
                    bytes,
                    key: ty.0,
                    symbol: None,
                }),
                slot: 4,
                writable: false,
            },
            None => Arg::Value {
                value: Expr::Const {
                    value: 0,
                    ty: Scalar::unsigned(Width::Long),
                },
                slot: 4,
            },
        };
        let args = vec![
            fmt,
            Arg::Address {
                addr: output(),
                slot: 4,
                writable: true,
            },
            item(p, v)?,
            width_arg,
            digits_arg,
            names,
        ];
        out.push(Stmt {
            kind: StmtKind::Call(Call {
                callee: Callee::External("PAS_$WRITE".into()),
                args,
                // 2 + 4 + 4 + 2 + 2 + 4 bytes: a pad word first.
                pad: 2,
                link: None,
            }),
            span: st.span,
        });
    }
    if newline {
        out.push(Stmt {
            kind: StmtKind::Call(Call {
                callee: Callee::External("PAS_$WRITELN".into()),
                args: vec![Arg::Address {
                    addr: output(),
                    slot: 4,
                    writable: true,
                }],
                pad: 0,
                link: None,
            }),
            span: st.span,
        });
    }
    Ok(out)
}
