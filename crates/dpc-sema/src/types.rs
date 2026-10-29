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
//! Records are laid out in the word-alignment environment (the default,
//! ref 4.7 p. 4-57) or the natural one (`%natural_alignment`, `aligned
//! record`, ref 3.10.6), with the alignment attributes `aligned(n)` and
//! `natural` (ref 3.15.6) overriding either ([`TypeTable::layout_fields`],
//! [`TypeTable::attributed`]).  Packed records follow ref 3.10.7 (see
//! [`TypeTable::packed_field_bits`] and [`TypeTable::layout_packed_fields`]).
//!
//! Attributes (ref 3.15) make a new type ([`TypeTable::attributed`]) that
//! is the same type as the one it is made from for compatibility
//! ([`TypeInfo::canon`]) and carries [`TypeAttrs`].

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
/// call through a pointer are decided like those of a routine heading,
/// from the options written after the type (`val_param`; ref p. 4-146,
/// Appendix B messages 181, 182).
#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    pub params: Vec<SigParam>,
    pub result: Option<TypeId>,
    /// The options given with the type (only `val_param` is accepted).
    pub options: crate::conv::RoutineOptions,
    /// Pad the caller pushes before the arguments, as
    /// [`crate::Routine::arg_pad`] (codegen-patterns 1.4: MAC_OS_$OPEN
    /// 0xE0B3DC `subq.l #2,SP` before a call through a pointer).
    pub arg_pad: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SigParam {
    pub mode: dpc_syntax::ast::ParamMode,
    pub univ: bool,
    pub ty: TypeId,
    /// The convention, from the mode, the type and the signature's options
    /// (ref 5.2 Table 5-1).
    pub conv: crate::conv::ParamConv,
}

/// A record (ref 3.10), unpacked or packed (3.10.7).
#[derive(Clone, Debug, PartialEq)]
pub struct RecordType {
    /// The fixed part, in declaration order.
    pub fields: Vec<Field>,
    pub variant: Option<Box<VariantPart>>,
    /// `packed record` (ref 3.10.3, 3.10.7): every field has [`Field::bits`].
    pub packed: bool,
    /// Laid out by the natural alignment rules: declared under
    /// `%natural_alignment` (ref 4.7 p. 4-57, 3.15.6.3 p. 3-70) or as an
    /// `aligned record` (ref 3.10.6 p. 3-33).  Otherwise the word
    /// alignment rules (the default, and `unaligned record`, p. 3-34).
    pub natural: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: TypeId,
    /// Byte offset from the start of the record (of a packed record's
    /// field: of the byte its first bit is in).
    pub offset: u32,
    pub span: Span,
    /// The bits a field of a packed record occupies (ref 3.10.7); `None`
    /// in an unpacked record.
    pub bits: Option<BitPos>,
}

/// The bits of a packed record's field (ref 3.10.7, p. 3-35: "Pascal
/// allocates fields left to right within bytes and then by increasing
/// byte address"): `width` bits starting at bit `bit` of the record,
/// counted from the most significant bit of its first byte.  So bit `b`
/// is bit `7 - b % 8` (0 = least significant) of byte `b / 8`: the record
/// is a big-endian bit string (Figure 3-17, p. 3-37, checked in the PDF;
/// image: `status_$t.fail`, bit 0, is `bset #7` on the record's first
/// byte at PROC2_$RESUME 0xE41444).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitPos {
    pub bit: u32,
    pub width: u32,
}

impl BitPos {
    /// The field fills whole bytes from a byte boundary: `Some(bytes)`.
    pub fn whole_bytes(self) -> Option<u32> {
        (self.bit % 8 == 0 && self.width % 8 == 0).then_some(self.width / 8)
    }

    /// The field is a whole object of `size` bytes at an address an
    /// object of that size can be accessed at: a byte anywhere, anything
    /// longer at an even address (MC68000/68010: word and longword
    /// accesses need an even address).  A packed record is word aligned
    /// (ref 3.10.7: "always starts the first field of a packed record on a
    /// word boundary"), so the parity of the field's byte offset is that
    /// of its address.  Such a field is accessed like an unpacked one; any
    /// other is a bit field.
    pub fn is_whole_object(self, size: u32) -> bool {
        self.whole_bytes() == Some(size) && (size == 1 || (self.bit / 8) % 2 == 0)
    }
}

/// How an element of a packed array is stored ([`TypeTable::packed_array_elem`];
/// ref 3.11.4.2 Table 3-5, p. 3-47).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackedElem {
    /// As in an array that is not packed.
    Unpacked,
    /// One byte: an integer subrange of 5 to 8 bits ("byte integers", ref
    /// 3.11.3 p. 3-45), stored like a `[byte]` subrange (ref 3.15.5).
    Byte,
    /// `n` bits, n = 1, 2 or 4, from the most significant bit (ref 3.15.5
    /// p. 3-62).
    Bits(u32),
}

/// How a packed record's field is aligned (ref 3.10.7 Table 3-3,
/// p. 3-36).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackAlign {
    /// "bit aligned": at the next free bit, unless the field would cross
    /// two or more 16-bit boundaries (p. 3-35).
    Bit,
    /// "byte aligned" (char, subrange of char).
    Byte,
    /// "word aligned" (integer, integer32, reals, pointers, structured
    /// types other than sets).
    Word,
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

