//! The type system and data layout (ref chapter 3; codegen-patterns 5).
//!
//! Types live in a [`TypeTable`] and are named by [`TypeId`].  Type
//! identity is by id: `type a = b;` makes `a` another name for `b`'s id,
//! while every `record ... end`, `(...)`, `^T` written in the source makes
//! a new type (standard Pascal name equivalence).
//!
//! Every type's layout is computed when the type is created:
//!
//! - `size`: bytes the object occupies;
//! - `align`: the alignment of a *freestanding* object of the type (a
//!   variable), ref 3.2.4 p. 3-5, 3.5.3 p. 3-11, 3.6.4 p. 3-13, 3.13.5
//!   p. 3-52, and the record allocation algorithm 3.10.5.6 pp. 3-27..3-28;
//! - `field_align`: the "guaranteed default alignment" of the type as a
//!   field of an unpacked record in the default (word) alignment
//!   environment, ref 3.10.5.3 Table 3-2 p. 3-24 and 3.10.5.5 p. 3-26.
//!
//! Only the default word-alignment environment is implemented.  Natural
//! alignment (`%natural_alignment`, `aligned record`, the `natural` /
//! `aligned(n)` attributes, ref 3.10.6 and 3.15.6) and packed records are
//! rejected by the checker for now.

use std::fmt;

use dpc_syntax::Span;

/// Index of a type in the [`TypeTable`].  (`Default` only so an empty
/// [`TypeTable::default`] can exist; it names no type.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(pub u32);

/// The structure of a type.
#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    /// Stands in for a type that failed to check, so one mistake does not
    /// cascade into many diagnostics.  Compatible with everything.
    Error,
    /// `integer`, `integer16`: 16-bit signed (ref 3.2.1, p. 3-4).
    Integer16,
    /// `integer32`: 32-bit signed (ref 3.2.1, p. 3-4).
    Integer32,
    /// 1 byte, 0xFF true, 0 false (ref 3.5.3, p. 3-11).
    Boolean,
    /// 1 byte, ISO Latin-1 (ref 3.6.4, p. 3-13).
    Char,
    /// `real`/`single` (4 bytes) or `double` (8 bytes) (ref 3.3.1, 3.3.4
    /// p. 3-7).  Layout only: no real arithmetic is implemented
    /// (CLAUDE.md 1, out of scope).
    Real {
        size: u32,
    },
    /// An enumeration; ordinal `i` is `consts[i]` (ref 3.7, 3.7.1 p. 3-13).
    Enum {
        consts: Vec<String>,
    },
    /// `lo..hi` of an ordinal host type (ref 3.8, p. 3-14).  `base` is
    /// never itself a subrange.
    Subrange {
        base: TypeId,
        lo: i64,
        hi: i64,
    },
    /// `set of elem` (ref 3.9).  `elem` is an ordinal type with ordinals
    /// in 0..=1023 (3.9.1, p. 3-15).
    Set {
        elem: TypeId,
    },
    /// One dimension of an array; `array[i, j] of T` is
    /// `array[i] of array[j] of T` (row-major, ref 3.11.4.1 p. 3-46).
    Array {
        index: TypeId,
        elem: TypeId,
        packed: bool,
    },
    /// `varying[max] of char` (ref 3.11.2 p. 3-44; layout 3.11.4.1 p. 3-46).
    /// Layout only for now.
    Varying {
        max: u32,
        elem: TypeId,
    },
    Record(RecordType),
    /// `^T` (ref 3.13.1).
    Pointer {
        target: TypeId,
    },
    /// `univ_ptr`, compatible with every pointer type (ref 3.13.2, p. 3-50).
    UnivPtr,
    /// The type of `nil`.
    Nil,
    /// `^procedure (...)` / `^function (...): T` (ref 3.13.3, p. 3-50).
    RoutinePointer(Box<Signature>),
    /// The type of a string constant of `len` characters (`len` >= 2; a
    /// one-character string is a `char`, ref 3.6.3).
    StringLit {
        len: u32,
    },
    /// A pointer's target named before its declaration in the same type
    /// part (ref 3.13.1).  Replaced by the real type when the part ends.
    Forward {
        name: String,
    },
}

