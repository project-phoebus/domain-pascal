//! `%` compiler directives (ref 4.7 "Compiler Directives", pp. 4-43..4-58).
//!
//! Token-level preprocessing.  The preprocessor pulls tokens from a stack
//! of [`Lexer`]s (one per open `%include` file), acts on
//! [`TokenKind::Directive`] tokens, and returns the flat token stream the
//! parser reads.  Every token keeps the span of the file it was lexed from,
//! so diagnostics point into the right include file.  The output never
//! contains `Directive` tokens; it ends with exactly one `Eof`.
//!
//! What each directive does here:
//!
//! | directive | effect |
//! |---|---|
//! | `%include 'p';` | tokens of file `p` are spliced in (p. 4-55; search order ref 6.4.13, p. 6-20) |
//! | `%if`/`%ifdef`/`%elseif`/`%elseifdef` *pred* `%then`, `%else`, `%endif` | conditional compilation (pp. 4-45..4-50) |
//! | `%var n...`, `%enable n...` | declare / set conditional variables (p. 4-50) |
//! | `%error 's'`, `%warning 's'` | a diagnostic (pp. 4-51..4-52) |
//! | `%exit` | stop reading the current file (p. 4-52) |
//! | `%debug;` | without [`Options::cond`], drop the rest of the line (pp. 4-54..4-55; ref 6.4.7, p. 6-12) |
//! | `%natural_alignment;` `%word_alignment;` `%push_alignment;` `%pop_alignment;` `%begin_inline;` `%end_inline;` `%begin_noinline;` `%end_noinline;` | passed on as [`TokenKind::Pragma`] |
//! | `%list;` `%nolist;` `%eject;` | dropped: they only affect the listing file (pp. 4-55..4-56), which we do not produce |
//! | `%slibrary 'p';` | rejected: precompiled libraries are not supported (p. 4-56) |
//!
//! Directives may be written as statements (`%name`) or as the first thing
//! in a comment (`{%name}`, p. 4-43), except the conditional ("-config")
//! directives, which "cannot be used as comments" (p. 4-45).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::diag::Diagnostic;
use crate::lexer::{DirectiveForm, Keyword, Lexer, Pragma, Token, TokenKind};
use crate::source::{FileId, SourceMap, Span};

/// Preprocessor options.
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// `%include` search directories (`-idir`, here `-I`), in order.
    pub include_dirs: Vec<PathBuf>,
    /// The "current working directory" searched first for relative
    /// include names (ref 6.4.13, p. 6-20).  `None` means the process's
    /// working directory.  Relative `include_dirs` are taken relative to it.
    pub cwd: Option<PathBuf>,
    /// `-config var1 ... varN` (ref 6.4.8, p. 6-12).  `None` when the option
    /// was not given, which also makes `%config` false (p. 4-50).
    pub config: Option<Vec<String>>,
    /// `-cond`: compile `%debug` lines (ref 6.4.7, p. 6-12).  Default off
    /// (`-ncond`).
    pub cond: bool,
}

/// Preprocessor result.  `tokens` ends with `Eof`.  Compilation should stop
/// if [`Output::has_errors`]; warnings (from `%warning`) may be present
/// either way.
#[derive(Debug)]
pub struct Output {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Output {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// Run the preprocessor over `file`.  `%include`d files are loaded into
/// `sm`.
pub fn preprocess(sm: &mut SourceMap, file: FileId, opts: &Options) -> Output {
    let mut vars = HashMap::new();
    // Predeclared conditional variables (p. 4-46).  We model the SR10 m68k
    // compiler: it "generates COFF files whenever it compiles" (p. 4-46)
    // and generates 68000 code.
    for (n, v) in [
        ("_bfmt__coff", true),
        ("_isp__m68k", true),
        ("_isp__a88k", false),
    ] {
        vars.insert(
            n.to_string(),
            Var {
                enabled: v,
                predeclared: true,
            },
        );
    }
    let config = opts
        .config
        .iter()
        .flatten()
        .map(|s| s.to_ascii_lowercase())
        .collect();
    let key = std::fs::canonicalize(&sm.file(file).path).ok();
    let lexer = Lexer::new(sm, file);
    let mut pp = Pp {
        sm,
        opts,
        files: vec![FileState { lexer, key }],
        frames: Vec::new(),
        vars,
        config,
        out: Vec::new(),
        diags: Vec::new(),
        debug_skip: None,
    };
    pp.run();
    Output {
        tokens: pp.out,
        diagnostics: pp.diags,
    }
}

struct FileState {
    lexer: Lexer,
    /// Canonical path, for include-cycle detection.
    key: Option<PathBuf>,
}

/// One open `%if` ... `%endif`.
struct Frame {
    /// Index in the include stack of the file that opened it.
    depth: usize,
    span: Span,
    parent_active: bool,
    /// Some branch has been selected (so later `%elseif`/`%else` are off).
    taken: bool,
    active: bool,
    seen_else: bool,
}

struct Var {
    enabled: bool,
    predeclared: bool,
}

/// The conditional ("-config") directives (p. 4-45).
const CONDITIONAL: [&str; 13] = [
    "if",
    "then",
    "elseif",
    "else",
    "endif",
    "ifdef",
    "elseifdef",
    "var",
    "enable",
    "config",
    "error",
    "warning",
    "exit",
];

struct Pp<'a> {
    sm: &'a mut SourceMap,
    opts: &'a Options,
    files: Vec<FileState>,
    frames: Vec<Frame>,
    vars: HashMap<String, Var>,
    config: HashSet<String>,
    out: Vec<Token>,
    diags: Vec<Diagnostic>,
    /// `%debug` without `-cond`: drop the tokens on this line of this file.
    debug_skip: Option<(FileId, u32)>,
}

type PredResult = Result<bool, (Span, String)>;

impl Pp<'_> {
    fn active(&self) -> bool {
        self.frames.last().is_none_or(|f| f.active)
    }

    fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic::error(span, msg));
    }

    fn run(&mut self) {
        while let Some(mut fs) = self.files.pop() {
            let tok = fs.lexer.next_token();
            let ld = fs.lexer.take_diagnostics();
            let mut debug_skipped = false;
            if let Some((f, line)) = self.debug_skip {
                if tok.kind != TokenKind::Eof
                    && tok.span.file == f
                    && self.sm.line_of(tok.span) == line
                {
                    debug_skipped = true;
                } else {
                    self.debug_skip = None;
                }
            }
            // Without -cond, %debug lines are treated "as comments" (ref
            // 6.4.7, p. 6-12), so their lexer errors are not reported.
            if self.active() && !debug_skipped {
                self.diags.extend(ld);
            }
            if debug_skipped {
                self.files.push(fs);
                continue;
            }
            match tok.kind {
                TokenKind::Eof => self.end_of_file(tok),
                TokenKind::Directive { .. } => self.directive(fs, tok),
                _ => {
                    if self.active() {
                        self.out.push(tok);
                    }
                    self.files.push(fs);
                }
            }
        }
    }

    /// The file at depth `self.files.len()` has ended (it is already
    /// popped).
    fn end_of_file(&mut self, eof: Token) {
        let depth = self.files.len();
        while self.frames.last().is_some_and(|f| f.depth >= depth) {
            let f = self.frames.pop().unwrap();
            self.error(f.span, "unterminated conditional: missing %endif");
        }
        if self.files.is_empty() {
            self.out.push(eof);
        }
    }

    /// The innermost `%if` frame, if it was opened in the current file.
    fn frame_here(&mut self, depth: usize, span: Span, name: &str) -> Option<usize> {
        match self.frames.last() {
            Some(f) if f.depth == depth => Some(self.frames.len() - 1),
            _ => {
                self.error(span, format!("%{name} without a matching %if in this file"));
                None
            }
        }
    }

    /// Handle one directive.  `fs` (the file it came from, popped off the
    /// stack) must be pushed back unless the directive ends the file.
    fn directive(&mut self, mut fs: FileState, tok: Token) {
        let TokenKind::Directive { name, form } = tok.kind.clone() else {
            unreachable!()
        };
        let depth = self.files.len();
        let active = self.active();
        let span = tok.span;
        let mut sub = match form {
            DirectiveForm::Comment { body } => {
                Some(Lexer::new_range(self.sm, body.file, body.lo, body.hi))
            }
            DirectiveForm::Statement => None,
        };
        let in_comment = sub.is_some();
        let conditional = CONDITIONAL.contains(&name.as_str());
        if in_comment && conditional {
            // p. 4-45: "Unlike the other compiler directives, conditional
            // directives cannot be used as comments."
            // TODO(ref-question): is such a comment an error or silently a
            // plain comment?  We reject it.
            if active {
                self.error(
                    span,
                    format!("conditional directive %{name} cannot be written inside a comment"),
                );
            }
            self.files.push(fs);
            return;
        }
        let lx: &mut Lexer = match sub.as_mut() {
            Some(s) => s,
            None => &mut fs.lexer,
        };
        let mut include: Option<(PathBuf, Span)> = None;
        let mut exit = false;
        match name.as_str() {
            // ---- conditional compilation, pp. 4-47..4-50 ----
            "if" | "ifdef" => {
                let v = self.predicate(lx, name == "ifdef", active, span);
                self.frames.push(Frame {
                    depth,
                    span,
                    parent_active: active,
                    taken: active && v,
                    active: active && v,
                    seen_else: false,
                });
            }
            "elseif" | "elseifdef" => {
                let fi = self.frame_here(depth, span, &name);
                if let Some(i) = fi {
                    if self.frames[i].seen_else {
                        self.error(span, format!("%{name} after %else"));
                    }
                }
                let eval =
                    fi.is_some_and(|i| self.frames[i].parent_active && !self.frames[i].taken);
                let v = self.predicate(lx, name == "elseifdef", eval, span);
                if let Some(i) = fi {
                    let f = &mut self.frames[i];
                    f.active = eval && v;
                    f.taken |= f.active;
                }
            }
            "else" => {
                if let Some(i) = self.frame_here(depth, span, "else") {
                    if self.frames[i].seen_else {
                        self.error(span, "second %else for the same %if");
                    }
                    let f = &mut self.frames[i];
                    f.active = f.parent_active && !f.taken;
                    f.taken = true;
                    f.seen_else = true;
                }
            }
            "endif" => {
                if self.frame_here(depth, span, "endif").is_some() {
                    self.frames.pop();
                }
                // The reference writes `%ENDIF;` (p. 4-47); the `;` belongs
                // to the directive.
                self.semi(lx, false, in_comment, &name, span);
            }
            // Everything else has no effect in a skipped region (p. 4-52
            // for %exit, p. 4-55 for %include).
            _ if !active => {}
            "var" => {
                for (n, nspan) in self.names(lx, span, &name) {
                    let enabled = self.config.contains(&n);
                    match self.vars.entry(n) {
                        // p. 4-50.
                        std::collections::hash_map::Entry::Occupied(e) => {
                            let m =
                                format!("conditional variable `{}` is already declared", e.key());
                            self.error(nspan, m);
                        }
                        std::collections::hash_map::Entry::Vacant(e) => {
                            e.insert(Var {
                                enabled,
                                predeclared: false,
                            });
                        }
                    }
                }
                self.semi(lx, false, in_comment, &name, span);
            }
            "enable" => {
                for (n, nspan) in self.names(lx, span, &name) {
                    let msg = match self.vars.get_mut(&n) {
                        None => Some(format!(
                            "conditional variable `{n}` must be declared with %var before %enable"
                        )),
                        // p. 4-48: predeclared variables cannot be enabled.
                        Some(v) if v.predeclared => Some(format!(
                            "predeclared conditional variable `{n}` cannot be enabled"
                        )),
                        // p. 4-50: setting a variable true twice is an error.
                        Some(v) if v.enabled => {
                            Some(format!("conditional variable `{n}` is already set to true"))
                        }
                        Some(v) => {
                            v.enabled = true;
                            None
                        }
                    };
                    if let Some(m) = msg {
                        self.error(nspan, m);
                    }
                }
                self.semi(lx, false, in_comment, &name, span);
            }
            "error" | "warning" => {
                // pp. 4-51..4-52.
                if let Some(s) = self.expect_str(lx, &name, span) {
                    self.diags.push(if name == "error" {
                        Diagnostic::error(span, s)
                    } else {
                        Diagnostic::warning(span, s)
                    });
                }
                self.semi(lx, false, in_comment, &name, span);
            }
            "exit" => {
                // p. 4-52: stop reading this file.
                self.semi(lx, false, in_comment, &name, span);
                exit = true;
            }
            "then" => self.error(span, "%then without %if or %elseif"),
            "config" => self.error(span, "%config can only be used in a predicate"),
            "include" => {
                if let Some(p) = self.expect_str(lx, &name, span) {
                    let sspan = span;
                    match self.resolve(&p) {
                        Ok(path) => include = Some((path, sspan)),
                        Err(m) => self.error(span, m),
                    }
                }
                self.semi(lx, true, in_comment, &name, span);
            }
            "slibrary" => {
                self.expect_str(lx, &name, span);
                self.semi(lx, true, in_comment, &name, span);
                self.error(span, "%slibrary (precompiled libraries) is not supported");
            }
            "debug" => {
                self.semi(lx, true, in_comment, &name, span);
                if !self.opts.cond {
                    // pp. 4-54..4-55: "applies to one physical line";
                    // text to the right of it is conditionally compiled.
                    self.debug_skip = Some((span.file, self.sm.line_of(span)));
                }
            }
            "list" | "nolist" | "eject" => {
                // Listing-file only (pp. 4-55..4-56).  TODO: no listing.
                self.semi(lx, true, in_comment, &name, span);
            }
            _ => match pragma(&name) {
                Some(p) => {
                    // push/pop are not in the "must place a semicolon"
                    // list on p. 4-43, so their `;` is optional.
                    let required = !matches!(p, Pragma::PushAlignment | Pragma::PopAlignment);
                    self.semi(lx, required, in_comment, &name, span);
                    self.out.push(Token {
                        kind: TokenKind::Pragma(p),
                        span,
                    });
                }
                None => self.error(span, format!("unknown compiler directive %{name}")),
            },
        }
        if active {
            if let Some(s) = sub.as_mut() {
                let t = s.next_token();
                if t.kind != TokenKind::Eof {
                    self.error(t.span, format!("unexpected text after %{name} in comment"));
                }
            }
            let ld = lx_diags(&mut fs, sub.as_mut());
            self.diags.extend(ld);
        } else {
            lx_diags(&mut fs, sub.as_mut());
        }
        if exit {
            self.frames.retain(|f| f.depth < depth);
            if self.files.is_empty() {
                self.out.push(Token {
                    kind: TokenKind::Eof,
                    span: Span::new(span.file, span.hi, span.hi),
                });
            }
            return;
        }
        self.files.push(fs);
        if let Some((path, ispan)) = include {
            self.open_include(&path, ispan);
        }
    }

    fn open_include(&mut self, path: &Path, span: Span) {
        let key = std::fs::canonicalize(path).ok();
        if key.is_some() && self.files.iter().any(|f| f.key == key) {
            self.error(
                span,
                format!("include cycle: {} is already being read", path.display()),
            );
            return;
        }
        match self.sm.load(path) {
            Ok(id) => {
                self.sm.set_included_from(id, span);
                let lexer = Lexer::new(self.sm, id);
                self.files.push(FileState { lexer, key });
            }
            Err(e) => self.error(span, format!("cannot read {}: {e}", path.display())),
        }
    }

    /// Include search, ref 6.4.13, p. 6-20: names beginning with `/`,
    /// `//`, `~` or `.` are absolute; others are tried in the working
    /// directory, then in each `-idir` directory, in order.  The reference
    /// does not search the including file's directory.
    fn resolve(&self, name: &str) -> Result<PathBuf, String> {
        let base = self.opts.cwd.as_deref().unwrap_or(Path::new(""));
        if name.starts_with('~') {
            // `~` is the Aegis naming directory; nothing maps it here.
            return Err(format!(
                "include file `{name}`: `~` pathnames are not supported"
            ));
        }
        // TODO: `//node/...` network paths and `/sys/ins/...` are taken as
        // host paths unchanged.
        let candidates: Vec<PathBuf> = if name.starts_with('/') || name.starts_with('.') {
            vec![base.join(name)]
        } else {
            std::iter::once(base.join(name))
                .chain(
                    self.opts
                        .include_dirs
                        .iter()
                        .map(|d| base.join(d).join(name)),
                )
                .collect()
        };
        candidates
            .into_iter()
            .find(|p| p.is_file())
            .ok_or_else(|| format!("include file `{name}` not found"))
    }

    /// Consume a `;` after a directive.  `required` lists the directives the
    /// reference says need one when written as statements (p. 4-43); inside
    /// a comment it is never needed.
    fn semi(&mut self, lx: &mut Lexer, required: bool, in_comment: bool, name: &str, span: Span) {
        if lx.peek().kind == TokenKind::Semi {
            lx.next_token();
        } else if required && !in_comment {
            self.error(span, format!("expected `;` after %{name}"));
        }
    }

    fn expect_str(&mut self, lx: &mut Lexer, name: &str, span: Span) -> Option<String> {
        // Peek, not next+seek: a token lexed and then pushed back would
        // report its lexer diagnostics twice.
        if let TokenKind::Str(s) = lx.peek().kind {
            lx.next_token();
            return Some(s);
        }
        self.error(span, format!("%{name} needs a quoted string"));
        None
    }

    /// The names after `%var` / `%enable` (p. 4-50).
    /// TODO(ref-question): the reference never says how the list ends; we
    /// take the names on the directive's own line.
    fn names(&mut self, lx: &mut Lexer, span: Span, name: &str) -> Vec<(String, Span)> {
        let line = self.sm.line_of(span);
        let mut out = Vec::new();
        lx.set_cond_names(true);
        loop {
            let t = lx.peek();
            match t.kind {
                TokenKind::Ident(n) if self.sm.line_of(t.span) == line => {
                    lx.next_token();
                    out.push((n, t.span));
                }
                _ => break,
            }
        }
        lx.set_cond_names(false);
        if out.is_empty() {
            self.error(span, format!("%{name} needs at least one variable name"));
        }
        out
    }

    /// Read `predicate %then` and evaluate it when `eval` (p. 4-46).  In a
    /// skipped region it is only scanned for `%then`.
    fn predicate(&mut self, lx: &mut Lexer, ifdef: bool, eval: bool, span: Span) -> bool {
        lx.set_cond_names(true);
        let mut toks = Vec::new();
        let mut found_then = false;
        loop {
            let t = lx.next_token();
            match &t.kind {
                TokenKind::Directive {
                    name,
                    form: DirectiveForm::Statement,
                } if name == "then" => {
                    found_then = true;
                    break;
                }
                TokenKind::Directive { name, .. } if name == "config" => toks.push(t),
                TokenKind::Directive { .. } | TokenKind::Eof => {
                    lx.seek(t.span.lo);
                    break;
                }
                _ => toks.push(t),
            }
        }
        lx.set_cond_names(false);
        if !found_then {
            if eval {
                self.error(span, "missing %then after predicate");
            }
            return false;
        }
        if !eval {
            return false;
        }
        let mut p = Pred {
            toks: &toks,
            i: 0,
            vars: &self.vars,
            config: self.opts.config.is_some(),
            ifdef,
            end: span,
        };
        let r = p.or().and_then(|v| match toks.get(p.i) {
            None => Ok(v),
            Some(t) => Err((t.span, "unexpected token in predicate".to_string())),
        });
        match r {
            Ok(v) => v,
            Err((sp, m)) => {
                self.error(sp, m);
                false
            }
        }
    }
}

