//! Recursive-descent parser (CLAUDE.md section 4.1).
//!
//! Input: the preprocessor's token stream ([`crate::preprocess::Output`]),
//! which ends with `Eof`.  Output: a [`CompilationUnit`] or the syntax
//! errors found.
//!
//! Each grammar function cites the reference section and printed page of
//! the rule it implements.  The reference is OCR text; rules were checked
//! against the PDF where the OCR scrambles syntax diagrams.
//!
//! Predeclared identifiers (Table A-2, p. A-1) are not keywords: `module`,
//! `otherwise`, `define`, `extern`, `static`, `options`, `attribute`,
//! `routine_option`, `univ`, `out`, `varying`, `aligned`, `unaligned`,
//! `exit`, `next`, `return` are recognised by spelling and context, so a
//! program can still use them as ordinary names where the context allows.
//!
//! Error recovery: a failed declaration item skips to the next `;` (or to
//! the start of the next declaration part); a failed statement skips to the
//! next `;`, `end`, `else` or `until`.  Nesting of `begin`/`record`/`case`
//! ... `end`, parentheses and brackets is respected while skipping.
//!
//! Directives ([`TokenKind::Pragma`]) are not seen by the grammar
//! functions.  They are collected by position and emitted as
//! [`Decl::Pragma`] at the next declaration boundary (see [`PragmaMark`]).
//! A directive *inside* a declaration item or routine heading is an error.
//! TODO(ref-question): 4.7 p. 4-43 says directives may appear "anywhere a
//! comment is valid"; what an alignment directive in the middle of, say, a
//! record type means is not stated.  Conservative choice: reject it.

use crate::ast::*;
use crate::diag::Diagnostic;
use crate::lexer::{Keyword as Kw, Token, TokenKind as T};
use crate::source::Span;

/// Parse the preprocessor's output ([`crate::preprocess::Output::tokens`],
/// which ends with `Eof`).
pub fn parse(tokens: &[Token]) -> Result<CompilationUnit, Vec<Diagnostic>> {
    let mut p = Parser::new(tokens);
    let unit = p.unit();
    match unit {
        Some(u) if p.diags.is_empty() => Ok(u),
        _ => {
            if p.diags.is_empty() {
                // Defensive: a failed parse always reports something.
                let span = p.span();
                p.diags.push(Diagnostic::error(span, "syntax error"));
            }
            Err(p.diags)
        }
    }
}

/// `Err(())` means a diagnostic has been recorded and the caller should
/// recover.
type PResult<T> = Result<T, ()>;

/// Which declaration parts a declaration list may contain.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// Program or module level: `define` allowed (ref 2.2.2.5, p. 2-15;
    /// 7.1, p. 7-2).
    Unit,
    /// Inside a routine: no `define` part (ref 2.2.3.2, p. 2-16).
    Routine,
}

struct Parser<'a> {
    /// The token stream without `Pragma` tokens.  Ends with `Eof`.
    toks: Vec<&'a Token>,
    /// Each directive with the index in `toks` of the token it precedes.
    pragmas: Vec<(usize, PragmaMark)>,
    /// First entry of `pragmas` not yet emitted.
    next_pragma: usize,
    pos: usize,
    diags: Vec<Diagnostic>,
    /// Current nesting of the recursive rules guarded by [`Parser::nested`].
    depth: usize,
}

/// Maximum nesting of expressions, types, statements, initialisers,
/// routines and parameter lists, counting each operator of a chain
/// (`a + b + c`) and each selector (`a^.b[i]`) as a level.  Deeper input
/// is rejected with a diagnostic instead of overflowing the stack of the
/// passes that recurse over the tree.  Sized for [`STACK_SIZE`] in a debug
/// build (bead domain-pascal-9gb): there, a level costs at most about
/// 13 KB across all the passes (5000 `*` overflowed 64 MB; every
/// construct at this limit compiles), so the margin is about 4x.  Far
/// past real code: a 997-term sum is accepted.
const MAX_DEPTH: usize = 1000;

/// The stack the compiler runs on (the driver's compile thread), for which
/// [`MAX_DEPTH`] is sized.  Callers running the passes on deep input use
/// a thread of this size; the 2 MB of a `cargo test` thread is enough
/// only for moderately nested input.
pub const STACK_SIZE: usize = 64 << 20;

/// A short description of a token for diagnostics.
fn describe(k: &T) -> String {
    match k {
        T::Ident(s) => format!("identifier `{s}`"),
        T::Keyword(kw) => format!("`{}`", kw.as_str()),
        T::Int(_) => "integer".into(),
        T::Real(_) => "real number".into(),
        T::Str(_) => "string".into(),
        T::Directive { name, .. } => format!("directive `%{name}`"),
        T::Pragma(_) => "directive".into(),
        T::Eof => "end of file".into(),
        other => format!("`{}`", punct(other)),
    }
}

fn punct(k: &T) -> &'static str {
    match k {
        T::Plus => "+",
        T::Minus => "-",
        T::Star => "*",
        T::StarStar => "**",
        T::Slash => "/",
        T::Amp => "&",
        T::Bang => "!",
        T::Tilde => "~",
        T::Eq => "=",
        T::NotEq => "<>",
        T::Lt => "<",
        T::Le => "<=",
        T::Gt => ">",
        T::Ge => ">=",
        T::Assign => ":=",
        T::Colon => ":",
        T::Semi => ";",
        T::Comma => ",",
        T::Dot => ".",
        T::DotDot => "..",
        T::Caret => "^",
        T::LParen => "(",
        T::RParen => ")",
        T::LBracket => "[",
        T::RBracket => "]",
        _ => "?",
    }
}