/// A routine pointer's signature (ref 3.13.3).  Parameter conventions of a
/// call through a pointer are decided like those of a routine heading.
#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    pub params: Vec<SigParam>,
    pub result: Option<TypeId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SigParam {
    pub mode: dpc_syntax::ast::ParamMode,
    pub univ: bool,
    pub ty: TypeId,
}

/// An unpacked record (ref 3.10).
#[derive(Clone, Debug, PartialEq)]
pub struct RecordType {
    /// The fixed part, in declaration order.
    pub fields: Vec<Field>,
    pub variant: Option<Box<VariantPart>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: TypeId,
    /// Byte offset from the start of the record.
    pub offset: u32,
    pub span: Span,
}

/// `case [tag:] type of ...` (ref 3.10.2).  Every arm starts at the offset
/// where the tag field (if any) ends.
#[derive(Clone, Debug, PartialEq)]
pub struct VariantPart {
    pub tag: Option<Field>,
    pub tag_type: TypeId,
    pub arms: Vec<VariantArm>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariantArm {
    /// Ordinal values of the arm's labels.
    pub labels: Vec<i64>,
    pub fields: Vec<Field>,
    pub variant: Option<Box<VariantPart>>,
}

impl RecordType {
    /// Look a field up by (folded) name, in the fixed part, the tag and
    /// every variant arm.  Field names are unique within a record.
    pub fn field(&self, name: &str) -> Option<&Field> {
        fn in_variant<'a>(v: &'a VariantPart, name: &str) -> Option<&'a Field> {
            if let Some(t) = &v.tag {
                if t.name == name {
                    return Some(t);
                }
            }
            for arm in &v.arms {
                if let Some(f) = arm.fields.iter().find(|f| f.name == name) {
                    return Some(f);
                }
                if let Some(sub) = &arm.variant {
                    if let Some(f) = in_variant(sub, name) {
                        return Some(f);
                    }
                }
            }
            None
        }
        self.fields
            .iter()
            .find(|f| f.name == name)
            .or_else(|| self.variant.as_deref().and_then(|v| in_variant(v, name)))
    }

    /// Every field (fixed, tag, variant arms) in declaration order.
    pub fn all_fields(&self) -> Vec<&Field> {
        fn walk<'a>(v: &'a VariantPart, out: &mut Vec<&'a Field>) {
            if let Some(t) = &v.tag {
                out.push(t);
            }
            for arm in &v.arms {
                out.extend(arm.fields.iter());
                if let Some(sub) = &arm.variant {
                    walk(sub, out);
                }
            }
        }
        let mut out: Vec<&Field> = self.fields.iter().collect();
        if let Some(v) = &self.variant {
            walk(v, &mut out);
        }
        out
    }
}

/// A type and its layout.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeInfo {
    pub kind: TypeKind,
    /// The first name the type was declared under, for diagnostics.
    pub name: Option<String>,
    /// Size in bytes.
    pub size: u32,
    /// Alignment of a freestanding object of this type (bytes).
    pub align: u32,
    /// Guaranteed default alignment as an unpacked-record field (bytes).
    pub field_align: u32,
    /// Natural alignment, used by the record allocation algorithm
    /// (ref 3.10.5.2 p. 3-23, 3.10.5.6 pp. 3-27..3-28).
    pub natural_align: u32,
}

/// Every type of a compilation unit.  [`TypeTable::default`] is empty (no
/// predeclared types); [`TypeTable::new`] is the real thing.
#[derive(Clone, Debug, Default)]
pub struct TypeTable {
    types: Vec<TypeInfo>,
    /// Ids of the predeclared types (valid after [`TypeTable::new`]).
    pub builtins: Builtins,
}

/// Ids of the predeclared types.
#[derive(Clone, Copy, Debug, Default)]
pub struct Builtins {
    pub error: TypeId,
    pub integer16: TypeId,
    pub integer32: TypeId,
    pub boolean: TypeId,
    pub char: TypeId,
    pub real: TypeId,
    pub double: TypeId,
    pub univ_ptr: TypeId,
    pub nil: TypeId,
}

