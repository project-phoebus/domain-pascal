//! Lexer (ref 2.1 "Building Blocks of Domain Pascal", pp. 2-1..2-7;
//! reserved words: Appendix A, Table A-1, p. A-1).
//!
//! The lexer turns one file (or a byte range of one, see
//! [`Lexer::new_range`]) into [`Token`]s.  It knows nothing about
//! `%include` or conditional compilation: `%name` directives come out as
//! [`TokenKind::Directive`] tokens and the preprocessor
//! ([`crate::preprocess`]) acts on them.  The parser consumes the
//! preprocessor's output, in which directives have been replaced by their
//! effect (included tokens, [`TokenKind::Pragma`] markers, nothing).
//!
//! Case: identifiers and keywords are case-insensitive (ref 2.1.6, p. 2-6).
//! [`TokenKind::Ident`] carries the name folded to lower case; the original
//! spelling is the span's text ([`crate::SourceMap::span_text`]).
//!
//! Strings: a [`TokenKind::Str`] is one quoted literal with `''` already
//! collapsed to `'` (ref 2.1.5, p. 2-4).  The `'abc'(13)(CR, LF)`
//! embedded-character syntax (ref 2.1.5.1, pp. 2-4..2-6) takes constant
//! names, so it is a parser/semantics matter: the lexer just yields
//! `Str`, `(`, `Int`/`Ident`, `)`, ... and the parser joins them.
//!
//! Signs: an integer literal never includes its sign; `-3` is `Minus`,
//! `Int(3)` (ref 2.1.2, p. 2-2).

use std::rc::Rc;

use crate::diag::Diagnostic;
use crate::source::{FileId, SourceMap, Span};

/// The reserved words of Table A-1 (ref Appendix A, p. A-1).  These and
/// only these are keywords.  Everything in Table A-2 (`module`,
/// `otherwise`, `xor`, `define`, `extern`, `internal`, `nil`, `exit`,
/// `return`, ...) is a *predeclared identifier* that programs may redefine,
/// so it lexes as [`TokenKind::Ident`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Keyword {
    And,
    Array,
    Begin,
    Case,
    Const,
    Div,
    Do,
    Downto,
    Else,
    End,
    File,
    For,
    Function,
    Goto,
    If,
    In,
    Label,
    Mod,
    Not,
    Of,
    Or,
    Packed,
    Procedure,
    Program,
    Record,
    Repeat,
    Set,
    Then,
    To,
    Type,
    Until,
    Var,
    While,
    With,
}

impl Keyword {
    /// Table A-1, p. A-1 (34 words; `nil` is in Table A-2, not here).
    const ALL: [(&'static str, Keyword); 34] = [
        ("and", Keyword::And),
        ("array", Keyword::Array),
        ("begin", Keyword::Begin),
        ("case", Keyword::Case),
        ("const", Keyword::Const),
        ("div", Keyword::Div),
        ("do", Keyword::Do),
        ("downto", Keyword::Downto),
        ("else", Keyword::Else),
        ("end", Keyword::End),
        ("file", Keyword::File),
        ("for", Keyword::For),
        ("function", Keyword::Function),
        ("goto", Keyword::Goto),
        ("if", Keyword::If),
        ("in", Keyword::In),
        ("label", Keyword::Label),
        ("mod", Keyword::Mod),
        ("not", Keyword::Not),
        ("of", Keyword::Of),
        ("or", Keyword::Or),
        ("packed", Keyword::Packed),
        ("procedure", Keyword::Procedure),
        ("program", Keyword::Program),
        ("record", Keyword::Record),
        ("repeat", Keyword::Repeat),
        ("set", Keyword::Set),
        ("then", Keyword::Then),
        ("to", Keyword::To),
        ("type", Keyword::Type),
        ("until", Keyword::Until),
        ("var", Keyword::Var),
        ("while", Keyword::While),
        ("with", Keyword::With),
    ];

    /// Look up an already lower-cased word.
    pub fn from_folded(s: &str) -> Option<Keyword> {
        Self::ALL.iter().find(|(w, _)| *w == s).map(|&(_, k)| k)
    }

    /// The keyword's spelling, lower case.
    pub fn as_str(self) -> &'static str {
        Self::ALL.iter().find(|(_, k)| *k == self).unwrap().0
    }
}

/// Directives that survive preprocessing as markers in the token stream,
/// because a later phase needs to see where they occur (ref 4.7 "Compiler
/// Directives", pp. 4-43..4-58).  The parser decides where it accepts
/// them; the directives may appear "anywhere a comment is valid" (p. 4-43).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pragma {
    /// `%natural_alignment;` (p. 4-57): undecorated data gets natural
    /// alignment from here on.
    NaturalAlignment,
    /// `%word_alignment;` (p. 4-57): back to the default word alignment.
    WordAlignment,
    /// `%push_alignment;` (p. 4-58): save the current alignment mode.
    PushAlignment,
    /// `%pop_alignment;` (p. 4-58): restore the last saved mode.
    PopAlignment,
    /// `%begin_inline;` (pp. 4-52..4-53).
    BeginInline,
    /// `%end_inline;` (pp. 4-52..4-53).
    EndInline,
    /// `%begin_noinline;` (pp. 4-53..4-54).
    BeginNoinline,
    /// `%end_noinline;` (pp. 4-53..4-54).
    EndNoinline,
}