/// The attributes of a type (ref 3.15 pp. 3-53..3-77).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TypeAttrs {
    /// `volatile` (ref 3.15.1 p. 3-56), also implied by `atomic` and
    /// `device`: every reference is made as written.
    pub volatile: bool,
    /// `atomic` (ref 3.15.2 p. 3-56): "Implies volatile.  Also perform
    /// updates with single instruction when possible" (Table 3-6).
    pub atomic: bool,
    /// `device`, `device(read)`, `device(write)` (ref 3.15.3 pp.
    /// 3-57..3-59): implies volatile; no merged references, no
    /// "gratuitous read-modify-write references".
    pub device: Option<DeviceAccess>,
    /// The alignment specification in bytes: `aligned(n)` is 2**n,
    /// `natural` the type's natural alignment (ref 3.15.6.1 p. 3-63); for
    /// a record also the largest specification of its fields, which it
    /// inherits (ref 3.15.6.2 pp. 3-67..3-68), or its alignment when it
    /// is laid out by the natural rules (3.15.6.3 p. 3-70).  `None`: the
    /// default rules of the alignment environment apply.
    pub align: Option<u32>,
    /// A size attribute's storage in bits (`[byte]` is 8, `[word]` 16,
    /// `[long]` 32; ref 3.15.5 pp. 3-60..3-62), as written.  The type's
    /// [`TypeInfo::size`] is the size it gives ([`TypeTable::sized`]).
    pub size_bits: Option<u32>,
}

/// Which accesses a `device` object allows (ref 3.15.3 p. 3-59:
/// `device(read)`, `device(write)`; `device` and `device(read, write)`
/// allow both).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceAccess {
    pub read: bool,
    pub write: bool,
}

impl TypeAttrs {
    /// Every reference is made as written (ref 3.15.1-3.15.3).
    pub fn is_volatile(&self) -> bool {
        self.volatile || self.atomic || self.device.is_some()
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
    /// Attributes (ref 3.15).
    pub attrs: TypeAttrs,
    /// The alignment attribute given to this type itself (`aligned(n)`,
    /// `natural`; ref 3.15.6.1 p. 3-63), through [`TypeTable::attributed`],
    /// in bytes; not the specification a record inherits from its fields
    /// ([`TypeAttrs::align`], ref 3.15.6.2 pp. 3-67..3-68).  A record is
    /// always allocated on at least a word boundary (ref 3.10.5.6), so only
    /// an attribute of its own lets it lie at an odd address (bead
    /// domain-pascal-epjn).
    pub align_attr: Option<u32>,
    /// The type this one is for compatibility: itself, or for a type
    /// made by attributes ([`TypeTable::attributed`]) the type the
    /// attributes were given to.  ref 3.15.7 p. 3-76: `semaphore =
    /// [VOLATILE] integer` is still an integer.
    pub canon: TypeId,
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
            attrs: TypeAttrs::default(),
            align_attr: None,
            canon: id,
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
        let attrs = self.inherited_attrs(&kind);
        let id = TypeId(self.types.len() as u32);
        self.types.push(TypeInfo {
            kind,
            name: None,
            size,
            align,
            field_align,
            natural_align,
            attrs,
            align_attr: None,
            canon: id,
        });
        id
    }

    /// The attributes a new type inherits from its parts (ref 3.15.7
    /// pp. 3-76..3-77): an array its element's `volatile` and `device`
    /// (`resource = array[1..10] of semaphore` "inherits the volatile
    /// attribute from semaphore"); a record `device` from any field ("if
    /// you designate any one field within a record as having the device
    /// attribute, the entire record itself inherits the device attribute",
    /// but not `volatile`: "the entire record is not considered volatile
    /// just because one field is declared that way"), and the largest
    /// alignment specification of its fields (3.15.6.2 p. 3-67: "a record
    /// inherits the largest alignment specification of its fields"), or
    /// under the natural rules its alignment (3.15.6.3 p. 3-70).
    /// "Pointer types do not inherit the device or volatile attributes of
    /// their base type" (p. 3-77).
    fn inherited_attrs(&self, kind: &TypeKind) -> TypeAttrs {
        match kind {
            TypeKind::Array { elem, .. } => {
                let e = self.get(*elem).attrs;
                TypeAttrs {
                    volatile: e.is_volatile(),
                    atomic: false,
                    device: e.device,
                    align: None,
                    size_bits: None,
                }
            }
            TypeKind::Record(r) => {
                let mut a = TypeAttrs::default();
                for f in r.all_fields() {
                    let fa = self.get(f.ty).attrs;
                    if let Some(d) = fa.device {
                        a.device = Some(match a.device {
                            Some(x) => DeviceAccess {
                                read: x.read || d.read,
                                write: x.write || d.write,
                            },
                            None => d,
                        });
                    }
                }
                if !r.packed {
                    a.align = self.record_spec(r);
                }
                a
            }
            _ => TypeAttrs::default(),
        }
    }