/// Largest object size the checker accepts: frame offsets are `i32`.
/// Layout arithmetic saturates at `u32::MAX`, so an oversized type is
/// detected by comparing against this, never by overflowing.
pub const MAX_OBJECT_SIZE: u32 = i32::MAX as u32;

/// `n` rounded up to a multiple of `a`, saturating at `u32::MAX`.
pub fn round_up(n: u32, a: u32) -> u32 {
    if a <= 1 {
        n
    } else {
        (n as u64)
            .div_ceil(a as u64)
            .saturating_mul(a as u64)
            .min(u32::MAX as u64) as u32
    }
}

impl TypeTable {
    /// A table holding the predeclared types.  `pointer_size` comes from
    /// the target (CLAUDE.md 4.5).
    pub fn new(pointer_size: u32) -> TypeTable {
        let mut t = TypeTable {
            types: Vec::new(),
            builtins: Builtins::default(),
        };
        let b = Builtins {
            error: t.push_raw(TypeKind::Error, None, 0, 1, 1, 1),
            // ref 3.2.4 p. 3-5: 16-bit integers word aligned, 32-bit
            // integers longword aligned when freestanding; Table 3-2
            // p. 3-24: both word aligned as record fields.
            integer16: t.push_raw(TypeKind::Integer16, Some("integer16"), 2, 2, 2, 2),
            integer32: t.push_raw(TypeKind::Integer32, Some("integer32"), 4, 4, 2, 4),
            // ref 3.5.3 p. 3-11, 3.6.4 p. 3-13: byte aligned.
            boolean: t.push_raw(TypeKind::Boolean, Some("boolean"), 1, 1, 1, 1),
            char: t.push_raw(TypeKind::Char, Some("char"), 1, 1, 1, 1),
            // ref 3.3.4 p. 3-7 (freestanding), Table 3-2 (fields).
            real: t.push_raw(TypeKind::Real { size: 4 }, Some("real"), 4, 4, 2, 4),
            double: t.push_raw(TypeKind::Real { size: 8 }, Some("double"), 8, 8, 2, 8),
            // ref 3.13.5 p. 3-52: pointers on longword boundaries;
            // Table 3-2: word aligned as fields.
            univ_ptr: t.push_raw(
                TypeKind::UnivPtr,
                Some("univ_ptr"),
                pointer_size,
                pointer_size,
                2,
                pointer_size,
            ),
            nil: t.push_raw(
                TypeKind::Nil,
                Some("nil"),
                pointer_size,
                pointer_size,
                2,
                pointer_size,
            ),
        };
        t.builtins = b;
        t
    }