/// Where a `%directive` was written (ref 4.7, p. 4-43).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectiveForm {
    /// `%name ...` in the token stream; its arguments are the following
    /// tokens.
    Statement,
    /// `{%name ...}`, `(*%name ...*)` or `"%name ..."`: the `%` is the
    /// first character after the opening delimiter (p. 4-43).  `body` is the
    /// text after the name, up to (not including) the closing delimiter;
    /// the arguments are lexed from it with [`Lexer::new_range`].
    Comment { body: Span },
}

/// What a token is.  Payload-free variants are punctuation and operators.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    /// Identifier (ref 2.1.1, p. 2-1), folded to lower case.
    Ident(String),
    /// Reserved word (Table A-1, p. A-1).
    Keyword(Keyword),
    /// Unsigned decimal integer literal (ref 2.1.2, p. 2-2).  The value is
    /// the literal's magnitude; range checks against `integer16`/
    /// `integer32` are the semantic phase's job.
    Int(u64),
    /// Unsigned `base#digits` integer literal (ref 2.1.2, p. 2-2), its
    /// value as written.  Kept apart from [`TokenKind::Int`] because the
    /// two have different ranges (ref 3.4 NOTE, p. 3-10: a decimal
    /// constant above 2147483647 is not valid, a based one with the
    /// sign bit set is negative); the semantic phase checks them.
    BasedInt(u64),
    /// Real literal (ref 2.1.3, pp. 2-2..2-3), as written.  Reals are out of
    /// scope for code generation (CLAUDE.md section 1) but are tokenised so
    /// the front end can diagnose them properly.
    Real(String),
    /// String literal (ref 2.1.5, p. 2-4) with `''` collapsed.
    Str(String),
    /// `%name` (ref 4.7, p. 4-43), name folded to lower case.  Only the raw
    /// lexer produces these; preprocessor output never contains them.
    Directive {
        name: String,
        form: DirectiveForm,
    },
    /// A layout or inlining directive passed through by the preprocessor.
    Pragma(Pragma),
    /// A `;` written right after a conditional directive (`%then`,
    /// `%else`, `%endif`, ref 4.7 pp. 4-47..4-49).  The lexer never
    /// produces it; the preprocessor passes such a `;` on as this token and
    /// the parser decides by context (owner ruling 2026-10-21,
    /// docs/reference-clarifications.md): it is a `;` where the parser
    /// accepts one (a statement, declaration, parameter, field or variant
    /// separator); elsewhere, where a stray `;` would be an error, it
    /// belongs to the directive and is dropped.
    CondSemi,

    // Operators (Table 4-1, p. 4-3; Bit Operators, p. 4-33).
    Plus,
    Minus,
    Star,
    /// `**` exponentiation (Table 4-1, p. 4-3).
    StarStar,
    Slash,
    /// `&` bitwise and (p. 4-33).
    Amp,
    /// `!` bitwise or (p. 4-33).
    Bang,
    /// `~` bitwise not (p. 4-33).
    Tilde,
    Eq,
    /// `<>`
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,

    // Punctuation.
    /// `:=`
    Assign,
    Colon,
    Semi,
    Comma,
    Dot,
    /// `..`
    DotDot,
    /// `^` pointer type / dereference.
    Caret,
    LParen,
    RParen,
    LBracket,
    RBracket,

    /// End of input.  Always the last token.
    Eof,
}

/// A token and where it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Longest identifier the reference allows (ref 2.1.1, p. 2-1).
pub const MAX_IDENT_LEN: usize = 4096;

/// A cursor over one file (or a byte range of it).  Diagnostics are
/// collected and fetched with [`Lexer::take_diagnostics`]; after an error
/// the lexer skips the bad input and carries on.
#[derive(Debug, Clone)]
pub struct Lexer {
    file: FileId,
    text: Rc<str>,
    pos: usize,
    end: usize,
    cond_names: bool,
    /// `-comchk` (ref 6.4.5, p. 6-11): warn of another left delimiter of
    /// a comment's own kind inside it.
    comchk: bool,
    diags: Vec<Diagnostic>,
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

impl Lexer {
    /// A lexer over the whole of `file`.
    pub fn new(sm: &SourceMap, file: FileId) -> Self {
        let text = sm.file(file).text.clone();
        let end = text.len();
        Self::with_range(file, text, 0, end)
    }