    /// The alignment specification an unpacked record carries
    /// ([`TypeAttrs::align`]): under the natural rules its alignment, the
    /// largest of its fields' (3.15.6.3 p. 3-70, Figure 3-25); otherwise
    /// the largest explicit specification of its fields (3.15.6.2 p.
    /// 3-67, Figures 3-21, 3-22), if any.
    fn record_spec(&self, r: &RecordType) -> Option<u32> {
        let mut spec: Option<u32> = None;
        for f in r.all_fields() {
            let fi = self.get(f.ty);
            let a = match fi.attrs.align {
                Some(a) => a,
                None if r.natural => fi.natural_align,
                None => continue,
            };
            spec = Some(spec.map_or(a, |s| s.max(a)));
        }
        spec
    }

    /// A new type: `base` with attributes `attrs` added (ref 3.15).  It is
    /// `base` for compatibility ([`TypeInfo::canon`]).  `attrs.align`
    /// replaces `base`'s alignment (ref 3.15.6.2 p. 3-64: the attributes
    /// "override the default alignment rules ... as well as the alignment
    /// rules specified by any compiler alignment directive"): a freestanding
    /// object, a field, and the record allocation algorithm (3.10.5.6) all
    /// take it.  A record's size is rounded up to a multiple of it
    /// (Figure 3-21 p. 3-67: `S = [ALIGNED(2)] record a: integer32; b:
    /// integer16 end` is 8 bytes; Figure 3-26 p. 3-71: `[ALIGNED(1)]` on a
    /// record laid out by the natural rules removes its final word of
    /// padding), and stays a multiple of 2 (ref 3.15.6.3 p. 3-70:
    /// "Records created by Domain Pascal are always 2-byte multiples").
    /// Other types keep their size.
    pub fn attributed(&mut self, base: TypeId, attrs: TypeAttrs) -> TypeId {
        let b = self.get(base).clone();
        let mut info = TypeInfo {
            name: None,
            attrs: TypeAttrs {
                volatile: b.attrs.volatile || attrs.volatile,
                atomic: b.attrs.atomic || attrs.atomic,
                device: match (b.attrs.device, attrs.device) {
                    (Some(x), Some(y)) => Some(DeviceAccess {
                        read: x.read && y.read,
                        write: x.write && y.write,
                    }),
                    (x, y) => x.or(y),
                },
                align: attrs.align.or(b.attrs.align),
                size_bits: attrs.size_bits.or(b.attrs.size_bits),
            },
            align_attr: attrs.align.or(b.align_attr),
            ..b
        };
        if let Some(a) = attrs.align {
            if let TypeKind::Record(r) = &info.kind {
                let a2 = a.max(2);
                info.size = round_up(self.record_end(r), a2).max(2);
                info.align = a2;
                info.field_align = a2;
                info.natural_align = a2;
            } else {
                info.align = a;
                info.field_align = a;
                info.natural_align = a;
            }
        }
        let id = TypeId(self.types.len() as u32);
        self.types.push(info);
        id
    }

    /// A new type: `base` with a size attribute of `bits` bits (ref
    /// 3.15.5, pp. 3-60..3-62), which the caller has checked against
    /// Table 3-7 (p. 3-61).  It is `base` for compatibility
    /// ([`TypeInfo::canon`]), as an alignment attribute's type is.  The
    /// object takes `bits / 8` bytes ("Size attributes specify the amount
    /// of storage to be reserved" for variables, formal parameters,
    /// function results, record fields and array components, p. 3-60).
    /// Only a one-byte object changes the layout: an integer subrange
    /// whose default storage is a word (`[byte] 0..255`, p. 3-62), or a
    /// set of at most 8 elements (`[byte] set of 0..7`, the owner's
    /// ruling, reference-clarifications "A size attribute on a set type":
    /// member k at bit k, [`Self::set_bit_location`] of a 1-byte set; at
    /// the odd offsets 0x19, 0x1B, 0x03, 0x07 of ACL_'s and APP_'s records
    /// in the image, ACL_$MIN_RIGHTS 0xE46934 `btst.b #4,(-0x13,A6)`,
    /// APP_$RECEIVE 0xE00904 `btst.b #3,(7,A3)`).  It is
    /// byte aligned freestanding, as a field and naturally, like `char`
    /// and `boolean`, the reference's other one-byte types (Table 3-2,
    /// p. 3-24, gives byte alignment for both; p. 9-41, message 305: "A
    /// value is naturally aligned if it begins at an address that is a
    /// multiple of its size in bytes").  **Uncertain:** the alignment of
    /// a sized type is not stated; the image's TPAD packet record has
    /// such bytes at consecutive offsets 0xA..0xF, read with `move.b`
    /// (TPAD_$DATA 0xE691DC `move.b (10,A1)`, 0xE691F0 `and.b (11,A1)`,
    /// 0xE69228 `move.b (12,A1)`; their Pascal types are inferred by
    /// `../source`, `tpad/tpad.ins.pas`).
    pub fn sized(&mut self, base: TypeId, bits: u32) -> TypeId {
        let b = self.get(base).clone();
        let size = bits / 8;
        let mut info = TypeInfo {
            name: None,
            attrs: TypeAttrs {
                size_bits: Some(bits),
                ..b.attrs
            },
            ..b
        };
        if size != info.size {
            debug_assert_eq!(size, 1, "only a one-byte size changes the layout");
            info.size = size;
            info.align = 1;
            info.field_align = 1;
            info.natural_align = 1;
        }
        let id = TypeId(self.types.len() as u32);
        self.types.push(info);
        id
    }