fn lx_diags(fs: &mut FileState, sub: Option<&mut Lexer>) -> Vec<Diagnostic> {
    let mut d = fs.lexer.take_diagnostics();
    if let Some(s) = sub {
        d.extend(s.take_diagnostics());
    }
    d
}

fn pragma(name: &str) -> Option<Pragma> {
    Some(match name {
        "natural_alignment" => Pragma::NaturalAlignment,
        "word_alignment" => Pragma::WordAlignment,
        "push_alignment" => Pragma::PushAlignment,
        "pop_alignment" => Pragma::PopAlignment,
        "begin_inline" => Pragma::BeginInline,
        "end_inline" => Pragma::EndInline,
        "begin_noinline" => Pragma::BeginNoinline,
        "end_noinline" => Pragma::EndNoinline,
        _ => return None,
    })
}

/// Predicate evaluator (p. 4-46): names, `not`, `and`, `or`, parentheses
/// and `%config`.
/// TODO(ref-question): the reference gives no precedence for predicates;
/// we use Pascal's (Table 4-3, p. 4-4): `not` > `and` > `or`.
struct Pred<'t> {
    toks: &'t [Token],
    i: usize,
    vars: &'t HashMap<String, Var>,
    config: bool,
    ifdef: bool,
    end: Span,
}