    /// A lexer over bytes `[lo, hi)` of `file`; spans stay file-relative.
    /// Used for the body of a comment-form directive.
    pub fn new_range(sm: &SourceMap, file: FileId, lo: u32, hi: u32) -> Self {
        Self::with_range(file, sm.file(file).text.clone(), lo as usize, hi as usize)
    }

    fn with_range(file: FileId, text: Rc<str>, lo: usize, hi: usize) -> Self {
        assert!(lo <= hi && hi <= text.len());
        Lexer {
            file,
            text,
            pos: lo,
            end: hi,
            cond_names: false,
            comchk: false,
            diags: Vec::new(),
        }
    }

    pub fn file(&self) -> FileId {
        self.file
    }

    /// Current byte offset.
    pub fn pos(&self) -> u32 {
        self.pos as u32
    }

    /// Move the cursor, e.g. back to the start of a token that should be
    /// lexed again (the preprocessor's one-token pushback).
    pub fn seek(&mut self, pos: u32) {
        assert!((pos as usize) <= self.end);
        self.pos = pos as usize;
    }

    /// Conditional-variable mode, used by the preprocessor while it reads
    /// `%var`/`%enable` names and `%if` predicates.  Names may then start
    /// with `_` (the predeclared `_BFMT__COFF`, `_ISP__M68K`, `_ISP__A88K`,
    /// p. 4-46) and contain `.` (`code.old`, `code.sr9`, p. 4-50).
    pub fn set_cond_names(&mut self, on: bool) {
        self.cond_names = on;
    }

    /// Comment checking, the `-comchk` compiler option (ref 6.4.5,
    /// p. 6-11; default off, `-ncomchk`): "the compiler returns a warning
    /// for every additional left comment delimiter" found before the
    /// comment's end, looking "only for the same kind of left comment
    /// delimiter" (a `{` inside `(* *)` is not flagged).  A `" "` comment
    /// cannot hold another of its kind: its left and right delimiters are
    /// the same character.
    pub fn set_comchk(&mut self, on: bool) {
        self.comchk = on;
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diags)
    }

    /// The next token without consuming it.  Diagnostics raised while
    /// peeking are dropped; they come back when the token is really read.
    pub fn peek(&mut self) -> Token {
        let (pos, nd) = (self.pos, self.diags.len());
        let t = self.next_token();
        self.pos = pos;
        self.diags.truncate(nd);
        t
    }

    fn byte(&self, i: usize) -> Option<u8> {
        if i < self.end {
            Some(self.text.as_bytes()[i])
        } else {
            None
        }
    }

    fn span(&self, lo: usize, hi: usize) -> Span {
        Span::new(self.file, lo as u32, hi as u32)
    }

    fn error(&mut self, lo: usize, hi: usize, msg: impl Into<String>) {
        let sp = self.span(lo, hi);
        self.diags.push(Diagnostic::error(sp, msg));
    }

    fn tok(&self, kind: TokenKind, lo: usize) -> Token {
        Token {
            kind,
            span: self.span(lo, self.pos),
        }
    }

    /// Read the next token.  At the end of the range this returns `Eof`
    /// (repeatedly).
    pub fn next_token(&mut self) -> Token {
        loop {
            // Whitespace, including the form feeds between listing pages.
            while matches!(
                self.byte(self.pos),
                Some(b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c)
            ) {
                self.pos += 1;
            }
            let lo = self.pos;
            let Some(c) = self.byte(lo) else {
                return self.tok(TokenKind::Eof, lo);
            };
            // Comments: ref 2.1.4, pp. 2-3..2-4.  Three forms, `{ }`,
            // `(* *)` and `" "`.  "The comment delimiters of Domain Pascal
            // must match": a comment ends at the first closing delimiter of
            // its own kind, so other kinds nest inside it but the same kind
            // does not ("interprets the first matching delimiter as the end
            // of the comment", p. 2-4).  The -iso unmatched-delimiter mode
            // is not supported.
            let comment = match c {
                b'{' => Some((1, "}")),
                b'(' if self.byte(lo + 1) == Some(b'*') => Some((2, "*)")),
                b'"' => Some((1, "\"")),
                _ => None,
            };
            if let Some((open, close)) = comment {
                if let Some(t) = self.comment(lo, open, close) {
                    return t;
                }
                continue;
            }
            return self.token(c, lo);
        }
    }