/// The short-form routine options (ref 5.5.1, p. 5-15): "You can use the
/// shorter format ... only for the forward, extern, internal, and val_param
/// routine options".
const SHORT_OPTIONS: [&str; 4] = ["forward", "extern", "internal", "val_param"];

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        let mut toks = Vec::with_capacity(tokens.len());
        let mut pragmas = Vec::new();
        for t in tokens {
            match t.kind {
                T::Pragma(pragma) => pragmas.push((
                    toks.len(),
                    PragmaMark {
                        pragma,
                        span: t.span,
                    },
                )),
                _ => toks.push(t),
            }
        }
        assert!(
            matches!(toks.last().map(|t| &t.kind), Some(T::Eof)),
            "token stream must end with Eof"
        );
        Parser {
            toks,
            pragmas,
            next_pragma: 0,
            pos: 0,
            diags: Vec::new(),
            depth: 0,
        }
    }

    // -----------------------------------------------------------------
    // Token cursor
    // -----------------------------------------------------------------

    fn peek(&self) -> &'a T {
        &self.toks[self.pos].kind
    }

    fn peek_at(&self, n: usize) -> &'a T {
        let i = (self.pos + n).min(self.toks.len() - 1);
        &self.toks[i].kind
    }

    fn span(&self) -> Span {
        self.toks[self.pos].span
    }

    /// Span of the last consumed token.
    fn prev_span(&self) -> Span {
        self.toks[self.pos.saturating_sub(1)].span
    }

    /// From `start` to the end of the last consumed token.
    fn since(&self, start: Span) -> Span {
        join(start, self.prev_span())
    }

    fn bump(&mut self) -> &'a Token {
        let t = self.toks[self.pos];
        if !matches!(t.kind, T::Eof) {
            self.pos += 1;
        }
        t
    }

    fn at(&self, k: &T) -> bool {
        self.peek() == k
    }

    fn at_kw(&self, kw: Kw) -> bool {
        matches!(self.peek(), T::Keyword(k) if *k == kw)
    }

    fn at_ident(&self, name: &str) -> bool {
        matches!(self.peek(), T::Ident(s) if s == name)
    }

    fn eat(&mut self, k: &T) -> bool {
        if self.at(k) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, kw: Kw) -> bool {
        if self.at_kw(kw) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Run `f` one nesting level deeper, or report that the input is
    /// nested too deeply (see [`MAX_DEPTH`]).
    fn nested<X>(&mut self, f: impl FnOnce(&mut Self) -> PResult<X>) -> PResult<X> {
        if self.depth >= MAX_DEPTH {
            let span = self.span();
            self.error_at(span, "constructs nested too deeply");
            return Err(());
        }
        self.depth += 1;
        let r = f(self);
        self.depth -= 1;
        r
    }

    /// Run `f`, the loop of a left-associative chain (`a + b + c`, ...),
    /// whose every operator calls [`Self::link`]: the operators nest the
    /// tree as deeply as parentheses do, and the passes after the parser
    /// recurse over it, so they count against [`MAX_DEPTH`] too (a chain
    /// of 1000 `+` overflowed the checker's stack, bead domain-pascal-1dn).
    fn chain<X>(&mut self, f: impl FnOnce(&mut Self) -> PResult<X>) -> PResult<X> {
        let depth = self.depth;
        let r = f(self);
        self.depth = depth;
        r
    }

    /// One more operator of a [`Self::chain`]: one level deeper, or the
    /// "nested too deeply" error at the operator.
    fn link(&mut self) -> PResult<()> {
        if self.depth >= MAX_DEPTH {
            let span = self.span();
            self.error_at(span, "constructs nested too deeply");
            return Err(());
        }
        self.depth += 1;
        Ok(())
    }

    // -----------------------------------------------------------------
    // Diagnostics
    // -----------------------------------------------------------------

    fn error_at(&mut self, span: Span, msg: impl Into<String>) {
        // One error per position: the first one is the useful one.
        if let Some(last) = self.diags.last() {
            if last.span == Some(span) {
                return;
            }
        }
        self.diags.push(Diagnostic::error(span, msg));
    }

    /// "expected WHAT, found TOKEN" at the current token.
    fn expected<X>(&mut self, what: &str) -> PResult<X> {
        let found = describe(self.peek());
        let span = self.span();
        self.error_at(span, format!("expected {what}, found {found}"));
        Err(())
    }

    fn expect(&mut self, k: &T) -> PResult<Span> {
        if self.at(k) {
            Ok(self.bump().span)
        } else {
            self.expected(&format!("`{}`", punct(k)))
        }
    }

    fn expect_kw(&mut self, kw: Kw) -> PResult<Span> {
        if self.at_kw(kw) {
            Ok(self.bump().span)
        } else {
            self.expected(&format!("`{}`", kw.as_str()))
        }
    }

    fn ident(&mut self) -> PResult<Ident> {
        match self.peek() {
            T::Ident(s) => {
                let span = self.bump().span;
                Ok(Ident {
                    name: s.clone(),
                    span,
                })
            }
            _ => self.expected("identifier"),
        }
    }

    /// `id {, id}`
    fn ident_list(&mut self) -> PResult<Vec<Ident>> {
        let mut v = vec![self.ident()?];
        while self.eat(&T::Comma) {
            v.push(self.ident()?);
        }
        Ok(v)
    }

    // -----------------------------------------------------------------
    // Recovery
    // -----------------------------------------------------------------

    /// Skip a broken declaration item: up to and including the next `;` at
    /// nesting depth 0, or up to (not including) the start of a declaration
    /// part, a routine, or `begin`.
    fn recover_decl(&mut self, start: usize) {
        let mut depth = 0usize;
        loop {
            match self.peek() {
                T::Eof => break,
                T::Semi if depth == 0 => {
                    self.bump();
                    break;
                }
                T::Keyword(Kw::Const | Kw::Type | Kw::Var | Kw::Label)
                | T::Keyword(Kw::Procedure | Kw::Function | Kw::Begin)
                    if depth == 0 && self.pos > start =>
                {
                    break
                }
                T::Keyword(Kw::Record) | T::LParen | T::LBracket => depth += 1,
                T::Keyword(Kw::End) if depth == 0 => {
                    if self.pos == start {
                        self.bump();
                    }
                    break;
                }
                // An unmatched `)`/`]` belongs to the broken item: skip it.
                T::Keyword(Kw::End) | T::RParen | T::RBracket => depth = depth.saturating_sub(1),
                _ => {}
            }
            self.bump();
        }
        if self.pos == start {
            self.bump();
        }
    }

    /// Skip a broken statement: up to (not including) the next `;`, `end`,
    /// `else` or `until` at nesting depth 0.
    fn recover_stmt(&mut self, start: usize) {
        let mut depth = 0usize;
        loop {
            match self.peek() {
                T::Eof => break,
                T::Semi | T::Keyword(Kw::Else) if depth == 0 => break,
                T::Keyword(Kw::Begin | Kw::Case | Kw::Record | Kw::Repeat)
                | T::LParen
                | T::LBracket => depth += 1,
                T::Keyword(Kw::End | Kw::Until) if depth == 0 => break,
                T::Keyword(Kw::End | Kw::Until) | T::RParen | T::RBracket => {
                    depth = depth.saturating_sub(1)
                }
                _ => {}
            }
            self.bump();
        }
        if self.pos == start
            && !matches!(
                self.peek(),
                T::Semi | T::Keyword(Kw::End | Kw::Else | Kw::Until) | T::Eof
            )
        {
            self.bump();
        }
    }

    // -----------------------------------------------------------------
    // Directives
    // -----------------------------------------------------------------

    /// Directives that precede the current token and have not been emitted.
    fn take_pragmas(&mut self) -> Vec<PragmaMark> {
        let mut out = Vec::new();
        while let Some(&(idx, mark)) = self.pragmas.get(self.next_pragma) {
            if idx > self.pos {
                break;
            }
            out.push(mark);
            self.next_pragma += 1;
        }
        out
    }

    fn pragmas_pending(&self) -> bool {
        matches!(self.pragmas.get(self.next_pragma), Some(&(idx, _)) if idx <= self.pos)
    }

    /// Reject directives that fell inside the construct just parsed (they
    /// precede an already-consumed token).  See the module docs.
    fn reject_inner_pragmas(&mut self) {
        while let Some(&(idx, mark)) = self.pragmas.get(self.next_pragma) {
            if idx >= self.pos {
                break;
            }
            self.next_pragma += 1;
            self.error_at(
                mark.span,
                "this directive is not supported inside a declaration; \
                 place it between declarations",
            );
        }
    }

    fn flush_pragmas(&mut self, out: &mut Vec<Decl>) {
        for m in self.take_pragmas() {
            out.push(Decl::Pragma(m));
        }
    }

    // -----------------------------------------------------------------
    // Compilation unit
    // -----------------------------------------------------------------

    /// program_heading | module_heading, declarations, [main action part]
    /// (ref 2.2 Figure 2-1, p. 2-8; 7.1 Figure 7-1, p. 7-2).
    fn unit(&mut self) -> Option<CompilationUnit> {
        let start = self.span();
        let leading_pragmas = self.take_pragmas();
        let (kind, name, code_section, data_section) = match self.heading() {
            Ok(h) => h,
            Err(()) => return None,
        };
        let is_program = matches!(kind, UnitKind::Program { .. });
        let mut decls = Vec::new();
        let mut body = None;
        loop {
            decls.extend(self.decls(Scope::Unit));
            if is_program && self.at_kw(Kw::Begin) {
                // Action part of the main program: `begin ... end.`
                // (ref 2.2.4, p. 2-17).
                match self.block() {
                    Ok(b) => body = Some(b),
                    Err(()) => self.recover_stmt(self.pos),
                }
                if self.expect(&T::Dot).is_ok() && !self.at(&T::Eof) {
                    let found = describe(self.peek());
                    let span = self.span();
                    self.error_at(span, format!("unexpected {found} after the final `end.`"));
                }
                break;
            }
            if !is_program && self.at_kw(Kw::End) && matches!(self.peek_at(1), T::Dot) {
                // A module has no main action part and no final `end.`: it
                // ends after its last routine's `end;` (ref 7.1 Figure 7-1
                // and the module examples, pp. 7-2, 7-9, 7-10).
                let span = join(self.span(), self.toks[self.pos + 1].span);
                self.error_at(
                    span,
                    "a module has no final `end.`; it ends after its last routine (ref 7.1, p. 7-2)",
                );
                self.bump();
                self.bump();
                continue;
            }
            if self.at(&T::Eof) {
                if is_program {
                    // A program must have an action part (ref 2.2.4).
                    let _: PResult<()> = self.expected("`begin`");
                }
                break;
            }
            // Something that is neither a declaration nor (for a program)
            // the action part.
            let at = self.pos;
            let _: PResult<()> = if is_program {
                self.expected("a declaration or `begin`")
            } else {
                self.expected("a declaration or routine")
            };
            if self.at_kw(Kw::Begin) {
                // A stray action part (e.g. after a broken heading): skip it.
                let _ = self.block();
                self.eat(&T::Semi);
                if self.pos == at {
                    self.bump();
                }
            } else {
                self.recover_decl(at);
            }
        }
        let trailing_pragmas = self.take_pragmas_all();
        let span = self.since(start);
        Some(CompilationUnit {
            kind,
            name,
            code_section,
            data_section,
            leading_pragmas,
            decls,
            body,
            trailing_pragmas,
            span,
        })
    }

    fn take_pragmas_all(&mut self) -> Vec<PragmaMark> {
        let v = self.pragmas[self.next_pragma..]
            .iter()
            .map(|&(_, m)| m)
            .collect();
        self.next_pragma = self.pragmas.len();
        v
    }

    /// `program name [(file_list)] [, code_section_name] [, data_section_name];`
    /// with `program name [(file_list)], , data_section_name;` for the
    /// default code section (ref 2.2.1, p. 2-10).
    ///
    /// `module name [, code_section_name] [, data_section_name];` (ref
    /// 7.1.1, p. 7-2).
    #[allow(clippy::type_complexity)]
    fn heading(&mut self) -> PResult<(UnitKind, Ident, Option<Ident>, Option<Ident>)> {
        let is_program = if self.eat_kw(Kw::Program) {
            true
        } else if self.at_ident("module") {
            self.bump();
            false
        } else {
            return self.expected("`program` or `module`");
        };
        let name = self.ident()?;
        let mut kind = UnitKind::Module;
        if is_program {
            let mut file_list = Vec::new();
            if self.eat(&T::LParen) {
                file_list = self.ident_list()?;
                self.expect(&T::RParen)?;
            }
            kind = UnitKind::Program { file_list };
        }
        let mut code = None;
        let mut data = None;
        if self.eat(&T::Comma) {
            if self.at(&T::Comma) {
                // `, , data_section_name` (p. 2-10).
                // TODO(ref-question): 7.1.1 p. 7-2 shows only
                // `[, code][, data]` for modules; the empty-code form is
                // documented for programs only.  Conservative: programs only.
                if !is_program {
                    return self.expected("code section name");
                }
            } else {
                code = Some(self.ident()?);
            }
            if self.eat(&T::Comma) {
                data = Some(self.ident()?);
            } else if code.is_none() {
                return self.expected("`,`");
            }
        }
        self.expect(&T::Semi)?;
        self.reject_inner_pragmas();
        Ok((kind, name, code, data))
    }

    // -----------------------------------------------------------------
    // Declarations
    // -----------------------------------------------------------------

    /// True at `define`/`attribute`/`routine_option` used as the word that
    /// opens a declaration part.  These are predeclared identifiers, not
    /// reserved words (Table A-2), so they open a part only when followed by
    /// an identifier.
    fn at_ident_part(&self) -> bool {
        matches!(self.peek(), T::Ident(s) if s == "define" || s == "attribute" || s == "routine_option")
            && matches!(self.peek_at(1), T::Ident(_))
    }

    /// An identifier that can begin another item of the current part.
    fn at_item(&self) -> bool {
        matches!(self.peek(), T::Ident(_)) && !self.at_ident_part()
    }

    /// Zero or more declaration parts and routines, in any order (ref
    /// 2.2.2, p. 2-11; 7.1 Figure 7-1, p. 7-2).  Stops at anything else.
    fn decls(&mut self, scope: Scope) -> Vec<Decl> {
        let mut out = Vec::new();
        loop {
            self.flush_pragmas(&mut out);
            let start = self.pos;
            match self.peek() {
                T::Keyword(Kw::Label) => self.label_part(&mut out),
                T::Keyword(Kw::Const) => self.const_part(&mut out),
                T::Keyword(Kw::Type) => self.type_part(&mut out),
                T::Keyword(Kw::Var) => self.var_part(&mut out),
                T::Keyword(Kw::Procedure | Kw::Function) | T::LBracket => {
                    match self.routine() {
                        Ok(r) => out.push(Decl::Routine(Box::new(r))),
                        Err(()) => {
                            self.recover_decl(start);
                            // Skip the action part of a routine whose heading
                            // was broken, so it is not taken for the main
                            // program's.
                            if self.at_kw(Kw::Begin) {
                                let _ = self.block();
                                self.eat(&T::Semi);
                            }
                        }
                    }
                }
                T::Ident(s) if self.at_ident_part() => match s.as_str() {
                    "define" => {
                        if scope == Scope::Routine {
                            // ref 2.2.3.2, p. 2-16: "the declaration part of
                            // a routine cannot contain a define declaration
                            // part".
                            let span = self.span();
                            self.error_at(span, "a define part is not allowed inside a routine");
                        }
                        self.define_part(&mut out)
                    }
                    "attribute" => self.attribute_part(&mut out),
                    _ => self.routine_option_part(&mut out),
                },
                _ => break,
            }
            if self.pos == start {
                // No progress: avoid looping on a token no rule accepts.
                self.bump();
            }
        }
        out
    }

    /// Parse the items of one declaration part.  `item` parses one item
    /// (including its `;`).  A directive between items splits the part
    /// (see [`Decl`]); `make` builds a part from its items.
    fn part_items<I>(
        &mut self,
        out: &mut Vec<Decl>,
        start: Span,
        more: fn(&Self) -> bool,
        item: fn(&mut Self) -> PResult<I>,
        make: &dyn Fn(Vec<I>, bool, Span) -> Decl,
    ) {
        let mut items = Vec::new();
        let mut cont = false;
        let mut part_start = start;
        let mut any = false;
        while more(self) {
            if self.pragmas_pending() {
                if !items.is_empty() {
                    let span = self.since(part_start);
                    out.push(make(std::mem::take(&mut items), cont, span));
                    cont = true;
                }
                self.flush_pragmas(out);
                part_start = self.span();
            }
            let at = self.pos;
            match item(self) {
                Ok(i) => {
                    items.push(i);
                    self.reject_inner_pragmas();
                }
                Err(()) => {
                    self.recover_decl(at);
                    self.reject_inner_pragmas();
                }
            }
            any = true;
        }
        if !any {
            let _: PResult<()> = self.expected("a declaration");
        }
        if !items.is_empty() {
            let span = self.since(part_start);
            out.push(make(items, cont, span));
        }
    }

    /// `label l1 [, ... lN];` (ref 2.2.2.1, p. 2-11).
    fn label_part(&mut self, out: &mut Vec<Decl>) {
        let start = self.bump().span;
        let at = self.pos;
        let r = (|| -> PResult<Vec<Label>> {
            let mut v = vec![self.label()?];
            while self.eat(&T::Comma) {
                v.push(self.label()?);
            }
            self.expect(&T::Semi)?;
            Ok(v)
        })();
        match r {
            Ok(labels) => {
                self.reject_inner_pragmas();
                out.push(Decl::Label(LabelPart {
                    labels,
                    span: self.since(start),
                }))
            }
            Err(()) => self.recover_decl(at),
        }
    }

    /// A label: an unsigned integer or an identifier (ref 2.2.2.1, p. 2-11).
    fn label(&mut self) -> PResult<Label> {
        match self.peek() {
            T::Int(n) => {
                let span = self.bump().span;
                Ok(Label::Int(*n, span))
            }
            T::Ident(_) => Ok(Label::Ident(self.ident()?)),
            _ => self.expected("label (identifier or unsigned integer)"),
        }
    }

    /// `const id = value; ...` (ref 2.2.2.2, p. 2-12).  Values are constant
    /// expressions (Table 2-1, p. 2-13); evaluation is semantic analysis.
    fn const_part(&mut self, out: &mut Vec<Decl>) {
        let start = self.bump().span;
        self.part_items(
            out,
            start,
            Self::at_item,
            |p| {
                let name = p.ident()?;
                p.expect(&T::Eq)?;
                let value = p.expr()?;
                p.expect(&T::Semi)?;
                let span = p.since(name.span);
                Ok(ConstDef { name, value, span })
            },
            &|defs, continues_previous, span| {
                Decl::Const(ConstPart {
                    defs,
                    continues_previous,
                    span,
                })
            },
        );
    }

    /// `type id = typename; ...` (ref 2.2.2.3, p. 2-13).
    fn type_part(&mut self, out: &mut Vec<Decl>) {
        let start = self.bump().span;
        self.part_items(
            out,
            start,
            Self::at_item,
            |p| {
                let name = p.ident()?;
                p.expect(&T::Eq)?;
                let mut ty = p.ty(false)?;
                p.expect(&T::Semi)?;
                p.routine_pointer_options(&mut ty)?;
                let span = p.since(name.span);
                Ok(TypeDef { name, ty, span })
            },
            &|defs, continues_previous, span| {
                Decl::Type(TypePart {
                    defs,
                    continues_previous,
                    span,
                })
            },
        );
    }

    /// `var [(section_name)] id_list : [alloc] typename [:= init]; ...`
    /// (ref 2.2.2.4 p. 2-14; 3.14 p. 3-52; 7.2 p. 7-3; initialisers 3.2.2
    /// p. 3-4 and following).
    fn var_part(&mut self, out: &mut Vec<Decl>) {
        let start = self.bump().span;
        let mut section = None;
        if self.at(&T::LParen) {
            // ref 3.14, p. 3-52: "any valid identifier or a string".
            self.bump();
            let r = match self.peek() {
                T::Ident(_) => self.ident().map(SectionName::Ident),
                T::Str(s) => {
                    let span = self.bump().span;
                    Ok(SectionName::Str(s.clone(), span))
                }
                _ => self.expected("section name"),
            };
            match r.and_then(|s| self.expect(&T::RParen).map(|_| s)) {
                Ok(s) => section = Some(s),
                Err(()) => {
                    let at = self.pos;
                    self.recover_decl(at);
                }
            }
        }
        self.part_items(
            out,
            start,
            Self::at_item,
            Self::var_decl,
            &|decls, continues_previous, span| {
                Decl::Var(VarPart {
                    section: section.clone(),
                    decls,
                    continues_previous,
                    span,
                })
            },
        );
    }

    /// `id_list : [define|extern|static] typename [:= init];` (ref 7.2,
    /// p. 7-3: "The optional variable_allocation_clause precedes the data
    /// type").
    fn var_decl(&mut self) -> PResult<VarDecl> {
        let start = self.span();
        let names = self.ident_list()?;
        self.expect(&T::Colon)?;
        let mut alloc = None;
        if let T::Ident(s) = self.peek() {
            let kind = match s.as_str() {
                "define" => Some(AllocKind::Define),
                "extern" => Some(AllocKind::Extern),
                "static" => Some(AllocKind::Static),
                _ => None,
            };
            // `x : static;` would name a type called `static`.
            if let Some(kind) = kind {
                if !matches!(self.peek_at(1), T::Semi | T::Assign) {
                    let span = self.bump().span;
                    alloc = Some(Alloc { kind, span });
                }
            }
        }
        let mut ty = self.ty(false)?;
        let init = if self.eat(&T::Assign) {
            Some(self.init()?)
        } else {
            None
        };
        self.expect(&T::Semi)?;
        self.routine_pointer_options(&mut ty)?;
        Ok(VarDecl {
            names,
            alloc,
            ty,
            init,
            span: self.since(start),
        })
    }

    /// `define variable1 [:= initial_value1]; ...` (ref 7.2.2, p. 7-5,
    /// checked against the PDF: items end with `;`).
    fn define_part(&mut self, out: &mut Vec<Decl>) {
        let start = self.bump().span;
        self.part_items(
            out,
            start,
            Self::at_item,
            |p| {
                let name = p.ident()?;
                let init = if p.eat(&T::Assign) {
                    Some(p.init()?)
                } else {
                    None
                };
                p.expect(&T::Semi)?;
                let span = p.since(name.span);
                Ok(DefineItem { name, init, span })
            },
            &|items, continues_previous, span| {
                Decl::Define(DefinePart {
                    items,
                    continues_previous,
                    span,
                })
            },
        );
    }

    /// `attribute id = attribute_name1 [, ... attribute_nameN]; ...` (ref
    /// 3.16, p. 3-78).
    fn attribute_part(&mut self, out: &mut Vec<Decl>) {
        let start = self.bump().span;
        self.part_items(
            out,
            start,
            Self::at_item,
            |p| {
                let name = p.ident()?;
                p.expect(&T::Eq)?;
                let mut attrs = vec![p.attribute()?];
                while p.eat(&T::Comma) {
                    attrs.push(p.attribute()?);
                }
                p.expect(&T::Semi)?;
                let span = p.since(name.span);
                Ok(AttributeDef { name, attrs, span })
            },
            &|defs, continues_previous, span| {
                Decl::Attribute(AttributePart {
                    defs,
                    continues_previous,
                    span,
                })
            },
        );
    }

    /// `routine_option id = name1 [, ... nameN]; ...` (ref 5.6.1, p. 5-21).
    fn routine_option_part(&mut self, out: &mut Vec<Decl>) {
        let start = self.bump().span;
        self.part_items(
            out,
            start,
            Self::at_item,
            |p| {
                let name = p.ident()?;
                p.expect(&T::Eq)?;
                let options = p.ident_list()?;
                p.expect(&T::Semi)?;
                let span = p.since(name.span);
                Ok(RoutineOptionDef {
                    name,
                    options,
                    span,
                })
            },
            &|defs, continues_previous, span| {
                Decl::RoutineOption(RoutineOptionPart {
                    defs,
                    continues_previous,
                    span,
                })
            },
        );
    }

    // -----------------------------------------------------------------
    // Initialisers
    // -----------------------------------------------------------------

    /// The value after `:=` in a var or define declaration: a constant
    /// expression or a bracketed list (ref 3.2.2 p. 3-4; 3.9.2 p. 3-15;
    /// 3.10.4 p. 3-21; 3.11.1 pp. 3-38..3-43; 3.13.4 p. 3-51).
    fn init(&mut self) -> PResult<Init> {
        self.nested(Self::init_inner)
    }

    fn init_inner(&mut self) -> PResult<Init> {
        if !self.at(&T::LBracket) {
            return Ok(Init::Expr(self.expr()?));
        }
        let start = self.bump().span;
        let mut items = Vec::new();
        // Items are separated by commas; a trailing comma before `]` is
        // allowed (examples on p. 3-40 and p. 3-41, checked in the PDF).
        while !self.at(&T::RBracket) {
            items.push(self.init_item()?);
            if !self.eat(&T::Comma) {
                break;
            }
        }
        self.expect(&T::RBracket)?;
        Ok(Init::List {
            items,
            span: self.since(start),
        })
    }

    /// One element of a bracketed initialiser:
    /// - `value` (3.11.1.2, p. 3-39; 3.10.4 second format, p. 3-21)
    /// - `n of value`, `* of value` (3.11.1.3, pp. 3-40..3-41)
    /// - `key := value`, `lo..hi := value`, `lo..* := value` (3.10.4 first
    ///   format, p. 3-21; 3.11.1.1 p. 3-39; 3.11.1.4 p. 3-41; 3.11.1.6
    ///   p. 3-43)
    /// - `lo..hi` (a set element range, 3.9.2 p. 3-15)
    fn init_item(&mut self) -> PResult<InitItem> {
        let start = self.span();
        if self.at(&T::Star) && matches!(self.peek_at(1), T::Keyword(Kw::Of)) {
            self.bump();
            self.bump();
            let value = self.init()?;
            return Ok(InitItem::Repeat {
                count: None,
                value,
                span: self.since(start),
            });
        }
        if self.at(&T::LBracket) {
            return Ok(InitItem::Value(self.init()?));
        }
        let e = self.expr()?;
        if self.eat_kw(Kw::Of) {
            let value = self.init()?;
            return Ok(InitItem::Repeat {
                count: Some(e),
                value,
                span: self.since(start),
            });
        }
        if self.eat(&T::DotDot) {
            let hi = if self.at(&T::Star) {
                Bound::Star(self.bump().span)
            } else {
                Bound::Expr(self.expr()?)
            };
            if self.eat(&T::Assign) {
                let value = self.init()?;
                return Ok(InitItem::Keyed {
                    key: e,
                    key_hi: Some(hi),
                    value,
                    span: self.since(start),
                });
            }
            return match hi {
                Bound::Expr(hi) => Ok(InitItem::Range {
                    lo: e,
                    hi,
                    span: self.since(start),
                }),
                Bound::Star(s) => {
                    self.error_at(s, "`lo..*` must be followed by `:=` and a value");
                    Err(())
                }
            };
        }
        if self.eat(&T::Assign) {
            let value = self.init()?;
            return Ok(InitItem::Keyed {
                key: e,
                key_hi: None,
                value,
                span: self.since(start),
            });
        }
        Ok(InitItem::Value(Init::Expr(e)))
    }

    // -----------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------

    /// A type (chapter 3).  `star_hi` allows `lo..*` (array index types,
    /// ref 3.11.1.5, p. 3-42).
    fn ty(&mut self, star_hi: bool) -> PResult<TypeExpr> {
        self.nested(|p| p.ty_inner(star_hi))
    }

    fn ty_inner(&mut self, star_hi: bool) -> PResult<TypeExpr> {
        let start = self.span();
        let kind = match self.peek() {
            // `[attr, ...] type` (ref 3.15, p. 3-53: "inside a pair of
            // brackets immediately before the type").
            T::LBracket => {
                let attrs = self.attribute_list()?;
                let ty = self.ty(star_hi)?;
                TypeKind::Attributed {
                    attrs,
                    ty: Box::new(ty),
                }
            }
            // `^typename` (3.13.1, p. 3-49) and `^procedure (...)` /
            // `^function (...): T` (3.13.3, p. 3-50).
            T::Caret => {
                self.bump();
                if matches!(self.peek(), T::Keyword(Kw::Procedure | Kw::Function)) {
                    let sig = self.routine_sig(false)?;
                    TypeKind::RoutinePointer(Box::new(sig))
                } else {
                    TypeKind::Pointer(Box::new(self.ty(false)?))
                }
            }
            T::Keyword(Kw::Packed) => {
                self.bump();
                match self.peek() {
                    T::Keyword(Kw::Array) => self.array_type(true)?,
                    T::Keyword(Kw::Record) => self.record_type(RecordForm::Packed)?,
                    T::Keyword(Kw::Set) => self.set_or_file(true)?,
                    T::Keyword(Kw::File) => self.set_or_file(true)?,
                    _ => return self.expected("`array`, `record`, `set` or `file` after `packed`"),
                }
            }
            T::Keyword(Kw::Array) => self.array_type(false)?,
            T::Keyword(Kw::Record) => self.record_type(RecordForm::Plain)?,
            T::Keyword(Kw::Set | Kw::File) => self.set_or_file(false)?,
            // `aligned record` / `unaligned record` (3.10.6, pp. 3-33..3-34).
            T::Ident(s)
                if (s == "aligned" || s == "unaligned")
                    && matches!(self.peek_at(1), T::Keyword(Kw::Record)) =>
            {
                let form = if s == "aligned" {
                    RecordForm::Aligned
                } else {
                    RecordForm::Unaligned
                };
                self.bump();
                self.record_type(form)?
            }
            // `varying[max_length] of char` (3.11.2, p. 3-44).
            T::Ident(s) if s == "varying" && matches!(self.peek_at(1), T::LBracket) => {
                self.bump();
                self.bump();
                let max = self.expr()?;
                self.expect(&T::RBracket)?;
                self.expect_kw(Kw::Of)?;
                let elem = self.ty(false)?;
                TypeKind::Varying {
                    max,
                    elem: Box::new(elem),
                }
            }
            // `(a, b, c)` (3.7, p. 3-13).
            T::LParen => {
                self.bump();
                let ids = self.ident_list()?;
                self.expect(&T::RParen)?;
                TypeKind::Enum(ids)
            }
            // A type name or a subrange `lo..hi` (3.8, p. 3-14).
            _ => {
                let lo = self.expr()?;
                if self.eat(&T::DotDot) {
                    let hi = if star_hi && self.at(&T::Star) {
                        Bound::Star(self.bump().span)
                    } else {
                        Bound::Expr(self.expr()?)
                    };
                    TypeKind::Subrange { lo, hi }
                } else if let ExprKind::Name(id) = lo.kind {
                    TypeKind::Named(id)
                } else {
                    self.error_at(lo.span, "expected a type");
                    return Err(());
                }
            }
        };
        Ok(TypeExpr {
            kind,
            span: self.since(start),
        })
    }

    /// `[attr {, attr}]` (ref 3.15, p. 3-53).
    fn attribute_list(&mut self) -> PResult<Vec<Attribute>> {
        self.expect(&T::LBracket)?;
        let mut v = vec![self.attribute()?];
        while self.eat(&T::Comma) {
            v.push(self.attribute()?);
        }
        self.expect(&T::RBracket)?;
        Ok(v)
    }

    /// `name [(args)]`: `volatile`, `address(16#ff7000)`, `device(read)`,
    /// `aligned(1)`, `long`, a user attribute (ref 3.15 pp. 3-53..3-63;
    /// 3.16 p. 3-78).  Arguments are expressions; their meaning is semantic
    /// analysis.
    fn attribute(&mut self) -> PResult<Attribute> {
        let name = self.ident()?;
        let args = if self.eat(&T::LParen) {
            let a = self.expr_list()?;
            self.expect(&T::RParen)?;
            Some(a)
        } else {
            None
        };
        Ok(Attribute {
            span: self.since(name.span),
            name,
            args,
        })
    }

    /// `array[index {, index}] of type` (ref 3.11, p. 3-37; up to eight
    /// dimensions, which is semantic analysis).
    fn array_type(&mut self, packed: bool) -> PResult<TypeKind> {
        self.expect_kw(Kw::Array)?;
        self.expect(&T::LBracket)?;
        let mut indices = vec![self.ty(true)?];
        while self.eat(&T::Comma) {
            indices.push(self.ty(true)?);
        }
        self.expect(&T::RBracket)?;
        self.expect_kw(Kw::Of)?;
        let elem = self.ty(false)?;
        Ok(TypeKind::Array {
            packed,
            indices,
            elem: Box::new(elem),
        })
    }

    /// `set of T` (3.9.1, p. 3-15) / `file of T` (3.12, p. 3-48).
    fn set_or_file(&mut self, packed: bool) -> PResult<TypeKind> {
        let is_set = self.at_kw(Kw::Set);
        self.bump();
        self.expect_kw(Kw::Of)?;
        let elem = Box::new(self.ty(false)?);
        Ok(if is_set {
            TypeKind::Set { packed, elem }
        } else {
            TypeKind::File { packed, elem }
        })
    }

    /// `record fixed_part; variant_part; end` (ref 3.10.1 p. 3-17; 3.10.2
    /// p. 3-19).
    fn record_type(&mut self, form: RecordForm) -> PResult<TypeKind> {
        self.expect_kw(Kw::Record)?;
        let (fields, variant) = self.field_list()?;
        self.expect_kw(Kw::End)?;
        Ok(TypeKind::Record {
            form,
            fields,
            variant,
        })
    }

    /// `[field {; field}] [; variant_part] [;]`, ended by `end` or `)`.
    #[allow(clippy::type_complexity)]
    fn field_list(&mut self) -> PResult<(Vec<FieldDecl>, Option<Box<VariantPart>>)> {
        self.nested(Self::field_list_inner)
    }

    #[allow(clippy::type_complexity)]
    fn field_list_inner(&mut self) -> PResult<(Vec<FieldDecl>, Option<Box<VariantPart>>)> {
        let mut fields = Vec::new();
        let mut variant = None;
        loop {
            match self.peek() {
                T::Keyword(Kw::End) | T::RParen => break,
                T::Keyword(Kw::Case) => {
                    variant = Some(Box::new(self.variant_part()?));
                    self.eat(&T::Semi);
                    break;
                }
                _ => {}
            }
            let start = self.span();
            let names = self.ident_list()?;
            self.expect(&T::Colon)?;
            let mut ty = self.ty(false)?;
            let span = self.since(start);
            if !self.eat(&T::Semi) {
                fields.push(FieldDecl { names, ty, span });
                break;
            }
            self.routine_pointer_options(&mut ty)?;
            fields.push(FieldDecl { names, ty, span });
        }
        Ok((fields, variant))
    }

    /// `case [tag_field :] typename of constantlist : (field; ...); ...`
    /// (ref 3.10.2, p. 3-19, checked against the PDF).
    fn variant_part(&mut self) -> PResult<VariantPart> {
        let start = self.expect_kw(Kw::Case)?;
        let first = self.ident()?;
        let (tag, tag_type) = if self.eat(&T::Colon) {
            (Some(first), self.ident()?)
        } else {
            (None, first)
        };
        self.expect_kw(Kw::Of)?;
        let mut arms = Vec::new();
        while !matches!(self.peek(), T::Keyword(Kw::End) | T::RParen) {
            let arm_start = self.span();
            let labels = self.expr_list()?;
            self.expect(&T::Colon)?;
            self.expect(&T::LParen)?;
            let (fields, variant) = self.field_list()?;
            self.expect(&T::RParen)?;
            arms.push(VariantArm {
                labels,
                fields,
                variant,
                span: self.since(arm_start),
            });
            if !self.eat(&T::Semi) {
                break;
            }
        }
        Ok(VariantPart {
            tag,
            tag_type,
            arms,
            span: self.since(start),
        })
    }

    // -----------------------------------------------------------------
    // Routines
    // -----------------------------------------------------------------

    /// `procedure [name] [(params)]` or `function [name] [(params)] [: type]`.
    /// `named` requires a name (parameters, 5.4 p. 5-13); routine pointer
    /// types have none (3.13.3, p. 3-50).
    fn routine_sig(&mut self, named: bool) -> PResult<RoutineSig> {
        let start = self.span();
        let kind = if self.eat_kw(Kw::Procedure) {
            RoutineKind::Procedure
        } else if self.eat_kw(Kw::Function) {
            RoutineKind::Function
        } else {
            return self.expected("`procedure` or `function`");
        };
        let name = if named { Some(self.ident()?) } else { None };
        let params = if self.at(&T::LParen) {
            Some(self.param_list()?)
        } else {
            None
        };
        let result = self.result_type(kind)?;
        Ok(RoutineSig {
            kind,
            name,
            params,
            result,
            options: Vec::new(),
            span: self.since(start),
        })
    }

    /// `: typename` after a function heading (ref 2.2.3.1, p. 2-16).
    fn result_type(&mut self, kind: RoutineKind) -> PResult<Option<TypeExpr>> {
        if !self.at(&T::Colon) {
            return Ok(None);
        }
        if kind == RoutineKind::Procedure {
            let span = self.span();
            self.error_at(span, "a procedure has no result type");
            return Err(());
        }
        self.bump();
        Ok(Some(self.ty(false)?))
    }

    /// `( [mode] par_list : typename; ... )` (ref 5.1, p. 5-1).
    fn param_list(&mut self) -> PResult<Vec<Param>> {
        self.nested(Self::param_list_inner)
    }

    fn param_list_inner(&mut self) -> PResult<Vec<Param>> {
        self.expect(&T::LParen)?;
        let mut v = vec![self.param()?];
        while self.eat(&T::Semi) {
            v.push(self.param()?);
        }
        self.expect(&T::RParen)?;
        Ok(v)
    }

    /// One parameter group: `[var|in|out|in out] names : [univ] type` (ref
    /// 5.3 p. 5-5; 5.3.2 p. 5-7; 5.3.3) or a routine parameter (5.4, p.
    /// 5-13).
    fn param(&mut self) -> PResult<Param> {
        let start = self.span();
        if matches!(self.peek(), T::Keyword(Kw::Procedure | Kw::Function)) {
            let sig = self.routine_sig(true)?;
            return Ok(Param::Routine {
                sig,
                span: self.since(start),
            });
        }
        // `out` is a predeclared identifier and `in` a keyword: a mode only
        // when a parameter name follows (`(out : integer)` names a
        // parameter `out`).
        let name_follows = |p: &Self, n: usize| matches!(p.peek_at(n), T::Ident(_));
        let mode = if self.at_kw(Kw::Var) {
            self.bump();
            ParamMode::Var
        } else if self.at_kw(Kw::In) {
            self.bump();
            if self.at_ident("out") && name_follows(self, 1) {
                self.bump();
                ParamMode::InOut
            } else {
                ParamMode::In
            }
        } else if self.at_ident("out") && name_follows(self, 1) {
            self.bump();
            ParamMode::Out
        } else {
            ParamMode::Value
        };
        let names = self.ident_list()?;
        self.expect(&T::Colon)?;
        // `univ` turns off type checking (ref 5.3.3, p. 5-9).  `x : univ`
        // alone would name a type called `univ`.
        let univ = self.at_ident("univ") && !matches!(self.peek_at(1), T::Semi | T::RParen);
        if univ {
            self.bump();
        }
        let ty = self.ty(false)?;
        Ok(Param::Data {
            mode,
            names,
            univ,
            ty,
            span: self.since(start),
        })
    }

    /// `[section(codesect, datasect)]`, either name optional (ref 5.7.1,
    /// p. 5-25).  The section attribute is the only routine attribute
    /// (5.7, p. 5-24).
    fn routine_attrs(&mut self) -> PResult<Vec<RoutineAttr>> {
        self.expect(&T::LBracket)?;
        let mut v = Vec::new();
        loop {
            let start = self.span();
            if !self.at_ident("section") {
                return self.expected("routine attribute `section`");
            }
            self.bump();
            self.expect(&T::LParen)?;
            let code = if self.at(&T::Comma) {
                None
            } else {
                Some(self.ident()?)
            };
            let data = if self.eat(&T::Comma) {
                Some(self.ident()?)
            } else {
                None
            };
            if code.is_none() && data.is_none() {
                return self.expected("section name");
            }
            self.expect(&T::RParen)?;
            v.push(RoutineAttr::Section {
                code,
                data,
                span: self.since(start),
            });
            if !self.eat(&T::Comma) {
                break;
            }
        }
        self.expect(&T::RBracket)?;
        Ok(v)
    }

    /// A routine declaration (ref 2.2.3.1, p. 2-16; chapter 5 intro, p. 5-1,
    /// checked against the PDF):
    ///
    /// ```text
    /// [attribute_list] procedure name [(parameter_list)]; [routine_options;]
    /// [attribute_list] function name [(parameter_list)] : typename; [routine_options;]
    /// ```
    ///
    /// followed, unless the options include `forward` or `extern`, by
    /// declarations, nested routines and `begin ... end;` (2.2.3.2..2.2.3.4,
    /// pp. 2-16..2-17).  The result type may be omitted when the routine
    /// was declared before (7.3.4, p. 7-11: `function exponent;`).
    fn routine(&mut self) -> PResult<Routine> {
        self.nested(Self::routine_inner)
    }

    fn routine_inner(&mut self) -> PResult<Routine> {
        let start = self.span();
        let attrs = if self.at(&T::LBracket) {
            self.routine_attrs()?
        } else {
            Vec::new()
        };
        let kind = if self.eat_kw(Kw::Procedure) {
            RoutineKind::Procedure
        } else if self.eat_kw(Kw::Function) {
            RoutineKind::Function
        } else {
            return self.expected("`procedure` or `function`");
        };
        let name = self.ident()?;
        let params = if self.at(&T::LParen) {
            Some(self.param_list()?)
        } else {
            None
        };
        let result = self.result_type(kind)?;
        self.expect(&T::Semi)?;
        let options = self.routine_options()?;
        self.reject_inner_pragmas();
        let has_body = !options
            .iter()
            .any(|o| o.name == "forward" || o.name == "extern");
        let body = if has_body {
            let decls = self.decls(Scope::Routine);
            let block = self.block()?;
            self.expect(&T::Semi)?;
            Some(RoutineBody { decls, block })
        } else {
            None
        };
        Ok(Routine {
            attrs,
            kind,
            name,
            params,
            result,
            options,
            body,
            span: self.since(start),
        })
    }

    /// Routine options after the heading's `;` (ref 5.5.1, p. 5-15):
    /// `options(opt1, ..., optN);` or, for forward/extern/internal/val_param
    /// only, the short form `opt1; ... optN;`.  Both forms may follow each
    /// other (p. 5-15: `EXTERN; val_param;`).
    fn routine_options(&mut self) -> PResult<Vec<Ident>> {
        let mut v = Vec::new();
        loop {
            if self.at_ident("options") && matches!(self.peek_at(1), T::LParen) {
                self.bump();
                self.bump();
                v.extend(self.ident_list()?);
                self.expect(&T::RParen)?;
                self.expect(&T::Semi)?;
            } else if matches!(self.peek(), T::Ident(s) if SHORT_OPTIONS.contains(&s.as_str()))
                && matches!(self.peek_at(1), T::Semi)
            {
                v.push(self.ident()?);
                self.bump();
            } else {
                break;
            }
        }
        Ok(v)
    }

    /// Routine options after a routine pointer type and its `;` (ref
    /// p. 4-146, `pptr : ^Procedure(...); EXTERN;`, checked in the PDF;
    /// Appendix B messages 181, 182): the same forms as after a heading
    /// ([`Self::routine_options`], 5.5.1 p. 5-15).  Nothing for other
    /// types.
    fn routine_pointer_options(&mut self, ty: &mut TypeExpr) -> PResult<()> {
        if let TypeKind::RoutinePointer(sig) = &mut ty.kind {
            sig.options = self.routine_options()?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------

    /// `begin stmt {; stmt} end` (ref "Statements", p. 4-188).
    fn block(&mut self) -> PResult<Block> {
        let start = self.expect_kw(Kw::Begin)?;
        let stmts = self.stmt_seq(|p| p.at_kw(Kw::End));
        self.expect_kw(Kw::End)?;
        Ok(Block {
            stmts,
            span: self.since(start),
        })
    }

    /// Statements separated by `;`, up to a token for which `done` holds
    /// (not consumed).
    fn stmt_seq(&mut self, done: fn(&Self) -> bool) -> Vec<Stmt> {
        let mut v = Vec::new();
        loop {
            let at = self.pos;
            match self.stmt() {
                Ok(s) => v.push(s),
                Err(()) => self.recover_stmt(at),
            }
            if self.eat(&T::Semi) {
                continue;
            }
            if done(self) || self.at(&T::Eof) {
                break;
            }
            // Something follows the statement that cannot.
            let at = self.pos;
            let _: PResult<()> = self.expected("`;`");
            self.recover_stmt(at);
            if self.eat(&T::Semi) {
                continue;
            }
            if done(self) || self.at(&T::Eof) {
                break;
            }
            if self.pos == at {
                if self.at_kw(Kw::Else) {
                    // `else` with no `if`: skip it and go on.
                    self.bump();
                    continue;
                }
                // `end`/`until` that belongs to an enclosing construct.
                break;
            }
        }
        v
    }

    /// True at a token that ends a statement without starting one.
    fn at_stmt_end(&self) -> bool {
        matches!(
            self.peek(),
            T::Semi | T::Eof | T::Keyword(Kw::End | Kw::Else | Kw::Until)
        )
    }

    /// One statement (ref "Statements", p. 4-188).
    fn stmt(&mut self) -> PResult<Stmt> {
        self.nested(Self::stmt_inner)
    }

    fn stmt_inner(&mut self) -> PResult<Stmt> {
        let start = self.span();
        // `label: stmt` (p. 4-188: "A statement can be preceded by a label").
        if matches!(self.peek(), T::Int(_) | T::Ident(_)) && matches!(self.peek_at(1), T::Colon) {
            let label = self.label()?;
            self.bump();
            let stmt = self.stmt()?;
            return Ok(Stmt {
                kind: StmtKind::Labeled {
                    label,
                    stmt: Box::new(stmt),
                },
                span: self.since(start),
            });
        }
        if self.at_stmt_end() {
            // The empty statement (p. 4-188).
            return Ok(Stmt {
                kind: StmtKind::Empty,
                span: Span::new(start.file, start.lo, start.lo),
            });
        }
        let kind = match self.peek() {
            T::Keyword(Kw::Begin) => StmtKind::Compound(self.block()?),
            T::Keyword(Kw::If) => self.if_stmt()?,
            T::Keyword(Kw::Case) => self.case_stmt()?,
            T::Keyword(Kw::While) => {
                // `while cond do stmt` (p. 4-208).
                self.bump();
                let cond = self.expr()?;
                self.expect_kw(Kw::Do)?;
                let body = Box::new(self.stmt()?);
                StmtKind::While { cond, body }
            }
            T::Keyword(Kw::Repeat) => {
                // `repeat stmt; ... until cond` (p. 4-161).
                self.bump();
                let body = self.stmt_seq(|p| p.at_kw(Kw::Until));
                self.expect_kw(Kw::Until)?;
                let cond = self.expr()?;
                StmtKind::Repeat { body, cond }
            }
            T::Keyword(Kw::For) => self.for_stmt()?,
            T::Keyword(Kw::With) => self.with_stmt()?,
            T::Keyword(Kw::Goto) => {
                // `goto lbl` (p. 4-95).
                self.bump();
                StmtKind::Goto(self.label()?)
            }
            // `exit`, `next`, `return` take no arguments (pp. 4-77, 4-123,
            // 4-166).  They are predeclared identifiers, so only a bare
            // word followed by the end of the statement (or by the
            // `otherwise` of an enclosing case, which `case_stmt` accepts
            // without a `;`) is the statement.
            T::Ident(s)
                if matches!(s.as_str(), "exit" | "next" | "return")
                    && (matches!(
                        self.peek_at(1),
                        T::Semi | T::Eof | T::Keyword(Kw::End | Kw::Else | Kw::Until)
                    ) || matches!(self.peek_at(1), T::Ident(o) if o == "otherwise")) =>
            {
                let k = match s.as_str() {
                    "exit" => StmtKind::Exit,
                    "next" => StmtKind::Next,
                    _ => StmtKind::Return,
                };
                self.bump();
                k
            }
            T::Ident(_) => {
                let target = self.postfix()?;
                if self.eat(&T::Assign) {
                    let value = self.expr()?;
                    StmtKind::Assign { target, value }
                } else {
                    StmtKind::Call(target)
                }
            }
            _ => return self.expected("statement"),
        };
        Ok(Stmt {
            kind,
            span: self.since(start),
        })
    }

    /// `if cond then stmt [else stmt]` (p. 4-99).
    fn if_stmt(&mut self) -> PResult<StmtKind> {
        self.expect_kw(Kw::If)?;
        let cond = self.expr()?;
        self.expect_kw(Kw::Then)?;
        let then_branch = Box::new(self.stmt()?);
        let else_branch = if self.eat_kw(Kw::Else) {
            Some(Box::new(self.stmt()?))
        } else {
            None
        };
        Ok(StmtKind::If {
            cond,
            then_branch,
            else_branch,
        })
    }

    /// `case expr of constantlist : stmt; ... [otherwise stmt_list] end`
    /// (pp. 4-36..4-37; no colon after `otherwise`, p. 4-37).
    fn case_stmt(&mut self) -> PResult<StmtKind> {
        self.expect_kw(Kw::Case)?;
        let expr = self.expr()?;
        self.expect_kw(Kw::Of)?;
        let mut arms = Vec::new();
        let mut otherwise = None;
        loop {
            if self.at_kw(Kw::End) {
                break;
            }
            if self.at_ident("otherwise") {
                self.bump();
                otherwise = Some(self.stmt_seq(|p| p.at_kw(Kw::End)));
                break;
            }
            let start = self.span();
            let at = self.pos;
            let arm = (|| -> PResult<CaseArm> {
                let labels = self.expr_list()?;
                if self.at(&T::DotDot) {
                    // TODO(ref-question): p. 4-36 defines a constantlist as
                    // "One or more values (separated by commas)"; ranges
                    // are not mentioned.  Conservative: rejected.
                    let span = self.span();
                    self.error_at(span, "ranges are not allowed in case labels (ref p. 4-36)");
                    return Err(());
                }
                self.expect(&T::Colon)?;
                let stmt = self.stmt()?;
                Ok(CaseArm {
                    labels,
                    stmt,
                    span: self.since(start),
                })
            })();
            match arm {
                Ok(a) => arms.push(a),
                Err(()) => self.recover_stmt(at),
            }
            if !self.eat(&T::Semi) && !self.at_ident("otherwise") {
                break;
            }
        }
        self.expect_kw(Kw::End)?;
        Ok(StmtKind::Case {
            expr,
            arms,
            otherwise,
        })
    }

    /// `for v := e1 to|downto e2 do stmt`.
    fn for_stmt(&mut self) -> PResult<StmtKind> {
        self.expect_kw(Kw::For)?;
        let var = self.ident()?;
        self.expect(&T::Assign)?;
        let from = self.expr()?;
        let dir = if self.eat_kw(Kw::To) {
            ForDir::To
        } else if self.eat_kw(Kw::Downto) {
            ForDir::Downto
        } else {
            return self.expected("`to` or `downto`");
        };
        let to = self.expr()?;
        self.expect_kw(Kw::Do)?;
        let body = Box::new(self.stmt()?);
        Ok(StmtKind::For {
            var,
            from,
            dir,
            to,
            body,
        })
    }

    /// `with v1 [: id1], ..., vN [: idN] do stmt` (pp. 4-211..4-212).
    fn with_stmt(&mut self) -> PResult<StmtKind> {
        self.expect_kw(Kw::With)?;
        let mut items = Vec::new();
        loop {
            let record = self.postfix()?;
            let alias = if self.eat(&T::Colon) {
                Some(self.ident()?)
            } else {
                None
            };
            items.push(WithItem { record, alias });
            if !self.eat(&T::Comma) {
                break;
            }
        }
        self.expect_kw(Kw::Do)?;
        let body = Box::new(self.stmt()?);
        Ok(StmtKind::With { items, body })
    }

    // -----------------------------------------------------------------
    // Expressions (precedence: Table 4-3, p. 4-4, checked in the PDF)
    //
    //   not                      highest   (`~` grouped here, see below)
    //   **
    //   & * / div mod and
    //   ! + - or
    //   = <> < > <= >= in
    //   and then
    //   or else                  lowest
    // -----------------------------------------------------------------

    fn expr_list(&mut self) -> PResult<Vec<Expr>> {
        let mut v = vec![self.expr()?];
        while self.eat(&T::Comma) {
            v.push(self.expr()?);
        }
        Ok(v)
    }

    fn binary(lhs: Expr, op: BinOp, rhs: Expr) -> Expr {
        let span = join(lhs.span, rhs.span);
        Expr {
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            span,
        }
    }

    /// expr = and_then {`or else` and_then}
    fn expr(&mut self) -> PResult<Expr> {
        self.nested(Self::expr_inner)
    }

    fn expr_inner(&mut self) -> PResult<Expr> {
        self.chain(Self::or_else_chain)
    }

    fn or_else_chain(&mut self) -> PResult<Expr> {
        let mut lhs = self.and_then()?;
        while self.at_kw(Kw::Or) && matches!(self.peek_at(1), T::Keyword(Kw::Else)) {
            self.link()?;
            self.bump();
            self.bump();
            let rhs = self.and_then()?;
            lhs = Self::binary(lhs, BinOp::OrElse, rhs);
        }
        Ok(lhs)
    }

    /// and_then = relation {`and then` relation}
    fn and_then(&mut self) -> PResult<Expr> {
        self.chain(Self::and_then_chain)
    }

    fn and_then_chain(&mut self) -> PResult<Expr> {
        let mut lhs = self.relation()?;
        while self.at_kw(Kw::And) && matches!(self.peek_at(1), T::Keyword(Kw::Then)) {
            self.link()?;
            self.bump();
            self.bump();
            let rhs = self.relation()?;
            lhs = Self::binary(lhs, BinOp::AndThen, rhs);
        }
        Ok(lhs)
    }

    /// relation = simple [relop simple].  Relations do not chain (standard
    /// Pascal).
    fn relation(&mut self) -> PResult<Expr> {
        let lhs = self.simple()?;
        let op = match self.peek() {
            T::Eq => BinOp::Eq,
            T::NotEq => BinOp::Ne,
            T::Lt => BinOp::Lt,
            T::Le => BinOp::Le,
            T::Gt => BinOp::Gt,
            T::Ge => BinOp::Ge,
            T::Keyword(Kw::In) => BinOp::In,
            _ => return Ok(lhs),
        };
        self.bump();
        let rhs = self.simple()?;
        Ok(Self::binary(lhs, op, rhs))
    }

    /// simple = [+|-] term {(! | + | - | or) term}.  The sign applies to
    /// the first term (standard Pascal).
    ///
    /// `a * -b` is rejected: "You cannot follow a binary operator with a
    /// unary operator of lower precedence" (p. 4-82: `9 DIV -3`, `5 * -100`
    /// are improper).
    ///
    /// TODO(ref-question): Table 4-3 (p. 4-4) does not list unary minus,
    /// so whether `a + -b` (same level) is allowed is open.  Conservative
    /// choice: standard Pascal, rejected.
    fn simple(&mut self) -> PResult<Expr> {
        self.chain(Self::simple_chain)
    }

    fn simple_chain(&mut self) -> PResult<Expr> {
        let start = self.span();
        let sign = match self.peek() {
            T::Minus => Some(UnOp::Neg),
            T::Plus => Some(UnOp::Plus),
            _ => None,
        };
        if sign.is_some() {
            self.bump();
        }
        let mut lhs = self.term()?;
        if let Some(op) = sign {
            lhs = Expr {
                span: join(start, lhs.span),
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(lhs),
                },
            };
        }
        loop {
            let op = match self.peek() {
                T::Bang => BinOp::BitOr,
                T::Plus => BinOp::Add,
                T::Minus => BinOp::Sub,
                T::Keyword(Kw::Or) if !matches!(self.peek_at(1), T::Keyword(Kw::Else)) => BinOp::Or,
                _ => break,
            };
            self.link()?;
            self.bump();
            let rhs = self.term()?;
            lhs = Self::binary(lhs, op, rhs);
        }
        Ok(lhs)
    }

    /// term = power {(& | * | / | div | mod | and) power}
    fn term(&mut self) -> PResult<Expr> {
        self.chain(Self::term_chain)
    }

    fn term_chain(&mut self) -> PResult<Expr> {
        let mut lhs = self.power()?;
        loop {
            let op = match self.peek() {
                T::Amp => BinOp::BitAnd,
                T::Star => BinOp::Mul,
                T::Slash => BinOp::RealDiv,
                T::Keyword(Kw::Div) => BinOp::Div,
                T::Keyword(Kw::Mod) => BinOp::Mod,
                T::Keyword(Kw::And) if !matches!(self.peek_at(1), T::Keyword(Kw::Then)) => {
                    BinOp::And
                }
                _ => break,
            };
            self.link()?;
            self.bump();
            let rhs = self.power()?;
            lhs = Self::binary(lhs, op, rhs);
        }
        Ok(lhs)
    }

    /// power = unary [** power].  Right-associative: `x ** y ** z` is
    /// `x ** (y ** z)` (Table 4-2, p. 4-4).
    fn power(&mut self) -> PResult<Expr> {
        let lhs = self.unary()?;
        if self.eat(&T::StarStar) {
            let rhs = self.nested(Self::power)?;
            return Ok(Self::binary(lhs, BinOp::Pow, rhs));
        }
        Ok(lhs)
    }

    /// unary = (not | ~) unary | postfix.
    ///
    /// TODO(ref-question): Table 4-3 (p. 4-4) does not list `~` (bit not,
    /// p. 4-33).  Conservative choice: it binds like `not`.
    fn unary(&mut self) -> PResult<Expr> {
        let start = self.span();
        let op = match self.peek() {
            T::Keyword(Kw::Not) => UnOp::Not,
            T::Tilde => UnOp::BitNot,
            _ => return self.postfix(),
        };
        self.bump();
        let operand = self.nested(Self::unary)?;
        Ok(Expr {
            span: join(start, operand.span),
            kind: ExprKind::Unary {
                op,
                operand: Box::new(operand),
            },
        })
    }

    /// postfix = primary {`^` | `.` field | `[` exprs `]` | `(` args `)`}
    fn postfix(&mut self) -> PResult<Expr> {
        self.chain(Self::postfix_chain)
    }

    fn postfix_chain(&mut self) -> PResult<Expr> {
        let mut e = self.primary()?;
        let start = e.span;
        loop {
            if matches!(self.peek(), T::Caret | T::Dot | T::LBracket)
                || (matches!(self.peek(), T::LParen) && !matches!(e.kind, ExprKind::Str(_)))
            {
                // Each selector nests the tree one level, like an operator.
                self.link()?;
            }
            let kind = match self.peek() {
                T::Caret => {
                    self.bump();
                    ExprKind::Deref(Box::new(e))
                }
                T::Dot => {
                    self.bump();
                    let field = self.ident()?;
                    ExprKind::Field {
                        base: Box::new(e),
                        field,
                    }
                }
                T::LBracket => {
                    self.bump();
                    let indices = self.expr_list()?;
                    self.expect(&T::RBracket)?;
                    ExprKind::Index {
                        base: Box::new(e),
                        indices,
                    }
                }
                T::LParen if !matches!(e.kind, ExprKind::Str(_)) => {
                    self.bump();
                    let args = self.expr_list()?;
                    self.expect(&T::RParen)?;
                    ExprKind::Call {
                        callee: Box::new(e),
                        args,
                    }
                }
                _ => break,
            };
            e = Expr {
                kind,
                span: self.since(start),
            };
        }
        Ok(e)
    }

    fn primary(&mut self) -> PResult<Expr> {
        let start = self.span();
        let kind = match self.peek() {
            T::Int(n) => {
                self.bump();
                ExprKind::Int(*n)
            }
            T::Real(s) => {
                self.bump();
                ExprKind::Real(s.clone())
            }
            T::Str(_) => ExprKind::Str(self.string()?),
            T::Ident(_) => ExprKind::Name(self.ident()?),
            T::LParen => {
                self.bump();
                let e = self.expr()?;
                self.expect(&T::RParen)?;
                ExprKind::Paren(Box::new(e))
            }
            T::LBracket => {
                // Set constructor `[a, b..c]`, possibly empty.
                self.bump();
                let mut elems = Vec::new();
                if !self.at(&T::RBracket) {
                    loop {
                        let lo = self.expr()?;
                        let hi = if self.eat(&T::DotDot) {
                            Some(self.expr()?)
                        } else {
                            None
                        };
                        elems.push(SetElem { lo, hi });
                        if !self.eat(&T::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&T::RBracket)?;
                ExprKind::Set(elems)
            }
            _ => return self.expected("expression"),
        };
        Ok(Expr {
            kind,
            span: self.since(start),
        })
    }

    /// A string literal with embedded character codes: `'text' (13)(CR, LF)
    /// 'more'` (ref 2.1.5.1, pp. 2-4..2-5).  The first component must be a
    /// quoted string (p. 2-5).
    fn string(&mut self) -> PResult<Vec<StrPart>> {
        let mut parts = Vec::new();
        loop {
            match self.peek() {
                T::Str(s) => {
                    self.bump();
                    parts.push(StrPart::Text(s.clone()));
                }
                T::LParen if !parts.is_empty() => {
                    self.bump();
                    for e in self.expr_list()? {
                        parts.push(StrPart::Code(e));
                    }
                    self.expect(&T::RParen)?;
                }
                _ => break,
            }
        }
        Ok(parts)
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
