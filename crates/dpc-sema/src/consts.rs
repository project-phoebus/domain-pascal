//! Compile-time constant values (ref 2.2.2.2 "Const Declaration Part",
//! p. 2-12; 3.2.3 p. 3-5: integer constants are represented as 32-bit
//! integers).

use crate::types::TypeId;

/// A constant value.  Integers are kept wide; the checker reports values
/// that do not fit the type they are used at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstValue {
    Int(i64),
    Bool(bool),
    /// A character code (ISO Latin-1).
    Char(u8),
    /// An enumeration constant: its type and ordinal.
    Enum {
        ty: TypeId,
        ord: i64,
    },
    /// A string of two or more characters.
    Str(Vec<u8>),
    Nil,
    /// A set constant: its members' ordinals, sorted, without duplicates.
    Set(Vec<i64>),
    /// A real constant: the bits of its value as a double (ref 3.3, p.
    /// 3-6; kept as bits so that the type stays `Eq`).
    Real(u64),
}

impl ConstValue {
    /// The value of a numeric constant as a double: an integer or a real.
    pub fn number(&self) -> Option<f64> {
        match self {
            ConstValue::Int(v) => Some(*v as f64),
            ConstValue::Real(b) => Some(f64::from_bits(*b)),
            _ => None,
        }
    }

    /// The ordinal of an ordinal constant (ref "Ord", p. 4-137).
    pub fn ordinal(&self) -> Option<i64> {
        match self {
            ConstValue::Int(v) => Some(*v),
            ConstValue::Bool(b) => Some(*b as i64),
            ConstValue::Char(c) => Some(*c as i64),
            ConstValue::Enum { ord, .. } => Some(*ord),
            _ => None,
        }
    }
}