    fn push_raw(
        &mut self,
        kind: TypeKind,
        name: Option<&str>,
        size: u32,
        align: u32,
        field_align: u32,
        natural_align: u32,
    ) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(TypeInfo {
            kind,
            name: name.map(str::to_string),
            size,
            align,
            field_align,
            natural_align,
        });
        id
    }

    pub fn get(&self, id: TypeId) -> &TypeInfo {
        &self.types[id.0 as usize]
    }

    pub fn kind(&self, id: TypeId) -> &TypeKind {
        &self.get(id).kind
    }

    pub fn size(&self, id: TypeId) -> u32 {
        self.get(id).size
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// The type first declared under `name` (folded), if any.
    pub fn find_named(&self, name: &str) -> Option<TypeId> {
        self.types
            .iter()
            .position(|t| t.name.as_deref() == Some(name))
            .map(|i| TypeId(i as u32))
    }

    /// Give a type its first declared name (for diagnostics).
    pub fn set_name_if_none(&mut self, id: TypeId, name: &str) {
        let info = &mut self.types[id.0 as usize];
        if info.name.is_none() {
            info.name = Some(name.to_string());
        }
    }

    /// Replace the kind of a type in place (forward pointer fix-up).
    pub(crate) fn set_kind(&mut self, id: TypeId, kind: TypeKind) {
        self.types[id.0 as usize].kind = kind;
    }

    /// Add a type, computing its layout.  `pointer_size` is the target's.
    pub fn add(&mut self, kind: TypeKind, pointer_size: u32) -> TypeId {
        let (size, align, field_align, natural_align) = self.layout_of(&kind, pointer_size);
        let id = TypeId(self.types.len() as u32);
        self.types.push(TypeInfo {
            kind,
            name: None,
            size,
            align,
            field_align,
            natural_align,
        });
        id
    }

    /// Layout of a new type: (size, align, field_align, natural_align).
    fn layout_of(&self, kind: &TypeKind, pointer_size: u32) -> (u32, u32, u32, u32) {
        match kind {
            TypeKind::Error => (0, 1, 1, 1),
            TypeKind::Integer16 => (2, 2, 2, 2),
            TypeKind::Integer32 => (4, 4, 2, 4),
            TypeKind::Boolean | TypeKind::Char => (1, 1, 1, 1),
            TypeKind::Real { size } => (*size, *size, 2, *size),
            // ref 3.7.1 p. 3-13: one 16-bit word.
            TypeKind::Enum { .. } => (2, 2, 2, 2),
            // ref 3.8.1 p. 3-14: "The storage allocation for subrange
            // variables is the same as that for their base types."  An
            // unsigned subrange that fits 16 bits is a 16-bit object even
            // when its upper bound exceeds 32767: 5.3.4.1 p. 5-13, "pinteger
            // = 0..65535 ... Even though pinteger and integer are both
            // 16-bit integers"; Table 7-2 p. 7-30, "0..65335 [sic]" is
            // Domain/C's "unsigned short"; 3.11.2 p. 3-44 / 3.11.4.1
            // p. 3-46, the varying-string length field `length: 0..65535`
            // is "a 2-byte length field".  The checker gives such a
            // subrange an integer32 host, because its values do not fit
            // integer16, so its storage is decided here, not by the host.
            // (How the original compiler did arithmetic on such values is
            // not yet observed.)  3.4 p. 3-10: 32 bits only once 31 bits
            // are needed.
            TypeKind::Subrange { base, lo, hi } => {
                if *lo >= 0 && *hi <= 65535 && matches!(self.kind(*base), TypeKind::Integer32) {
                    (2, 2, 2, 2)
                } else {
                    let b = self.get(*base);
                    (b.size, b.align, b.field_align, b.natural_align)
                }
            }
            TypeKind::Set { elem } => {
                // ref 3.9.3 p. 3-16: one bit per element, rounded up to a
                // multiple of 16 bits; the high end of the base type's
                // ordinal range decides ("80..170" is 11 words, "'a'..'z'"
                // 8 words, p. 3-17).
                let hi = self.ordinal_range(*elem).map_or(0, |(_, hi)| hi.max(0)) as u32;
                let words = (hi + 1).div_ceil(16);
                let size = words * 2;
                // Freestanding and natural alignment of sets are not
                // stated by the reference; word is the conservative
                // choice (sets are word strings, 3.9.3).  Uncertain.
                (size, 2, 2, 2)
            }
            TypeKind::Array { index, elem, .. } => {
                let n = self
                    .ordinal_range(*index)
                    .map_or(0, |(lo, hi)| (hi - lo + 1).max(0)) as u64;
                let e = self.get(*elem);
                let mut size = (n * e.size as u64).min(u32::MAX as u64) as u32;
                // ref 3.11.4.1 p. 3-46: an array of an odd number of
                // booleans gets a pad byte, so its size is even.  (Char
                // arrays get none.)
                if matches!(self.kind(*elem), TypeKind::Boolean) && size % 2 == 1 {
                    size = size.saturating_add(1);
                }
                let fa = if size > 1 { 2 } else { 1 };
                // Freestanding alignment of arrays is not stated by the
                // reference; the element's, at least word for a
                // multi-byte object, is assumed.  Uncertain.
                let align = e.align.max(fa);
                (size, align, fa, e.natural_align)
            }
            TypeKind::Varying { max, .. } => {
                // ref 3.11.4.1 p. 3-46: a 2-byte length, the body with room
                // for a trailing null, padding: "add either 5 or 6 bytes
                // to the number of bytes specified as the maximum length
                // ... such that the total number of bytes is an even
                // number" (v2 = 8, v7 = 12, v8 = 14 bytes).
                let size = max + if max % 2 == 1 { 5 } else { 6 };
                (size, 2, 2, 2)
            }
            TypeKind::Record(r) => self.record_layout(r),
            TypeKind::Pointer { .. }
            | TypeKind::UnivPtr
            | TypeKind::Nil
            | TypeKind::RoutinePointer(_) => (pointer_size, pointer_size, 2, pointer_size),
            TypeKind::StringLit { len } => (*len, 1, if *len > 1 { 2 } else { 1 }, 1),
            TypeKind::Forward { .. } => (0, 1, 1, 1),
        }
    }

    /// Size and alignments of a record whose field offsets are already
    /// set (see [`TypeTable::layout_fields`]).
    fn record_layout(&self, r: &RecordType) -> (u32, u32, u32, u32) {
        let mut end = 0u32;
        // ref 3.10.5.6 p. 3-27: "Domain Pascal always allocates a record on
        // at least a word boundary.  It allocates a record on a larger
        // boundary if that larger allocation produces natural alignment
        // for any of the record's fields": lay out from address 0, find
        // the largest naturally aligned field, allocate on its natural
        // alignment.
        let mut alloc = 2u32;
        for f in r.all_fields() {
            let fi = self.get(f.ty);
            end = end.max(f.offset.saturating_add(fi.size));
            if fi.natural_align > 0 && f.offset % fi.natural_align == 0 {
                alloc = alloc.max(fi.natural_align);
            }
        }
        // ref 3.10.5.5 p. 3-26: "The size of a record must be an even
        // number of bytes ... the smallest possible record is 2 bytes."
        let size = round_up(end, 2).max(2);
        // A record field is word aligned: "The default alignment of the
        // beginning of a record is at least word aligned" (p. 3-26).
        (size, alloc, 2, alloc)
    }

    /// Assign offsets to a list of fields starting at `start`, in the
    /// default (word) alignment environment.  Returns the end offset.
    ///
    /// ref 3.10.5.5 p. 3-26: "the compiler lays out record fields based on
    /// their guaranteed default alignment.  Thus, all objects longer than a
    /// byte are aligned on word boundaries.  Objects which are chars and
    /// booleans may be aligned on byte boundaries."  Checked against
    /// Figures 3-8..3-13 (PDF pp. 82-86).
    pub fn layout_fields(&self, fields: &mut [Field], start: u32) -> u32 {
        let mut off = start;
        for f in fields {
            let fi = self.get(f.ty);
            off = round_up(off, fi.field_align);
            f.offset = off;
            off = off.saturating_add(fi.size);
        }
        off
    }

    /// The ordinal host type of an ordinal type: subranges map to their
    /// base; non-ordinal types map to `None`.
    pub fn ordinal_base(&self, id: TypeId) -> Option<TypeId> {
        match self.kind(id) {
            TypeKind::Integer16
            | TypeKind::Integer32
            | TypeKind::Boolean
            | TypeKind::Char
            | TypeKind::Enum { .. } => Some(id),
            TypeKind::Subrange { base, .. } => Some(*base),
            _ => None,
        }
    }

    /// Smallest and largest ordinal of an ordinal type.
    pub fn ordinal_range(&self, id: TypeId) -> Option<(i64, i64)> {
        match self.kind(id) {
            TypeKind::Integer16 => Some((-32768, 32767)),
            TypeKind::Integer32 => Some((i32::MIN as i64, i32::MAX as i64)),
            TypeKind::Boolean => Some((0, 1)),
            TypeKind::Char => Some((0, 255)),
            TypeKind::Enum { consts } => Some((0, consts.len() as i64 - 1)),
            TypeKind::Subrange { lo, hi, .. } => Some((*lo, *hi)),
            _ => None,
        }
    }

    pub fn is_ordinal(&self, id: TypeId) -> bool {
        self.ordinal_base(id).is_some()
    }

    /// Integer16, integer32 or a subrange of them.
    pub fn is_integer(&self, id: TypeId) -> bool {
        matches!(
            self.ordinal_base(id).map(|b| self.kind(b)),
            Some(TypeKind::Integer16 | TypeKind::Integer32)
        )
    }

    pub fn is_boolean(&self, id: TypeId) -> bool {
        matches!(
            self.ordinal_base(id).map(|b| self.kind(b)),
            Some(TypeKind::Boolean)
        )
    }

    pub fn is_char(&self, id: TypeId) -> bool {
        matches!(
            self.ordinal_base(id).map(|b| self.kind(b)),
            Some(TypeKind::Char)
        )
    }

    pub fn is_error(&self, id: TypeId) -> bool {
        matches!(self.kind(id), TypeKind::Error)
    }

    /// Pointer-like: `^T`, `univ_ptr`, `nil`, routine pointers.
    pub fn is_pointer(&self, id: TypeId) -> bool {
        matches!(
            self.kind(id),
            TypeKind::Pointer { .. }
                | TypeKind::UnivPtr
                | TypeKind::Nil
                | TypeKind::RoutinePointer(_)
        )
    }

    /// An array of `char` (packed or not), ref 4-25 "Array Operations".
    pub fn char_array_len(&self, id: TypeId) -> Option<u32> {
        match self.kind(id) {
            TypeKind::Array { index, elem, .. } if self.is_char(*elem) => {
                let (lo, hi) = self.ordinal_range(*index)?;
                Some((hi - lo + 1).max(0) as u32)
            }
            _ => None,
        }
    }

    /// Location of set member `k` in a set of type `set_ty`: (byte offset
    /// from the set's first byte, bit number in that byte, 0 = LSB).
    ///
    /// ref 3.9.3 Figure 3-6 p. 3-16 (checked in the PDF): the set is a
    /// big-endian bit string, member 0 is bit 0 of the last word, so member
    /// `k` is in byte `size-1 - k/8`, bit `k mod 8`.  Image: MST_$ALLOC_ASID
    /// 0xE42D5C computes `(63-k)>>3` for a 64-bit set (codegen-patterns 5.1).
    pub fn set_bit_location(&self, set_ty: TypeId, k: u32) -> (u32, u32) {
        let size = self.size(set_ty);
        (size - 1 - k / 8, k % 8)
    }

    /// A readable name for diagnostics.
    pub fn display(&self, id: TypeId) -> String {
        let info = self.get(id);
        if let Some(n) = &info.name {
            return n.clone();
        }
        match &info.kind {
            TypeKind::Error => "<error>".into(),
            TypeKind::Integer16 => "integer16".into(),
            TypeKind::Integer32 => "integer32".into(),
            TypeKind::Boolean => "boolean".into(),
            TypeKind::Char => "char".into(),
            TypeKind::Real { size } => format!("real{}", size * 8),
            TypeKind::Enum { consts } => format!("({})", consts.join(", ")),
            TypeKind::Subrange { lo, hi, .. } => format!("{lo}..{hi}"),
            TypeKind::Set { elem } => format!("set of {}", self.display(*elem)),
            TypeKind::Array { index, elem, .. } => {
                format!("array[{}] of {}", self.display(*index), self.display(*elem))
            }
            TypeKind::Varying { max, elem } => {
                format!("varying[{max}] of {}", self.display(*elem))
            }
            TypeKind::Record(_) => "record".into(),
            TypeKind::Pointer { target } => format!("^{}", self.display(*target)),
            TypeKind::UnivPtr => "univ_ptr".into(),
            TypeKind::Nil => "nil".into(),
            TypeKind::RoutinePointer(_) => "routine pointer".into(),
            TypeKind::StringLit { len } => format!("string of {len} characters"),
            TypeKind::Forward { name } => name.clone(),
        }
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}