    /// The bits an integer subrange `lo..hi` needs: its "extreme values"
    /// (ref 3.10.7 Table 3-3, p. 3-36; Table 3-7, p. 3-61, "Number of
    /// bits needed to store subrange"), two's complement when `lo` is
    /// negative.
    pub fn subrange_bits(lo: i64, hi: i64) -> u32 {
        fn bits_for(v: u64) -> u32 {
            (64 - v.leading_zeros()).max(1)
        }
        if lo >= 0 {
            bits_for(hi as u64)
        } else {
            1 + bits_for(hi.max(-lo - 1).max(0) as u64)
        }
    }

    /// The type `id` is for compatibility ([`TypeInfo::canon`]).
    pub fn canon(&self, id: TypeId) -> TypeId {
        self.get(id).canon
    }

    /// The attributes of type `id`.
    pub fn attrs(&self, id: TypeId) -> TypeAttrs {
        self.get(id).attrs
    }

    /// Whether an object of type `id` has a part that is volatile (or
    /// atomic, or device), itself included.
    pub fn has_volatile_part(&self, id: TypeId) -> bool {
        let info = self.get(id);
        if info.attrs.is_volatile() {
            return true;
        }
        match &info.kind {
            TypeKind::Record(r) => r.all_fields().iter().any(|f| self.has_volatile_part(f.ty)),
            TypeKind::Array { elem, .. } => self.has_volatile_part(*elem),
            _ => false,
        }
    }