impl Pred<'_> {
    fn eat_kw(&mut self, k: Keyword) -> bool {
        if self
            .toks
            .get(self.i)
            .is_some_and(|t| t.kind == TokenKind::Keyword(k))
        {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn or(&mut self) -> PredResult {
        let mut v = self.and()?;
        while self.eat_kw(Keyword::Or) {
            let r = self.and()?;
            v = v || r;
        }
        Ok(v)
    }

    fn and(&mut self) -> PredResult {
        let mut v = self.not()?;
        while self.eat_kw(Keyword::And) {
            let r = self.not()?;
            v = v && r;
        }
        Ok(v)
    }

    fn not(&mut self) -> PredResult {
        if self.eat_kw(Keyword::Not) {
            return Ok(!self.not()?);
        }
        self.primary()
    }

    fn primary(&mut self) -> PredResult {
        let Some(t) = self.toks.get(self.i) else {
            return Err((self.end, "incomplete predicate".to_string()));
        };
        self.i += 1;
        match &t.kind {
            TokenKind::LParen => {
                let v = self.or()?;
                match self.toks.get(self.i) {
                    Some(t) if t.kind == TokenKind::RParen => {
                        self.i += 1;
                        Ok(v)
                    }
                    _ => Err((t.span, "missing `)` in predicate".to_string())),
                }
            }
            TokenKind::Directive { name, .. } if name == "config" => Ok(self.config),
            TokenKind::Ident(n) => match self.vars.get(n) {
                // %ifdef: true if declared; %if: true if enabled (p. 4-49).
                Some(v) => Ok(self.ifdef || v.enabled),
                None if self.ifdef => Ok(false),
                // p. 4-50: "You cannot use a name in a predicate unless you
                // first declare it with the %var directive."
                None => Err((
                    t.span,
                    format!("conditional variable `{n}` is not declared with %var"),
                )),
            },
            _ => Err((t.span, "unexpected token in predicate".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind as K;

    /// Scratch dir under the cargo target dir (never outside the repo).
    fn scratch(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-scratch/preprocess")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    struct Run {
        sm: SourceMap,
        out: Output,
    }

    impl Run {
        fn kinds(&self) -> Vec<K> {
            let mut k: Vec<K> = self.out.tokens.iter().map(|t| t.kind.clone()).collect();
            assert_eq!(k.pop(), Some(K::Eof));
            k
        }
        fn diags(&self) -> Vec<String> {
            self.out
                .diagnostics
                .iter()
                .map(|d| d.render(&self.sm, "dpc"))
                .collect()
        }
        /// Identifier names in the output.
        fn idents(&self) -> Vec<String> {
            self.kinds()
                .into_iter()
                .filter_map(|k| match k {
                    K::Ident(s) => Some(s),
                    _ => None,
                })
                .collect()
        }
    }

    fn run_opts(src: &str, opts: &Options) -> Run {
        let mut sm = SourceMap::new();
        let f = sm.add("t.pas", src.to_string());
        let out = preprocess(&mut sm, f, opts);
        Run { sm, out }
    }

    fn run(src: &str) -> Run {
        run_opts(src, &Options::default())
    }

    /// Identifiers of a run that must be clean.
    fn clean(src: &str) -> Vec<String> {
        let r = run(src);
        assert!(r.out.diagnostics.is_empty(), "{:?}", r.diags());
        r.idents()
    }

    fn errs(src: &str) -> Vec<String> {
        run(src).diags()
    }

    #[test]
    fn passes_tokens_through() {
        let r = run("module m; x := 1;");
        assert!(r.out.diagnostics.is_empty());
        assert_eq!(r.kinds().len(), 7);
        assert_eq!(r.out.tokens.last().unwrap().kind, K::Eof);
    }

    #[test]
    fn if_else_endif() {
        let src = "%var color mono\n%if color %then a %elseif mono %then b %else c %endif; d";
        assert_eq!(clean(src), ["c", "d"]);
        let opts = Options {
            config: Some(vec!["MONO".into()]),
            ..Default::default()
        };
        let r = run_opts(src, &opts);
        assert!(r.out.diagnostics.is_empty(), "{:?}", r.diags());
        assert_eq!(r.idents(), ["b", "d"]);
        let src2 = "%var color mono\n%enable color\n%if color %then a %elseif mono %then b %else c %endif d";
        assert_eq!(clean(src2), ["a", "d"]);
    }

    #[test]
    fn first_true_branch_wins() {
        // p. 4-48: if both are true the first branch is compiled.
        let src =
            "%var color mono\n%enable color mono\n%if color %then a %elseif mono %then b %endif";
        assert_eq!(clean(src), ["a"]);
    }

    #[test]
    fn nested_conditionals() {
        let src = "%var a b\n%enable a\n\
                   %if a %then x %if b %then y %else z %endif w %else \
                   %if a %then q %endif %endif e";
        assert_eq!(clean(src), ["x", "z", "w", "e"]);
        // Inner directives in a skipped region are only tracked.
        let src = "%var a\n%if a %then %if undeclared %then x %else y %endif %error 'no' %endif e";
        assert_eq!(clean(src), ["e"]);
    }

    #[test]
    fn predicates() {
        let pre = "%var a b\n%enable a\n";
        for (p, want) in [
            ("a", true),
            ("b", false),
            ("not(a)", false),
            ("not b", true),
            ("a and b", false),
            ("a or b", true),
            ("(a and not b)", true),
            ("b or a and b", false),
            ("not a or a", true),
            ("_isp__m68k", true),
            ("_ISP__A88K", false),
            ("_BFMT__COFF", true),
            ("%config", false),
        ] {
            let got = clean(&format!("{pre}%if {p} %then t %else f %endif"));
            assert_eq!(got, [if want { "t" } else { "f" }], "predicate {p}");
        }
        // Predicate and %then may be on different lines (p. 4-57).
        assert_eq!(clean("%if _ISP__A88K\n%then x\n%endif y"), ["y"]);
        let opts = Options {
            config: Some(vec![]),
            ..Default::default()
        };
        let r = run_opts("%if %config %then x %endif", &opts);
        assert_eq!(r.idents(), ["x"]);
    }

    #[test]
    fn dotted_names() {
        // p. 4-50: %VAR code.sr9 code.sr8 code.sr7; %ENABLE code.sr9 code.sr7
        let src = "%VAR code.sr9 code.sr8 code.sr7\n%ENABLE code.sr9 code.sr7\n\
                   %IF code.sr8 %THEN a %ELSEIF code.sr7 %THEN b %ENDIF;";
        assert_eq!(clean(src), ["b"]);
    }

    #[test]
    fn ifdef_and_elseifdef() {
        // pp. 4-49..4-50.
        let src = "%ifdef not(color) %then\n%var color\n%endif;\n\
                   %ifdef not(color) %then\n%var color\n%endif;\n\
                   %ifdef color %then a %endif\n\
                   %ifdef mono %then b %elseifdef not(mono) %then c %endif";
        assert_eq!(clean(src), ["a", "c"]);
        assert_eq!(clean("%ifdef _ISP__M68K %then a %endif"), ["a"]);
    }

    #[test]
    fn conditional_errors() {
        assert!(errs("%if x %then %endif")[0].contains("`x` is not declared with %var"));
        assert!(errs("%var x\n%var x")[0].contains("already declared"));
        assert!(errs("%var _isp__m68k")[0].contains("already declared"));
        assert!(errs("%var x\n%enable x\n%enable x")[0].contains("already set to true"));
        let opts = Options {
            config: Some(vec!["x".into()]),
            ..Default::default()
        };
        assert!(run_opts("%var x\n%enable x", &opts).diags()[0].contains("already set"));
        assert!(errs("%enable y")[0].contains("must be declared with %var"));
        assert!(errs("%enable _ISP__M68K")[0].contains("cannot be enabled"));
        assert!(errs("%endif")[0].contains("%endif without a matching %if"));
        assert!(errs("%else")[0].contains("without a matching %if"));
        assert!(errs("%elseif a %then")[0].contains("without a matching %if"));
        assert!(errs("%var a\n%if a %then %else %else %endif")[0].contains("second %else"));
        assert!(errs("%var a\n%if a %then %else %elseif a %then %endif")[0].contains("after %else"));
        assert_eq!(
            errs("%var a\n%if a %then x"),
            ["t.pas:2:1: error: unterminated conditional: missing %endif"]
        );
        assert!(errs("%var a\n%if a b")[0].contains("missing %then"));
        assert!(errs("%var a\n%if a b %then %endif")[0].contains("unexpected token"));
        assert!(errs("%var a\n%if (a %then %endif")[0].contains("missing `)`"));
        assert!(errs("%if %then %endif")[0].contains("incomplete predicate"));
        assert!(errs("%then")[0].contains("%then without"));
        assert!(errs("%config")[0].contains("only be used in a predicate"));
        assert!(errs("%var")[0].contains("at least one variable"));
        assert!(errs("%frobnicate;")[0].contains("unknown compiler directive %frobnicate"));
    }

    #[test]
    fn conditionals_not_allowed_in_comments() {
        // p. 4-45.
        assert!(errs("{%var x}")[0].contains("cannot be written inside a comment"));
        assert!(errs("(*%endif*)")[0].contains("cannot be written inside a comment"));
    }

    #[test]
    fn var_list_ends_at_line_end() {
        // `module` is a predeclared identifier, not a reserved word.
        assert_eq!(clean("%var color\nmodule m;"), ["module", "m"]);
    }

    #[test]
    fn error_and_warning() {
        let r = run("%error 'I have not finished.'\nx");
        assert_eq!(r.diags(), ["t.pas:1:1: error: I have not finished."]);
        let r = run("%var c\n%if c %then\n%else\n%WARNING 'You forgot -config.';\n%endif\nx");
        assert_eq!(r.diags(), ["t.pas:4:1: warning: You forgot -config."]);
        assert!(!r.out.has_errors());
        assert_eq!(r.idents(), ["x"]);
        assert!(errs("%error x")[0].contains("needs a quoted string"));
        // A rejected argument token is lexed (and diagnosed) only once.
        assert_eq!(
            errs("%error 2#9"),
            [
                "t.pas:1:1: error: %error needs a quoted string",
                "t.pas:1:10: error: `9` is not a base-2 digit"
            ]
        );
        assert_eq!(
            errs("%include 16#;")
                .iter()
                .filter(|d| d.contains("missing digits"))
                .count(),
            1
        );
    }

    #[test]
    fn exit_stops_the_file() {
        assert_eq!(clean("a %exit; b c"), ["a"]);
        // Inside an untaken branch %exit has no effect (p. 4-52).
        assert_eq!(clean("%var v\n%if v %then %exit %endif a"), ["a"]);
        // Open conditionals of the exited file are dropped silently.
        assert_eq!(clean("%if _isp__m68k %then a %exit b"), ["a"]);
    }

    #[test]
    fn debug_lines() {
        // pp. 4-54..4-55.
        let src = "a := 1;\n%DEBUG; writeln(a);\nb\nc %debug; d e\nf";
        assert_eq!(clean(src), ["a", "b", "c", "f"]);
        let opts = Options {
            cond: true,
            ..Default::default()
        };
        let r = run_opts(src, &opts);
        assert_eq!(r.idents(), ["a", "writeln", "a", "b", "c", "d", "e", "f"]);
        assert!(errs("%debug x")[0].contains("expected `;` after %debug"));
        // Without -cond the line is a comment (ref 6.4.7, p. 6-12): no
        // lexer errors from it.  With -cond they are reported.
        let src = "%debug; writeln('unterminated ?\nx";
        assert_eq!(clean(src), ["x"]);
        let r = run_opts(src, &opts);
        assert!(
            r.diags()[0].contains("unterminated string"),
            "{:?}",
            r.diags()
        );
    }

    #[test]
    fn pragmas_pass_through() {
        let r = run(
            "%natural_alignment; %word_alignment; %push_alignment %pop_alignment; \
             %begin_inline; %end_inline; %begin_noinline; %end_noinline; \
             {%natural_alignment} (*%push_alignment*)",
        );
        assert!(r.out.diagnostics.is_empty(), "{:?}", r.diags());
        use Pragma as P;
        assert_eq!(
            r.kinds(),
            [
                P::NaturalAlignment,
                P::WordAlignment,
                P::PushAlignment,
                P::PopAlignment,
                P::BeginInline,
                P::EndInline,
                P::BeginNoinline,
                P::EndNoinline,
                P::NaturalAlignment,
                P::PushAlignment,
            ]
            .map(K::Pragma)
        );
        assert_eq!(r.sm.span_text(r.out.tokens[8].span), "{%natural_alignment}");
        assert!(errs("%natural_alignment x")[0].contains("expected `;`"));
        // Skipped in an untaken branch.
        assert!(clean("%var v\n%if v %then %natural_alignment; %endif").is_empty());
    }

    #[test]
    fn listing_directives_are_dropped() {
        assert_eq!(
            clean("%list; a %nolist; b %eject; {%eject} c"),
            ["a", "b", "c"]
        );
        assert!(errs("%list")[0].contains("expected `;` after %list"));
        assert!(errs("{%list junk}")[0].contains("unexpected text after %list"));
    }

    #[test]
    fn slibrary_is_rejected() {
        assert!(errs("%slibrary 'x.plb';")[0].contains("not supported"));
    }

    #[test]
    fn include_and_search_path() {
        let dir = scratch("include");
        std::fs::create_dir_all(dir.join("ins")).unwrap();
        std::fs::create_dir_all(dir.join("ins2")).unwrap();
        std::fs::write(dir.join("ins/a.ins.pas"), "a1 %include 'b.ins.pas'; a2").unwrap();
        std::fs::write(dir.join("ins2/b.ins.pas"), "b1").unwrap();
        // Same name in cwd and -idir: the working directory wins (p. 6-20).
        std::fs::write(dir.join("ins/c.ins.pas"), "c_idir").unwrap();
        std::fs::write(dir.join("c.ins.pas"), "c_cwd").unwrap();
        let opts = Options {
            include_dirs: vec!["ins".into(), dir.join("ins2")],
            cwd: Some(dir.clone()),
            ..Default::default()
        };
        let r = run_opts(
            "m1 %INCLUDE 'a.ins.pas'; {%include 'c.ins.pas'} %include './ins2/b.ins.pas'; m2",
            &opts,
        );
        assert!(r.out.diagnostics.is_empty(), "{:?}", r.diags());
        assert_eq!(r.idents(), ["m1", "a1", "b1", "a2", "c_cwd", "b1", "m2"]);
        // Spans point into the included file.
        let a1 = &r.out.tokens[1];
        assert!(r.sm.file(a1.span.file).path.ends_with("ins/a.ins.pas"));
        assert_eq!(r.sm.span_text(a1.span), "a1");
        // ... and the file records the %include that loaded it.
        let b1 = &r.out.tokens[2];
        let chain = r.sm.include_chain(b1.span.file);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].file, a1.span.file);
        assert_eq!(r.sm.file(chain[1].file).path, PathBuf::from("t.pas"));
        // Eof belongs to the main file.
        let eof = r.out.tokens.last().unwrap();
        assert_eq!(r.sm.file(eof.span.file).path, PathBuf::from("t.pas"));
    }

    #[test]
    fn include_errors() {
        let dir = scratch("include_errors");
        std::fs::write(dir.join("self.ins.pas"), "s %include 'self.ins.pas';").unwrap();
        std::fs::write(dir.join("p.ins.pas"), "%include 'q.ins.pas';").unwrap();
        std::fs::write(dir.join("q.ins.pas"), "%include 'p.ins.pas';").unwrap();
        std::fs::write(dir.join("open.ins.pas"), "%if _isp__m68k %then x").unwrap();
        let opts = Options {
            cwd: Some(dir.clone()),
            ..Default::default()
        };
        let d = run_opts("%include 'nope.ins.pas';", &opts).diags();
        assert_eq!(
            d,
            ["t.pas:1:1: error: include file `nope.ins.pas` not found"]
        );
        let d = run_opts("%include 'self.ins.pas';", &opts).diags();
        assert_eq!(d.len(), 1);
        assert!(
            d[0].starts_with(&format!(
                "{}:1:3: error: include cycle",
                dir.join("self.ins.pas").display()
            )),
            "{d:?}"
        );
        let d = run_opts("%include 'p.ins.pas';", &opts).diags();
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("include cycle"), "{d:?}");
        let d = run_opts("%include 'open.ins.pas'; %endif", &opts).diags();
        assert_eq!(d.len(), 2, "{d:?}");
        assert!(d[0].contains("open.ins.pas:1:1: error: unterminated conditional"));
        assert!(d[1].contains("%endif without a matching %if"));
        assert!(run_opts("%include 'x'", &opts).diags()[0].contains("`x` not found"));
        assert!(run_opts("%include '~/x';", &opts).diags()[0].contains("not supported"));
        assert!(run_opts("%include 'self.ins.pas'", &opts)
            .diags()
            .iter()
            .any(|d| d.contains("expected `;` after %include")));
        // %include in a skipped region has no effect (p. 4-55).
        let r = run_opts("%var v\n%if v %then %include 'nope'; %endif", &opts);
        assert!(r.out.diagnostics.is_empty());
    }

    #[test]
    fn lexer_errors_suppressed_in_skipped_regions() {
        assert_eq!(clean("%var v\n%if v %then @ ? %else a %endif"), ["a"]);
        assert!(errs("@")[0].contains("unexpected character"));
    }
}