    /// Skip a comment starting at `lo`, or return the directive token if
    /// it is a comment-form directive.
    fn comment(&mut self, lo: usize, open: usize, close: &str) -> Option<Token> {
        let body = lo + open;
        let rest = &self.text[body..self.end];
        let Some(rel) = rest.find(close) else {
            self.error(lo, body, "unterminated comment");
            self.pos = self.end;
            return None;
        };
        let close_at = body + rel;
        self.pos = close_at + close.len();
        if self.comchk && close != "\"" {
            let left = &self.text[lo..body];
            let mut from = body;
            while let Some(k) = self.text[from..close_at].find(left) {
                let at = from + k;
                // ref 6.4.5, p. 6-11: the reference's message.
                self.diags.push(Diagnostic::warning(
                    self.span(at, at + left.len()),
                    "Unbalanced comment; another comment start found before end.",
                ));
                from = at + left.len();
            }
        }
        // Ref 4.7, p. 4-43: "If you specify a directive within a comment,
        // the percent sign must be the first character after the delimiter
        // (where spaces count as characters)."
        if self.byte(body) == Some(b'%')
            && self.byte(body + 1).is_some_and(|b| b.is_ascii_alphabetic())
        {
            let mut n = body + 1;
            while n < close_at && is_ident_char(self.text.as_bytes()[n]) {
                n += 1;
            }
            let name = self.text[body + 1..n].to_ascii_lowercase();
            return Some(Token {
                kind: TokenKind::Directive {
                    name,
                    form: DirectiveForm::Comment {
                        body: self.span(n, close_at),
                    },
                },
                span: self.span(lo, self.pos),
            });
        }
        None
    }

    fn token(&mut self, c: u8, lo: usize) -> Token {
        use TokenKind as K;
        let next = self.byte(lo + 1);
        let (kind, len) = match c {
            b'+' => (K::Plus, 1),
            b'-' => (K::Minus, 1),
            b'*' if next == Some(b'*') => (K::StarStar, 2),
            b'*' => (K::Star, 1),
            b'/' => (K::Slash, 1),
            b'&' => (K::Amp, 1),
            b'!' => (K::Bang, 1),
            b'~' => (K::Tilde, 1),
            b'=' => (K::Eq, 1),
            b'<' if next == Some(b'>') => (K::NotEq, 2),
            b'<' if next == Some(b'=') => (K::Le, 2),
            b'<' => (K::Lt, 1),
            b'>' if next == Some(b'=') => (K::Ge, 2),
            b'>' => (K::Gt, 1),
            b':' if next == Some(b'=') => (K::Assign, 2),
            b':' => (K::Colon, 1),
            b';' => (K::Semi, 1),
            b',' => (K::Comma, 1),
            b'.' if next == Some(b'.') => (K::DotDot, 2),
            b'.' => (K::Dot, 1),
            b'^' => (K::Caret, 1),
            b'(' => (K::LParen, 1),
            b')' => (K::RParen, 1),
            b'[' => (K::LBracket, 1),
            b']' => (K::RBracket, 1),
            b'\'' => return self.string(lo),
            b'%' => return self.directive(lo),
            b'0'..=b'9' => return self.number(lo),
            b'a'..=b'z' | b'A'..=b'Z' => return self.ident(lo),
            b'_' if self.cond_names => return self.ident(lo),
            b'_' => {
                // Ref 2.1.1, p. 2-1: "The first character is a letter".
                let t = self.ident(lo);
                self.error(lo, self.pos, "an identifier must begin with a letter");
                return t;
            }
            _ => {
                let ch = self.text[lo..].chars().next().unwrap();
                self.pos = lo + ch.len_utf8();
                self.error(lo, self.pos, format!("unexpected character `{ch}`"));
                return self.next_token();
            }
        };
        self.pos = lo + len;
        self.tok(kind, lo)
    }

    /// Identifier or reserved word (ref 2.1.1, p. 2-1).
    fn ident(&mut self, lo: usize) -> Token {
        let mut p = lo + 1;
        loop {
            match self.byte(p) {
                Some(b) if is_ident_char(b) => p += 1,
                Some(b'.') if self.cond_names && self.byte(p + 1).is_some_and(is_ident_char) => {
                    p += 1
                }
                _ => break,
            }
        }
        self.pos = p;
        if p - lo > MAX_IDENT_LEN {
            self.error(
                lo,
                p,
                format!("identifier longer than {MAX_IDENT_LEN} characters"),
            );
        }
        let folded = self.text[lo..p].to_ascii_lowercase();
        let kind = match Keyword::from_folded(&folded) {
            Some(k) => TokenKind::Keyword(k),
            None => TokenKind::Ident(folded),
        };
        self.tok(kind, lo)
    }

    /// `%name` in statement form (ref 4.7, p. 4-43).
    fn directive(&mut self, lo: usize) -> Token {
        let mut p = lo + 1;
        if !self.byte(p).is_some_and(|b| b.is_ascii_alphabetic()) {
            self.pos = p;
            self.error(lo, p, "`%` must be followed by a directive name");
            return self.next_token();
        }
        while self.byte(p).is_some_and(is_ident_char) {
            p += 1;
        }
        self.pos = p;
        let name = self.text[lo + 1..p].to_ascii_lowercase();
        self.tok(
            TokenKind::Directive {
                name,
                form: DirectiveForm::Statement,
            },
            lo,
        )
    }