    /// The end of a record's last byte used by a field (bytes from its
    /// start; a packed record's in whole 16-bit words, ref 3.10.7).
    fn record_end(&self, r: &RecordType) -> u32 {
        let mut end = 0u32;
        for f in r.all_fields() {
            end = end.max(match f.bits {
                Some(b) => round_up(b.bit.saturating_add(b.width), 16) / 8,
                None => f.offset.saturating_add(self.size(f.ty)),
            });
        }
        end
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
            TypeKind::Array {
                index,
                elem,
                packed,
            } => {
                let n = self
                    .ordinal_range(*index)
                    .map_or(0, |(lo, hi)| (hi - lo + 1).max(0)) as u64;
                let e = self.get(*elem);
                let bits = match packed.then(|| self.packed_array_elem(*elem)) {
                    Some(Ok(PackedElem::Bits(b))) => Some(b),
                    _ => None,
                };
                let mut size = match bits {
                    // ref 3.11.4.2 p. 3-47 and 3.15.5 p. 3-62: the elements'
                    // bits, in whole bytes ("2 bytes would be used to store
                    // the entire array1", 3 elements of 4 bits; "3 bytes"
                    // for 6 elements of 4 bits).
                    Some(b) => (n * u64::from(b)).div_ceil(8).min(u32::MAX as u64) as u32,
                    None => (n * e.size as u64).min(u32::MAX as u64) as u32,
                };
                // ref 3.11.4.1 p. 3-46: an array of an odd number of
                // booleans gets a pad byte, so its size is even.  (Char
                // arrays get none.)  A packed one has bits (3.11.4.2).
                if bits.is_none() && matches!(self.kind(*elem), TypeKind::Boolean) && size % 2 == 1
                {
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

    /// How an element of type `elem` is stored in a packed array (ref
    /// 3.11.4.2 Table 3-5 p. 3-47, read in the PDF; the OCR scrambles the
    /// table), or why it is not supported:
    ///
    /// | type of element | space |
    /// |---|---|
    /// | subranges of integers, enumerated, subrange of enumerated | "Exact number of bits required; bit aligned" |
    /// | integer, integer16 | 16 bits; word aligned |
    /// | boolean | "1 bit; bit aligned" |
    /// | real, integer32, pointer | 32 bits; word aligned |
    /// | double | 64 bits; word aligned |
    /// | character, subrange of character | 8 bits; byte aligned |
    /// | character array | "Exact number of bytes required; byte aligned" |
    /// | record, array of non-characters | "Exact number of words required; word aligned" |
    /// | set | "Exact number of bits required; bit aligned" (footnotes 1, 2) |
    ///
    /// Footnote 1, and the text below the table for every element: "if
    /// the element size is less than 16 bits, then the element is padded
    /// up to the nearest power of 2 bits" (`packed array [1..3] of 0..7`:
    /// 4 bits each, 2 bytes in all).  3.15.5 p. 3-62 (PDF p. 117): "Domain
    /// Pascal supports arrays of 1-, 2-, and 4-bit elements.  The order of
    /// elements within words is from most significant bit to least
    /// significant bit."  Image (codegen-patterns 5.2, "Packed arrays of
    /// booleans"): PCHIST's `packed array[1..64] of boolean`, 8 bytes at
    /// +0x18 of its WIRED_DATA block, element p in bit 7 - (p - 1) mod 8
    /// of byte (p - 1) div 8 (PCHIST_$COUNT 0xE1A156..0xE1A16C).
    ///
    /// The bits an ordinal needs are those of a packed record's field
    /// ([`TypeTable::subrange_bits`]: two's complement for a negative low
    /// bound).  So:
    ///
    /// - 1 to 4 bits: [`PackedElem::Bits`] of 1, 2 or 4;
    /// - an integer subrange of 5 to 8 bits: [`PackedElem::Byte`] (3.11.3
    ///   p. 3-45: packed arrays "store sequences of characters, byte
    ///   integers, and boolean variables");
    /// - 9 to 16 bits, and the 16-, 32- and 64-bit types, character types
    ///   and arrays, records: the same as unpacked ([`PackedElem::Unpacked`]);
    ///   an array of non-characters of an odd number of bytes would be
    ///   padded to words ("exact number of words") and is refused;
    /// - refused: an enumeration of 5 to 8 bits (a byte enumeration is not
    ///   implemented), a subrange of 17 to 31 bits (footnote 2, "If the
    ///   element size is greater than 16 bits, then the elements are word
    ///   aligned", does not say how many bits each element then takes),
    ///   sets of up to 4 members (1, 2 or 4 bits: their bit order is not
    ///   settled; 5 to 8 members are a byte each, more a word or whole
    ///   words as unpacked), varying strings (not in the table), and a
    ///   type with a
    ///   size attribute that changes its size (ref 3.15.5: `[bit (4)]
    ///   0..1` makes 4-bit elements, `dpc` has no bit sizes).
    pub fn packed_array_elem(&self, elem: TypeId) -> Result<PackedElem, String> {
        fn bits_for(v: u64) -> u32 {
            (64 - v.leading_zeros()).max(1)
        }
        let info = self.get(elem);
        if let Some(b) = info.attrs.size_bits {
            return if b == 8 * info.size {
                Ok(PackedElem::Unpacked)
            } else {
                Err("a packed array of elements with a bit size attribute".into())
            };
        }
        let (bits, byte_ok) = match &info.kind {
            TypeKind::Boolean => (1, false),
            TypeKind::Enum { consts } => (bits_for(consts.len().saturating_sub(1) as u64), false),
            TypeKind::Subrange { base, lo, hi } => match self.kind(*base) {
                TypeKind::Char => return Ok(PackedElem::Unpacked),
                TypeKind::Boolean | TypeKind::Enum { .. } => (bits_for((*hi).max(0) as u64), false),
                _ => {
                    let n = Self::subrange_bits(*lo, *hi);
                    // ref 3.4 p. 3-10: an unsigned subrange of 31 bits is
                    // allocated 32 (as in packed records, Table 3-3).
                    (if n == 31 && *lo >= 0 { 32 } else { n }, true)
                }
            },
            TypeKind::Set { elem: e } => {
                // Table 3-5 "Set: Exact number of bits required; bit
                // aligned", one bit per member up to the base type's high
                // end (3.9.3; the size computation above), padded to a
                // power of 2 below 16 bits (footnote 1): 5 to 8 members a
                // byte, the one-byte set of `[byte] set of 0..7` (SAU9
                // SCSI_$PPN_OWNERS, `packed array[16#10000..16#1FFFF] of
                // set of 0..7`, one byte per page: scsi_wire_page
                // 0x3C4A67B8 reads it with `move.b`; ../source
                // docs/pascal-translation-notes.md gap 300; bead
                // domain-pascal-nb0x); 9 to 16 members a word, 17 or
                // more whole words, word aligned (footnote 2): as
                // unpacked.  Up to 4 members (1, 2 or 4 bits): where the
                // bits of such an element lie is not settled: refused.
                let bits = self.ordinal_range(*e).map_or(0, |(_, hi)| hi.max(0)) as u32 + 1;
                return match bits {
                    1..=4 => Err(format!(
                        "a packed array of sets of {bits} bits (ref 3.11.4.2 Table 3-5 \
                         footnote 1: {}-bit elements, their bit order not settled)",
                        bits.next_power_of_two()
                    )),
                    5..=8 => Ok(PackedElem::Byte),
                    _ => Ok(PackedElem::Unpacked),
                };
            }
            TypeKind::Varying { .. } => {
                return Err(
                    "a packed array of varying strings (not in ref 3.11.4.2 Table 3-5)".into(),
                )
            }
            TypeKind::Array { elem: e, .. } => {
                return if self.is_char(*e) || info.size % 2 == 0 {
                    Ok(PackedElem::Unpacked)
                } else {
                    Err(
                        "a packed array of arrays of an odd number of bytes (\"Exact number of \
                         words required\", ref 3.11.4.2 Table 3-5: padding not settled)"
                            .into(),
                    )
                };
            }
            _ => return Ok(PackedElem::Unpacked),
        };
        match bits {
            1 => Ok(PackedElem::Bits(1)),
            2 => Ok(PackedElem::Bits(2)),
            3 | 4 => Ok(PackedElem::Bits(4)),
            5..=8 if byte_ok => Ok(PackedElem::Byte),
            5..=8 => {
                Err("a packed array of a one-byte enumeration (ref 3.11.4.2 Table 3-5)".into())
            }
            9..=16 | 32 => Ok(PackedElem::Unpacked),
            _ => Err(format!(
                "a packed array of {bits}-bit elements (ref 3.11.4.2 Table 3-5 footnote 2: \"word \
                 aligned\", their size not stated)"
            )),
        }
    }

    /// The bits of an element of array type `array` when it is a packed
    /// array of 1-, 2- or 4-bit elements ([`TypeTable::packed_array_elem`]):
    /// element `k` is bits `(k - lo) * n .. (k - lo) * n + n - 1` of the
    /// array, counted from the most significant bit of its first byte.
    pub fn packed_elem_bits(&self, array: TypeId) -> Option<u32> {
        match self.kind(array) {
            TypeKind::Array {
                elem, packed: true, ..
            } => match self.packed_array_elem(*elem) {
                Ok(PackedElem::Bits(b)) => Some(b),
                _ => None,
            },
            _ => None,
        }
    }

    /// Size and alignments of a record whose field offsets are already
    /// set (see [`TypeTable::layout_fields`]).
    fn record_layout(&self, r: &RecordType) -> (u32, u32, u32, u32) {
        if r.packed {
            return self.packed_record_layout(r);
        }
        let mut end = 0u32;
        // ref 3.10.5.6 p. 3-27: "Domain Pascal always allocates a record on
        // at least a word boundary.  It allocates a record on a larger
        // boundary if that larger allocation produces natural alignment
        // for any of the record's fields": lay out from address 0, find
        // the largest naturally aligned field, allocate on its natural
        // alignment.  A field with an alignment attribute counts with that
        // alignment ([`TypeTable::attributed`] makes it its natural one).
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
        // A record that carries an alignment specification (inherited
        // from its fields, or its natural alignment) is rounded up to a
        // multiple of it and aligned on it, as a field too (Figure 3-21
        // p. 3-67, 8 bytes; Figure 3-22 p. 3-68: `y: S` at offset 4 and
        // 2 bytes of padding at the end of S1; Figure 3-25 p. 3-71, 12
        // bytes).
        let spec = self.record_spec(r).unwrap_or(2).max(2);
        let size = round_up(end, spec).max(2);
        // A record field is word aligned: "The default alignment of the
        // beginning of a record is at least word aligned" (p. 3-26).
        let alloc = alloc.max(spec);
        (size, alloc, spec, alloc)
    }

    /// Size and alignments of a packed record whose fields' bits are set
    /// ([`TypeTable::layout_packed_fields`]).
    ///
    /// - Size: the bits used, rounded up to whole 16-bit words; "The
    ///   minimum size of a packed record is 16 bits" (ref 3.10.7, p.
    ///   3-35).  Figure 3-17 (p. 3-37): 56 bits used, "The whole record
    ///   takes up only eight bytes".
    /// - Alignment: the allocation algorithm of ref 3.10.5.6 (p. 3-27)
    ///   applied to the fields that are whole objects of their type at a
    ///   byte offset (a field narrower than its type cannot be naturally
    ///   aligned).  The reference states the algorithm for records in
    ///   general and gives no other for packed ones: Uncertain.  For
    ///   `status_$t` (its `all`, an integer32 at 0) and `uid_$t` it gives
    ///   longword alignment, the alignment of the unpacked records
    ///   `../source` declared them as before (codegen-patterns 5).  As a
    ///   field of an unpacked record a packed record is word aligned like
    ///   any record (ref 3.10.5.5 p. 3-26).
    fn packed_record_layout(&self, r: &RecordType) -> (u32, u32, u32, u32) {
        let mut end = 0u32;
        let mut alloc = 2u32;
        for f in r.all_fields() {
            let Some(b) = f.bits else { continue };
            end = end.max(b.bit.saturating_add(b.width));
            let fi = self.get(f.ty);
            if b.whole_bytes() == Some(fi.size)
                && fi.natural_align > 0
                && (b.bit / 8) % fi.natural_align == 0
            {
                alloc = alloc.max(fi.natural_align);
            }
        }
        let size = (round_up(end, 16) / 8).max(2);
        (size, alloc, 2, alloc)
    }

    /// The bits a field of type `ty` takes in a packed record and how it
    /// is aligned, ref 3.10.7 Table 3-3 (p. 3-36, read in the PDF) and the
    /// text above it (p. 3-35):
    ///
    /// | type | bits | aligned |
    /// |---|---|---|
    /// | integer, integer16 | 16 | word |
    /// | integer32, real, single | 32 | word |
    /// | double | 64 | word |
    /// | boolean | 1 | bit |
    /// | char | 8 | byte |
    /// | enumerated | "Number of bits required for largest ordinal value" | bit |
    /// | subrange of char | 8 | byte |
    /// | other subranges | "the number of bits required for their extreme values" | bit |
    /// | set | "If fewer than 32 elements, then exactly one bit per element; if more then 32 elements, then same size as set" | bit |
    /// | array | "the same space as an array outside of a packed record" | word ("Structured types, except for sets, are aligned on word boundaries") |
    /// | pointer | 32 | word |
    ///
    /// - Subranges: a negative low bound needs a sign bit (`-100..40`
    ///   takes 8 bits, p. 3-37: "seven bits can represent only 128
    ///   values").  An unsigned subrange that needs 31 bits takes 32
    ///   where it starts on a 16-bit boundary (ref 3.4, p. 3-10: "When
    ///   you have an unsigned subrange that is large enough to require 31
    ///   bits, the compiler allocates 32 bits"): Apollo's `linteger =
    ///   0..2147483647` is 32 bits in the packed records of the SR10.2
    ///   inserts (`fontn_$table_tver1`, whose lintegers sit at the
    ///   longword offsets 0x28, 0x2C, 0x34, 0x38 that `../source`'s
    ///   `smd_font_v3_t` asserts; `stream_$IR_rec_t`, whose `unused6 :
    ///   set of 0..21; { nxt field needs word alignment }` is right only
    ///   so; TTY_$I_WAIT 0xE1C2C0 stores `time_$clock_t.low32`, at bit
    ///   16, with `move.l`).  Where it does not, it takes its 31 bits:
    ///   the SAU9 page-map entry (`unused_31: boolean; fm_daddr:
    ///   0..16#7FFFFFFF`, 4 bytes) is read `bfextu (A0){1:31}` at
    ///   ast_$load_aote 0x3C403ADA (../source gap 361; bead
    ///   domain-pascal-nvqu; [`Self::layout_packed_fields`]).  A subrange
    ///   of boolean takes 1 bit, of an enumeration the bits of its high
    ///   bound.
    /// - Sets: "exactly one bit per element" for the ordinals 0..hi (a set
    ///   is a bit string indexed by ordinal, 3.9.3; `stream_$IR_rec_t`'s
    ///   `set of 0..21` must be 22 bits).  A set whose base type does not
    ///   start at ordinal 0 is not handled (how many bits "one bit per
    ///   element" gives it is not settled).  32 elements: both rules give
    ///   32 bits.
    /// - Records (packed or not) and varying strings are structured:
    ///   their size, word aligned.
    ///
    /// A size attribute (ref 3.15.5) that gives other bits than these is
    /// refused: which one a packed record takes is not stated (`[byte]
    /// 0..15`, `[byte] boolean`).
    pub fn packed_field_bits(&self, ty: TypeId) -> Result<(u32, PackAlign), String> {
        let (n, al) = self.packed_field_bits_by_kind(ty)?;
        if self.get(ty).attrs.size_bits.is_some_and(|b| b != n) {
            return Err(
                "a packed record field whose size attribute gives other bits than ref 3.10.7 \
                 Table 3-3 does (which one decides is not stated, 3.15.5)"
                    .into(),
            );
        }
        Ok((n, al))
    }

    fn packed_field_bits_by_kind(&self, ty: TypeId) -> Result<(u32, PackAlign), String> {
        fn bits_for(v: u64) -> u32 {
            (64 - v.leading_zeros()).max(1)
        }
        let info = self.get(ty);
        Ok(match &info.kind {
            TypeKind::Error | TypeKind::Forward { .. } => (8 * info.size, PackAlign::Word),
            TypeKind::Integer16 => (16, PackAlign::Word),
            TypeKind::Integer32 => (32, PackAlign::Word),
            TypeKind::Real { size } => (8 * size, PackAlign::Word),
            TypeKind::Boolean => (1, PackAlign::Bit),
            TypeKind::Char => (8, PackAlign::Byte),
            TypeKind::Enum { consts } => (
                bits_for(consts.len().saturating_sub(1) as u64),
                PackAlign::Bit,
            ),
            TypeKind::Subrange { base, lo, hi } => match self.kind(*base) {
                TypeKind::Char => (8, PackAlign::Byte),
                TypeKind::Boolean | TypeKind::Enum { .. } => {
                    (bits_for((*hi).max(0) as u64), PackAlign::Bit)
                }
                _ => {
                    let n = Self::subrange_bits(*lo, *hi);
                    (n.min(32), PackAlign::Bit)
                }
            },
            TypeKind::Set { elem } => {
                let (lo, hi) = self.ordinal_range(*elem).unwrap_or((0, 0));
                if lo != 0 {
                    return Err(
                        "a set whose base type does not start at ordinal 0 in a packed record \
                         (ref 3.10.7 Table 3-3: the bits of \"one bit per element\" not settled)"
                            .into(),
                    );
                }
                let n = (hi + 1) as u32;
                if n < 32 {
                    (n, PackAlign::Bit)
                } else {
                    (8 * info.size, PackAlign::Bit)
                }
            }
            TypeKind::Array { .. } | TypeKind::Record(_) | TypeKind::Varying { .. } => {
                (8 * info.size, PackAlign::Word)
            }
            TypeKind::Pointer { .. }
            | TypeKind::UnivPtr
            | TypeKind::Nil
            | TypeKind::RoutinePointer(_) => (8 * info.size, PackAlign::Word),
            TypeKind::StringLit { .. } => {
                return Err("a string constant type as a packed record field".into())
            }
        })
    }

    /// Give a packed record's fields their bits, from bit `start` (ref
    /// 3.10.7, p. 3-35): "Domain Pascal always starts the first field of a
    /// packed record on a word boundary.  After the first field, if the
    /// exact number of bits required for the next field crosses zero or
    /// one 16-bit boundary, the field starts in the next free bit.  If the
    /// field would cross two or more 16-bit boundaries, it starts at the
    /// next 16-bit boundary."  A boundary is crossed when it lies strictly
    /// inside the field (Figure 3-17: `Mass`, 32 bits at bit 0, ends on a
    /// boundary and does not cross it).  Byte- and word-aligned types
    /// start at the next byte or word.  Returns the end bit, or the index
    /// of a field whose type cannot be packed and why.
    pub fn layout_packed_fields(
        &self,
        fields: &mut [Field],
        start: u32,
    ) -> Result<u32, (usize, String)> {
        let mut at = start;
        for (i, f) in fields.iter_mut().enumerate() {
            let (n, al) = self.packed_field_bits(f.ty).map_err(|e| (i, e))?;
            let mut pos = match al {
                PackAlign::Bit => at,
                PackAlign::Byte => round_up(at, 8),
                PackAlign::Word => round_up(at, 16),
            };
            if n > 0 && (pos + n - 1) / 16 - pos / 16 >= 2 {
                pos = round_up(pos, 16);
            }
            // An unsigned subrange of 31 bits takes 32 where it starts on a
            // 16-bit boundary (ref 3.4 p. 3-10, "the compiler allocates 32
            // bits": TTY_$I_WAIT 0xE1C2C0 stores time_$clock_t's `low32:
            // linteger`, at bit 16, with `move.l`), its 31 where it does
            // not (the SAU9 page-map entry's `fm_daddr: 0..16#7FFFFFFF`
            // after a boolean, 4 bytes: ast_$load_aote 0x3C403ADA `bfextu
            // (A0){1:31},D1`; AST_$ALLOCATE_ASTE 0x3C40357C; ../source gap
            // 361, bead domain-pascal-nvqu).  Whether a bit-aligned field
            // after a 32-bit one starts at bit 32 or 31 is not visible:
            // Uncertain.
            let n = if n == 31
                && pos % 16 == 0
                && matches!(self.kind(f.ty), TypeKind::Subrange { lo, .. } if *lo >= 0)
            {
                32
            } else {
                n
            };
            f.offset = pos / 8;
            f.bits = Some(BitPos { bit: pos, width: n });
            at = pos.saturating_add(n);
        }
        Ok(at)
    }

    /// Assign offsets to a list of fields starting at `start`, by the
    /// word alignment rules or, with `natural`, the natural ones.  Returns
    /// the end offset.
    ///
    /// - Word rules, ref 3.10.5.5 p. 3-26: "the compiler lays out record
    ///   fields based on their guaranteed default alignment.  Thus, all
    ///   objects longer than a byte are aligned on word boundaries.
    ///   Objects which are chars and booleans may be aligned on byte
    ///   boundaries."  Checked against Figures 3-8..3-13 (PDF pp. 82-86).
    /// - Natural rules (`%natural_alignment`, `aligned record`): each
    ///   field at its natural alignment (ref 3.15.6.3 p. 3-70, Figure
    ///   3-25: `a: char; b: integer32; c: char` at 0, 4, 8; 3.10.6 p.
    ///   3-33, Figure 3-15).
    /// - A field whose type has an alignment attribute is aligned on it in
    ///   either environment (3.15.6.2 p. 3-64; 3.15.6.3 Figure 3-24 p.
    ///   3-70: `b, c: [ALIGNED(0)] integer16` after a char at 1 and 3;
    ///   3.15.6.4 Figure 3-29 p. 3-73; read in the PDF).
    /// - "A byte aligned field may not cross two word boundaries" (p.
    ///   3-26): such a field goes to the next word (3.15.6.3 p. 3-72: a
    ///   byte-aligned integer32 after a char lands at 2, as a word-aligned
    ///   one does).
    pub fn layout_fields(&self, fields: &mut [Field], start: u32, natural: bool) -> u32 {
        let mut off = start;
        for f in fields {
            let fi = self.get(f.ty);
            let align = match fi.attrs.align {
                Some(a) => a,
                None if natural => fi.natural_align,
                None => fi.field_align,
            };
            off = round_up(off, align);
            if align == 1 && fi.size > 1 {
                // Word boundaries strictly inside the field.
                let crossed = (off.saturating_add(fi.size) - 1) / 2 - off / 2;
                if crossed >= 2 {
                    off = round_up(off, 2);
                }
            }
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
            | TypeKind::Enum { .. } => Some(self.canon(id)),
            TypeKind::Subrange { base, .. } => Some(self.canon(*base)),
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

    /// Whether the integer type `id` holds unsigned values (the integer
    /// cases of `dpc-ir`'s `sign_of`, which uses this): a one-byte
    /// subrange with no negative value (`[byte] 0..255`, ref 3.15.5
    /// p. 3-62); a subrange of `integer32` stored in fewer bytes
    /// (`0..65535`) or with no negative value (`0..2147483647`); a
    /// subrange of such a subrange.  `integer16`, `integer32` and their
    /// other subranges are signed.
    pub fn unsigned_integer(&self, id: TypeId) -> bool {
        match self.kind(id) {
            TypeKind::Subrange { base, lo, .. }
                if self.size(id) == 1
                    && matches!(self.kind(*base), TypeKind::Integer16 | TypeKind::Integer32) =>
            {
                *lo >= 0
            }
            TypeKind::Subrange { base, lo, .. } => {
                let long_host = matches!(self.kind(*base), TypeKind::Integer32);
                if long_host && (self.size(id) < self.size(*base) || *lo >= 0) {
                    true
                } else {
                    self.unsigned_integer(*base)
                }
            }
            _ => false,
        }
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
        if let (TypeKind::Set { elem }, Some(8)) = (&info.kind, info.attrs.size_bits) {
            // A one-byte set (`sized`): its size attribute shown, since
            // `set of 0..7` alone is a word.
            return format!("[byte] set of {}", self.display(*elem));
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