    /// String literal (ref 2.1.5, p. 2-4).  `''` stands for one quote.  A
    /// string cannot be split across lines (ref 2.1.7, p. 2-6).  Comment
    /// delimiters inside are ordinary characters (p. 2-4).
    fn string(&mut self, lo: usize) -> Token {
        let mut value = String::new();
        let mut p = lo + 1;
        let mut seg = p;
        loop {
            match self.byte(p) {
                Some(b'\'') if self.byte(p + 1) == Some(b'\'') => {
                    value.push_str(&self.text[seg..=p]);
                    p += 2;
                    seg = p;
                }
                Some(b'\'') => {
                    value.push_str(&self.text[seg..p]);
                    self.pos = p + 1;
                    return self.tok(TokenKind::Str(value), lo);
                }
                None | Some(b'\n') | Some(b'\r') => {
                    value.push_str(&self.text[seg..p]);
                    self.pos = p;
                    self.error(lo, p, "unterminated string (a string cannot span lines)");
                    return self.tok(TokenKind::Str(value), lo);
                }
                Some(_) => p += 1,
            }
        }
    }

    fn digits(&mut self, mut p: usize) -> usize {
        while self.byte(p).is_some_and(|b| b.is_ascii_digit()) {
            p += 1;
        }
        p
    }

    /// Integer (ref 2.1.2, p. 2-2) or real (ref 2.1.3, pp. 2-2..2-3).
    fn number(&mut self, lo: usize) -> Token {
        let mut p = self.digits(lo);
        if self.byte(p) == Some(b'#') {
            return self.based(lo, p);
        }
        let mut real = false;
        // `1.5` is real; `1..5` is a range; `1.x` is `1` then `.`.  Ref
        // 2.1.3, p. 2-2: a decimal point needs a digit on both sides.
        if self.byte(p) == Some(b'.') && self.byte(p + 1).is_some_and(|b| b.is_ascii_digit()) {
            p = self.digits(p + 1);
            real = true;
        }
        // Exponent: `e` or `E`, optional sign, digits (p. 2-3, `5E-1`).
        // An `e` not followed by a valid exponent is left for the next
        // token.
        if matches!(self.byte(p), Some(b'e' | b'E')) {
            let mut q = p + 1;
            if matches!(self.byte(q), Some(b'+' | b'-')) {
                q += 1;
            }
            if self.byte(q).is_some_and(|b| b.is_ascii_digit()) {
                p = self.digits(q);
                real = true;
            }
        }
        self.pos = p;
        if real {
            return self.tok(TokenKind::Real(self.text[lo..p].to_string()), lo);
        }
        let v = self.text[lo..p].bytes().try_fold(0u64, |v, b| {
            v.checked_mul(10)?.checked_add((b - b'0') as u64)
        });
        let v = v.unwrap_or_else(|| {
            self.error(lo, p, "integer literal too large");
            0
        });
        self.tok(TokenKind::Int(v), lo)
    }

    /// `base#value`, base 2..16, digits `0-9`, `a-f`/`A-F` (ref 2.1.2,
    /// p. 2-2; C.1.2, p. C-1).  `hash` is the offset of `#`.
    fn based(&mut self, lo: usize, hash: usize) -> Token {
        let base: u64 = self.text[lo..hash].parse().unwrap_or(u64::MAX);
        let start = hash + 1;
        let mut p = start;
        while self.byte(p).is_some_and(|b| b.is_ascii_alphanumeric()) {
            p += 1;
        }
        self.pos = p;
        if !(2..=16).contains(&base) {
            self.error(lo, hash, "integer base must be from 2 to 16");
            return self.tok(TokenKind::BasedInt(0), lo);
        }
        if p == start {
            self.error(lo, p, format!("missing digits after `{base}#`"));
            return self.tok(TokenKind::BasedInt(0), lo);
        }
        let mut v: Option<u64> = Some(0);
        for (i, ch) in self.text[start..p].chars().enumerate() {
            match ch.to_digit(16).map(u64::from).filter(|&d| d < base) {
                Some(d) => v = v.and_then(|v| v.checked_mul(base)?.checked_add(d)),
                None => {
                    let at = start + i;
                    self.error(at, at + 1, format!("`{ch}` is not a base-{base} digit"));
                    return self.tok(TokenKind::BasedInt(0), lo);
                }
            }
        }
        let v = v.unwrap_or_else(|| {
            self.error(lo, p, "integer literal too large");
            0
        });
        self.tok(TokenKind::BasedInt(v), lo)
    }
}

/// Lex a whole file without preprocessing (directives stay as
/// [`TokenKind::Directive`]).  The last token is `Eof`.
pub fn tokenize(sm: &SourceMap, file: FileId) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut lx = Lexer::new(sm, file);
    let mut out = Vec::new();
    loop {
        let t = lx.next_token();
        let eof = t.kind == TokenKind::Eof;
        out.push(t);
        if eof {
            break;
        }
    }
    (out, lx.take_diagnostics())
}

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind as K;

    fn lex(src: &str) -> (Vec<TokenKind>, Vec<String>) {
        let mut sm = SourceMap::new();
        let id = sm.add("t.pas", src.to_string());
        let (toks, diags) = tokenize(&sm, id);
        (
            toks.into_iter().map(|t| t.kind).collect(),
            diags.iter().map(|d| d.render(&sm, "dpc")).collect(),
        )
    }

    /// Kinds of a clean lex, without the trailing `Eof`.
    fn kinds(src: &str) -> Vec<TokenKind> {
        let (mut k, d) = lex(src);
        assert!(d.is_empty(), "unexpected diagnostics: {d:?}");
        assert_eq!(k.pop(), Some(K::Eof));
        k
    }

    fn errors(src: &str) -> Vec<String> {
        lex(src).1
    }

    fn id(s: &str) -> K {
        K::Ident(s.to_string())
    }

    #[test]
    fn identifiers_with_dollar_and_underscore() {
        // ref 2.1.1, p. 2-1; C.1.1 p. C-1 (`mailing$lists`).
        assert_eq!(
            kinds("AST_$PAGE_ZERO M$OIU$WLW mailing$lists x1_"),
            vec![
                id("ast_$page_zero"),
                id("m$oiu$wlw"),
                id("mailing$lists"),
                id("x1_")
            ]
        );
    }

    #[test]
    fn case_folding_keeps_spelling_in_span() {
        let mut sm = SourceMap::new();
        let f = sm.add("t.pas", "BEGIN begin Begin Foo_Bar".to_string());
        let (toks, d) = tokenize(&sm, f);
        assert!(d.is_empty());
        for t in &toks[..3] {
            assert_eq!(t.kind, K::Keyword(Keyword::Begin));
        }
        assert_eq!(toks[3].kind, id("foo_bar"));
        assert_eq!(sm.span_text(toks[3].span), "Foo_Bar");
        assert_eq!(sm.span_text(toks[1].span), "begin");
    }

    #[test]
    fn identifier_rules() {
        assert!(errors("_x")[0].contains("must begin with a letter"));
        let long = "a".repeat(MAX_IDENT_LEN);
        assert!(errors(&long).is_empty());
        assert!(errors(&format!("{long}b"))[0].contains("longer than 4096"));
        // `$` cannot start an identifier.
        assert!(errors("$x")[0].contains("unexpected character `$`"));
    }

    #[test]
    fn all_reserved_words() {
        for (w, k) in Keyword::ALL {
            assert_eq!(kinds(w), vec![K::Keyword(k)]);
            assert_eq!(kinds(&w.to_uppercase()), vec![K::Keyword(k)]);
            assert_eq!(k.as_str(), w);
        }
        // Table A-2 words are identifiers, not keywords.
        for w in [
            "module",
            "otherwise",
            "xor",
            "nil",
            "define",
            "extern",
            "exit",
            "return",
        ] {
            assert_eq!(kinds(w), vec![id(w)]);
        }
    }

    #[test]
    fn decimal_integers() {
        assert_eq!(
            kinds("0 5260 007"),
            vec![K::Int(0), K::Int(5260), K::Int(7)]
        );
        assert_eq!(kinds("18446744073709551615"), vec![K::Int(u64::MAX)]);
        assert!(errors("18446744073709551616")[0].contains("too large"));
        // Signs are separate tokens (ref 2.1.2, p. 2-2).
        assert_eq!(kinds("-3"), vec![K::Minus, K::Int(3)]);
    }

    #[test]
    fn decimal_and_based_kept_apart() {
        // Same magnitude, different tokens: ref 3.4 NOTE, p. 3-10 gives
        // them different ranges (checked by the semantic phase).
        assert_eq!(kinds("2147483648"), vec![K::Int(1 << 31)]);
        assert_eq!(kinds("16#80000000"), vec![K::BasedInt(1 << 31)]);
        assert_eq!(kinds("4294967295"), vec![K::Int(u32::MAX as u64)]);
        assert_eq!(kinds("16#FFFFFFFF"), vec![K::BasedInt(u32::MAX as u64)]);
        assert_eq!(kinds("10#5"), vec![K::BasedInt(5)]);
        assert!(errors("4294967295 16#FFFFFFFF").is_empty());
    }

    #[test]
    fn based_integers_every_base() {
        // ref 2.1.2, p. 2-2 examples.
        assert_eq!(kinds("16#1c6"), vec![K::BasedInt(0x1c6)]);
        assert_eq!(kinds("16#1C6"), vec![K::BasedInt(0x1c6)]);
        assert_eq!(kinds("2#10010"), vec![K::BasedInt(18)]);
        assert_eq!(kinds("8#723"), vec![K::BasedInt(0o723)]);
        // Every base 2..16: the largest digit, then "10" = base.
        for base in 2u32..=16 {
            let d = char::from_digit(base - 1, 16).unwrap();
            assert_eq!(
                kinds(&format!("{base}#{d}")),
                vec![K::BasedInt(base as u64 - 1)]
            );
            assert_eq!(kinds(&format!("{base}#10")), vec![K::BasedInt(base as u64)]);
            let bad = char::from_digit(base % 16, 16).unwrap();
            if base < 16 {
                let e = errors(&format!("{base}#{bad}"));
                assert!(e[0].contains("is not a base"), "{e:?}");
            }
        }
        assert!(errors("1#0")[0].contains("from 2 to 16"));
        assert!(errors("17#0")[0].contains("from 2 to 16"));
        assert!(errors("16#")[0].contains("missing digits"));
        assert!(errors("16#ffffffffffffffff0")[0].contains("too large"));
        assert!(errors("16#1g")[0].contains("`g` is not a base-16 digit"));
    }

    #[test]
    fn reals_and_ranges() {
        // ref 2.1.3, pp. 2-2..2-3.
        assert_eq!(
            kinds("5.2 5.2E0 5.2E-2 5E-1 1e+3"),
            vec![
                K::Real("5.2".into()),
                K::Real("5.2E0".into()),
                K::Real("5.2E-2".into()),
                K::Real("5E-1".into()),
                K::Real("1e+3".into()),
            ]
        );
        assert_eq!(kinds("1..10"), vec![K::Int(1), K::DotDot, K::Int(10)]);
        // `.5` is wrong Pascal: it lexes as `.` `5` and the parser rejects it.
        assert_eq!(kinds(".5"), vec![K::Dot, K::Int(5)]);
        assert_eq!(kinds("5.x"), vec![K::Int(5), K::Dot, id("x")]);
        assert_eq!(kinds("2e"), vec![K::Int(2), id("e")]);
    }

    #[test]
    fn operators_and_punctuation() {
        assert_eq!(
            kinds("+ - * ** / & ! ~ = <> < <= > >= := : ; , . .. ^ ( ) [ ]"),
            vec![
                K::Plus,
                K::Minus,
                K::Star,
                K::StarStar,
                K::Slash,
                K::Amp,
                K::Bang,
                K::Tilde,
                K::Eq,
                K::NotEq,
                K::Lt,
                K::Le,
                K::Gt,
                K::Ge,
                K::Assign,
                K::Colon,
                K::Semi,
                K::Comma,
                K::Dot,
                K::DotDot,
                K::Caret,
                K::LParen,
                K::RParen,
                K::LBracket,
                K::RBracket,
            ]
        );
        assert_eq!(
            kinds("p^.x:=a"),
            vec![id("p"), K::Caret, K::Dot, id("x"), K::Assign, id("a")]
        );
        assert!(errors("@")[0].contains("unexpected character `@`"));
        assert!(errors("a } b")[0].contains("unexpected character `}`"));
    }

    #[test]
    fn three_comment_forms() {
        // ref 2.1.4, p. 2-3.
        assert_eq!(
            kinds("a { This is a comment. } b (* This is a comment. *) c \"This is a comment.\" d"),
            vec![id("a"), id("b"), id("c"), id("d")]
        );
        assert_eq!(
            kinds("a{x}b(*y*)c\"z\"d"),
            vec![id("a"), id("b"), id("c"), id("d")]
        );
        // Multi-line (p. 2-4).
        assert_eq!(
            kinds("{ This is a comment\n that stretches\n}x"),
            vec![id("x")]
        );
    }

    #[test]
    fn comment_nesting_rules() {
        // Other delimiter kinds nest (p. 2-3) ...
        assert_eq!(
            kinds("{ You can (*nest*) comments inside other comments. } x"),
            vec![id("x")]
        );
        assert_eq!(kinds("(* a { b } \" c \" *) x"), vec![id("x")]);
        // ... but the first matching delimiter ends the comment (p. 2-4).
        assert_eq!(kinds("{ a { b } x"), vec![id("x")]);
        assert_eq!(kinds("(* a (* b *) x"), vec![id("x")]);
        // `*)` does not end a `{` comment.
        assert_eq!(kinds("{ a *) } x"), vec![id("x")]);
        assert!(errors("{ never closed")[0].contains("1:1: error: unterminated comment"));
        assert!(errors("(* never closed")[0].contains("unterminated comment"));
        assert!(errors("\" never closed")[0].contains("unterminated comment"));
    }

    #[test]
    fn strings_and_quote_escape() {
        // ref 2.1.5, p. 2-4.
        assert_eq!(
            kinds("'This is a string.' '18' 'b[2~{q^%pl' 'can''t' ''"),
            vec![
                K::Str("This is a string.".into()),
                K::Str("18".into()),
                K::Str("b[2~{q^%pl".into()),
                K::Str("can't".into()),
                K::Str("".into()),
            ]
        );
        // Comment delimiters inside a string are ordinary characters.
        assert_eq!(kinds("'{ (* \"'"), vec![K::Str("{ (* \"".into())]);
        assert_eq!(kinds("''''"), vec![K::Str("'".into())]);
        // Strings cannot span lines (ref 2.1.7, p. 2-6).
        assert!(errors("'abc\ndef'")[0].contains("unterminated string"));
    }

    #[test]
    fn embedded_character_syntax_is_left_to_the_parser() {
        // ref 2.1.5.1, p. 2-5: 'newline'(CR,LF) lexes as plain tokens.
        assert_eq!(
            kinds("'newline'(CR,LF) 'x'(13)"),
            vec![
                K::Str("newline".into()),
                K::LParen,
                id("cr"),
                K::Comma,
                id("lf"),
                K::RParen,
                K::Str("x".into()),
                K::LParen,
                K::Int(13),
                K::RParen,
            ]
        );
    }

    #[test]
    fn directive_tokens() {
        let mut sm = SourceMap::new();
        let src = "%INCLUDE 'x'; {%nolist} (*%Eject*) \"%list\" { %notdirective } {%}";
        let f = sm.add("t.pas", src.to_string());
        let (toks, d) = tokenize(&sm, f);
        assert!(d.is_empty(), "{d:?}");
        let dir = |t: &Token| match &t.kind {
            K::Directive { name, form } => (name.clone(), *form),
            k => panic!("{k:?}"),
        };
        assert_eq!(dir(&toks[0]), ("include".into(), DirectiveForm::Statement));
        assert_eq!(toks[1].kind, K::Str("x".into()));
        assert_eq!(toks[2].kind, K::Semi);
        let (n, form) = dir(&toks[3]);
        assert_eq!(n, "nolist");
        assert_eq!(sm.span_text(toks[3].span), "{%nolist}");
        let DirectiveForm::Comment { body } = form else {
            panic!()
        };
        assert_eq!(sm.span_text(body), "");
        assert_eq!(dir(&toks[4]).0, "eject");
        assert_eq!(dir(&toks[5]).0, "list");
        // A space before `%` makes it an ordinary comment (p. 4-43).
        assert_eq!(toks[6].kind, K::Eof);
        assert!(errors("% x")[0].contains("directive name"));
    }

    #[test]
    fn comment_directive_body() {
        let mut sm = SourceMap::new();
        let f = sm.add("t.pas", "{%include 'a.ins.pas'}".to_string());
        let (toks, _) = tokenize(&sm, f);
        let K::Directive {
            form: DirectiveForm::Comment { body },
            ..
        } = toks[0].kind
        else {
            panic!()
        };
        assert_eq!(sm.span_text(body), " 'a.ins.pas'");
        let mut sub = Lexer::new_range(&sm, f, body.lo, body.hi);
        assert_eq!(sub.next_token().kind, K::Str("a.ins.pas".into()));
        assert_eq!(sub.next_token().kind, K::Eof);
    }

    #[test]
    fn cond_name_mode() {
        let mut sm = SourceMap::new();
        let f = sm.add("t.pas", "_ISP__M68K code.old x.".to_string());
        let mut lx = Lexer::new(&sm, f);
        lx.set_cond_names(true);
        assert_eq!(lx.next_token().kind, id("_isp__m68k"));
        assert_eq!(lx.next_token().kind, id("code.old"));
        assert_eq!(lx.next_token().kind, id("x"));
        assert_eq!(lx.next_token().kind, K::Dot);
        assert!(lx.take_diagnostics().is_empty());
    }

    #[test]
    fn spans_and_positions() {
        let mut sm = SourceMap::new();
        let f = sm.add("t.pas", "module m;\n  x := 16#ff;\n".to_string());
        let (toks, _) = tokenize(&sm, f);
        let texts: Vec<&str> = toks.iter().map(|t| sm.span_text(t.span)).collect();
        assert_eq!(texts, ["module", "m", ";", "x", ":=", "16#ff", ";", ""]);
        assert_eq!(sm.line_of(toks[3].span), 2);
        assert_eq!(sm.file(f).line_col(toks[5].span.lo).col, 8);
    }

    #[test]
    fn peek_and_seek() {
        let mut sm = SourceMap::new();
        let f = sm.add("t.pas", "a 9#9 b".to_string());
        let mut lx = Lexer::new(&sm, f);
        assert_eq!(lx.peek().kind, id("a"));
        let a = lx.next_token();
        assert_eq!(lx.peek().kind, K::BasedInt(0));
        assert!(lx.take_diagnostics().is_empty(), "peek drops diagnostics");
        lx.next_token();
        assert_eq!(lx.take_diagnostics().len(), 1);
        lx.seek(a.span.lo);
        assert_eq!(lx.next_token().kind, id("a"));
    }

    #[test]
    fn bad_characters_are_skipped() {
        let (k, d) = lex("a ? b é c");
        assert_eq!(k, vec![id("a"), id("b"), id("c"), K::Eof]);
        assert_eq!(d.len(), 2);
        assert!(d[0].contains("1:3:"), "{d:?}");
    }
}
