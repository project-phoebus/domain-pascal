//! Parser unit tests: one test (or more) per construct, plus whole-module
//! tests.  Expressions and types are checked through compact
//! S-expression renderings ([`sx`], [`sty`]).

use super::*;
use crate::lexer::Pragma;
use crate::preprocess::{preprocess, Options};
use crate::SourceMap;

fn parse_src(src: &str) -> Result<CompilationUnit, Vec<String>> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &Options::default());
    assert!(
        !pp.has_errors(),
        "preprocessor errors: {:?}",
        pp.diagnostics
            .iter()
            .map(|d| d.render(&sm, "dpc"))
            .collect::<Vec<_>>()
    );
    parse(&pp.tokens).map_err(|ds| ds.iter().map(|d| d.render(&sm, "dpc")).collect())
}

fn ok(src: &str) -> CompilationUnit {
    parse_src(src).unwrap_or_else(|e| panic!("unexpected errors: {e:#?}"))
}

fn errs(src: &str) -> Vec<String> {
    match parse_src(src) {
        Ok(_) => panic!("expected errors"),
        Err(e) => e,
    }
}

/// Declarations of `module m; <body>`.
fn decls(body: &str) -> Vec<Decl> {
    ok(&format!("module m;\n{body}")).decls
}

/// Statements of a procedure body.
fn stmts(body: &str) -> Vec<Stmt> {
    let d = decls(&format!("procedure p;\nbegin\n{body}\nend;\n"));
    match d.into_iter().next() {
        Some(Decl::Routine(r)) => r.body.unwrap().block.stmts,
        other => panic!("{other:?}"),
    }
}

/// The single statement of a procedure body.
fn stmt(body: &str) -> StmtKind {
    let mut s = stmts(body);
    assert_eq!(s.len(), 1, "{s:#?}");
    s.remove(0).kind
}

/// `x := <src>`, rendered.
fn ex(src: &str) -> String {
    match stmt(&format!("x := {src}")) {
        StmtKind::Assign { value, .. } => sx(&value),
        other => panic!("{other:?}"),
    }
}

/// The type of `type t = <src>;`, rendered.
fn ty(src: &str) -> String {
    match decls(&format!("type t = {src};")).remove(0) {
        Decl::Type(p) => sty(&p.defs[0].ty),
        other => panic!("{other:?}"),
    }
}

fn binop(op: BinOp) -> &'static str {
    use BinOp::*;
    match op {
        Pow => "**",
        BitAnd => "&",
        Mul => "*",
        RealDiv => "/",
        Div => "div",
        Mod => "mod",
        And => "and",
        BitOr => "!",
        Add => "+",
        Sub => "-",
        Or => "or",
        Eq => "=",
        Ne => "<>",
        Lt => "<",
        Le => "<=",
        Gt => ">",
        Ge => ">=",
        In => "in",
        AndThen => "and-then",
        OrElse => "or-else",
    }
}

fn list(v: &[Expr]) -> String {
    v.iter().map(sx).collect::<Vec<_>>().join(" ")
}

/// S-expression rendering of an expression.
fn sx(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Int(n) => n.to_string(),
        ExprKind::Real(s) => s.clone(),
        ExprKind::Str(parts) => {
            let p: Vec<String> = parts
                .iter()
                .map(|p| match p {
                    StrPart::Text(t) => format!("'{t}'"),
                    StrPart::Code(c) => format!("#{}", sx(c)),
                })
                .collect();
            p.join("")
        }
        ExprKind::Name(i) => i.name.clone(),
        ExprKind::Set(els) => {
            let v: Vec<String> = els
                .iter()
                .map(|s| match &s.hi {
                    Some(h) => format!("{}..{}", sx(&s.lo), sx(h)),
                    None => sx(&s.lo),
                })
                .collect();
            format!("[{}]", v.join(" "))
        }
        ExprKind::Unary { op, operand } => {
            let o = match op {
                UnOp::Not => "not",
                UnOp::BitNot => "~",
                UnOp::Neg => "neg",
                UnOp::Plus => "pos",
            };
            format!("({o} {})", sx(operand))
        }
        ExprKind::Binary { op, lhs, rhs } => {
            format!("({} {} {})", binop(*op), sx(lhs), sx(rhs))
        }
        ExprKind::Call { callee, args } => format!("(call {} {})", sx(callee), list(args)),
        ExprKind::Index { base, indices } => format!("(idx {} {})", sx(base), list(indices)),
        ExprKind::Field { base, field } => format!("(. {} {})", sx(base), field.name),
        ExprKind::Deref(b) => format!("(^ {})", sx(b)),
        ExprKind::Paren(b) => format!("(paren {})", sx(b)),
    }
}

fn bound(b: &Bound) -> String {
    match b {
        Bound::Expr(e) => sx(e),
        Bound::Star(_) => "*".into(),
    }
}

fn sfields(fields: &[FieldDecl], variant: &Option<Box<VariantPart>>) -> String {
    let mut v: Vec<String> = fields
        .iter()
        .map(|f| {
            let n: Vec<&str> = f.names.iter().map(|i| i.name.as_str()).collect();
            format!("{}:{}", n.join(","), sty(&f.ty))
        })
        .collect();
    if let Some(vp) = variant {
        let arms: Vec<String> = vp
            .arms
            .iter()
            .map(|a| format!("{}=>({})", list(&a.labels), sfields(&a.fields, &a.variant)))
            .collect();
        v.push(format!(
            "case {}{} {}",
            vp.tag
                .as_ref()
                .map(|t| format!("{}:", t.name))
                .unwrap_or_default(),
            vp.tag_type.name,
            arms.join(" ")
        ));
    }
    v.join("; ")
}

fn ssig(s: &RoutineSig) -> String {
    let k = match s.kind {
        RoutineKind::Procedure => "procedure",
        RoutineKind::Function => "function",
    };
    let name = s.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");
    let params = s.params.as_ref().map(|p| sparams(p)).unwrap_or_default();
    let res = s
        .result
        .as_ref()
        .map(|r| format!(":{}", sty(r)))
        .unwrap_or_default();
    format!("{k} {name}({params}){res}")
}

fn sparams(ps: &[Param]) -> String {
    ps.iter()
        .map(|p| match p {
            Param::Data {
                mode,
                names,
                univ,
                ty,
                ..
            } => {
                let m = match mode {
                    ParamMode::Value => "",
                    ParamMode::Var => "var ",
                    ParamMode::In => "in ",
                    ParamMode::Out => "out ",
                    ParamMode::InOut => "in out ",
                };
                let n: Vec<&str> = names.iter().map(|i| i.name.as_str()).collect();
                let u = if *univ { "univ " } else { "" };
                format!("{m}{}:{u}{}", n.join(","), sty(ty))
            }
            Param::Routine { sig, .. } => ssig(sig),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// S-expression rendering of a type.
fn sty(t: &TypeExpr) -> String {
    match &t.kind {
        TypeKind::Named(i) => i.name.clone(),
        TypeKind::Subrange { lo, hi } => format!("{}..{}", sx(lo), bound(hi)),
        TypeKind::Enum(ids) => {
            let v: Vec<&str> = ids.iter().map(|i| i.name.as_str()).collect();
            format!("({})", v.join(","))
        }
        TypeKind::Array {
            packed,
            indices,
            elem,
        } => {
            let ix: Vec<String> = indices.iter().map(sty).collect();
            format!(
                "{}array[{}] of {}",
                if *packed { "packed " } else { "" },
                ix.join(", "),
                sty(elem)
            )
        }
        TypeKind::Record {
            form,
            fields,
            variant,
        } => {
            let f = match form {
                RecordForm::Plain => "",
                RecordForm::Packed => "packed ",
                RecordForm::Aligned => "aligned ",
                RecordForm::Unaligned => "unaligned ",
            };
            format!("{f}record {{{}}}", sfields(fields, variant))
        }
        TypeKind::Set { packed, elem } => {
            format!(
                "{}set of {}",
                if *packed { "packed " } else { "" },
                sty(elem)
            )
        }
        TypeKind::File { packed, elem } => {
            format!(
                "{}file of {}",
                if *packed { "packed " } else { "" },
                sty(elem)
            )
        }
        TypeKind::Varying { max, elem } => format!("varying[{}] of {}", sx(max), sty(elem)),
        TypeKind::Pointer(t) => format!("^{}", sty(t)),
        TypeKind::RoutinePointer(s) => format!("^{}", ssig(s)),
        TypeKind::Attributed { attrs, ty } => {
            let a: Vec<String> = attrs
                .iter()
                .map(|a| match &a.args {
                    Some(args) => format!("{}({})", a.name.name, list(args)),
                    None => a.name.name.clone(),
                })
                .collect();
            format!("[{}] {}", a.join(", "), sty(ty))
        }
    }
}

fn sinit(i: &Init) -> String {
    match i {
        Init::Expr(e) => sx(e),
        Init::List { items, .. } => {
            let v: Vec<String> = items
                .iter()
                .map(|it| match it {
                    InitItem::Value(v) => sinit(v),
                    InitItem::Range { lo, hi, .. } => format!("{}..{}", sx(lo), sx(hi)),
                    InitItem::Repeat { count, value, .. } => format!(
                        "{} of {}",
                        count.as_ref().map(sx).unwrap_or("*".into()),
                        sinit(value)
                    ),
                    InitItem::Keyed {
                        key, key_hi, value, ..
                    } => match key_hi {
                        Some(h) => format!("{}..{} := {}", sx(key), bound(h), sinit(value)),
                        None => format!("{} := {}", sx(key), sinit(value)),
                    },
                })
                .collect();
            format!("[{}]", v.join(", "))
        }
    }
}

fn var_part(d: &Decl) -> &VarPart {
    match d {
        Decl::Var(v) => v,
        other => panic!("{other:?}"),
    }
}

fn routine(d: &Decl) -> &Routine {
    match d {
        Decl::Routine(r) => r,
        other => panic!("{other:?}"),
    }
}

// ---------------------------------------------------------------------
// Headings (ref 2.2.1 p. 2-10; 7.1.1 p. 7-2)
// ---------------------------------------------------------------------

#[test]
fn module_heading() {
    let u = ok("MODULE Proc1_Tst;");
    assert_eq!(u.kind, UnitKind::Module);
    assert_eq!(u.name.name, "proc1_tst");
    assert!(u.code_section.is_none() && u.data_section.is_none());
    assert!(u.body.is_none());
}

#[test]
fn module_heading_sections() {
    let u = ok("module m, wired_proc, wired_data;");
    assert_eq!(u.code_section.unwrap().name, "wired_proc");
    assert_eq!(u.data_section.unwrap().name, "wired_data");
    let u = ok("module m, mycode;");
    assert_eq!(u.code_section.unwrap().name, "mycode");
    assert!(u.data_section.is_none());
}

#[test]
fn module_heading_empty_code_section_rejected() {
    // Documented for programs only (p. 2-10); see TODO(ref-question).
    let e = errs("module m, , d;");
    assert!(e[0].contains("expected code section name"), "{e:?}");
}

#[test]
fn program_heading_forms() {
    let u = ok("program trapezoids (input, output, datafile);\nbegin end.");
    match &u.kind {
        UnitKind::Program { file_list } => assert_eq!(file_list.len(), 3),
        k => panic!("{k:?}"),
    }
    assert!(u.body.unwrap().stmts.len() == 1);
    let u = ok("program trapezoids, mycode, mydata; begin end.");
    assert_eq!(u.code_section.unwrap().name, "mycode");
    let u = ok("program p (input), , d; begin end.");
    assert!(u.code_section.is_none());
    assert_eq!(u.data_section.unwrap().name, "d");
}

#[test]
fn program_needs_action_part_and_final_dot() {
    let e = errs("program p; var x: integer;");
    assert!(e[0].contains("expected `begin`"), "{e:?}");
    let e = errs("program p; begin end;");
    assert!(e[0].contains("expected `.`"), "{e:?}");
    let e = errs("program p; begin end. x");
    assert!(e[0].contains("after the final `end.`"), "{e:?}");
}

#[test]
fn module_has_no_final_end_dot() {
    // Every module example ends with its last routine's `END;` (Figure 7-1,
    // p. 7-2; pp. 7-9, 7-10, 5-12).  One clear error, not a cascade.
    let e = errs("module m;\nprocedure p;\nbegin\nend;\nend.\n");
    assert_eq!(
        e,
        vec![
            "t.pas:5:1: error: a module has no final `end.`; it ends after its \
             last routine (ref 7.1, p. 7-2)"
        ]
    );
}

#[test]
fn missing_heading() {
    let e = errs("var x : integer;");
    assert_eq!(
        e,
        vec!["t.pas:1:1: error: expected `program` or `module`, found `var`"]
    );
}

// ---------------------------------------------------------------------
// Declaration parts (ref 2.2.2, pp. 2-11..2-15)
// ---------------------------------------------------------------------

#[test]
fn label_part() {
    let d = decls("label 40, reprompt, finish, 9999;");
    match &d[0] {
        Decl::Label(l) => {
            assert_eq!(l.labels.len(), 4);
            assert!(matches!(l.labels[0], Label::Int(40, _)));
            assert!(matches!(&l.labels[1], Label::Ident(i) if i.name == "reprompt"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn const_part() {
    // Examples from p. 2-12 and p. 2-13.
    let d = decls(
        "const x = 10; z = x + y; leap_offset = (current_year mod 4);\n\
         bell = chr(7); pathname_len = sizeof(pathname); ptr1 = nil;\n\
         vowels = ['a', 'e']; s1 = 'newline'(13)(10); s2 = 'a'(cr, lf)'b';\n\
         mask = 16#ff & ~2#101;",
    );
    let Decl::Const(c) = &d[0] else {
        panic!("{d:?}")
    };
    let v: Vec<String> = c
        .defs
        .iter()
        .map(|d| format!("{}={}", d.name.name, sx(&d.value)))
        .collect();
    assert_eq!(
        v,
        vec![
            "x=10",
            "z=(+ x y)",
            "leap_offset=(paren (mod current_year 4))",
            "bell=(call chr 7)",
            "pathname_len=(call sizeof pathname)",
            "ptr1=nil",
            "vowels=['a' 'e']",
            "s1='newline'#13#10",
            "s2='a'#cr#lf'b'",
            "mask=(& 255 (~ 5))",
        ]
    );
}

#[test]
fn parts_in_any_order_and_repeated() {
    // ref 2.2.2, p. 2-11: "in any order".
    let d = decls("var a: integer; const c = 1; type t = integer; var b: t; const d = 2;");
    let kinds: Vec<&str> = d
        .iter()
        .map(|d| match d {
            Decl::Var(_) => "var",
            Decl::Const(_) => "const",
            Decl::Type(_) => "type",
            _ => "?",
        })
        .collect();
    assert_eq!(kinds, vec!["var", "const", "type", "var", "const"]);
}

#[test]
fn var_part_and_allocation_clauses() {
    // ref 7.2 p. 7-3; 3.2.2 p. 3-4.
    let d = decls(
        "var x, y : integer16 := 0;\n\
         z : define integer32 := 7000000;\n\
         e : extern ^rec;\n\
         s : static integer;\n\
         static : integer;\n\
         t : static;",
    );
    let v = var_part(&d[0]);
    assert_eq!(v.decls.len(), 6);
    let names: Vec<&str> = v.decls[0].names.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec!["x", "y"]);
    assert_eq!(sinit(v.decls[0].init.as_ref().unwrap()), "0");
    let allocs: Vec<Option<AllocKind>> = v.decls.iter().map(|d| d.alloc.map(|a| a.kind)).collect();
    assert_eq!(
        allocs,
        vec![
            None,
            Some(AllocKind::Define),
            Some(AllocKind::Extern),
            Some(AllocKind::Static),
            None,
            None
        ]
    );
    assert_eq!(sty(&v.decls[2].ty), "^rec");
    // A variable called `static`, and a type called `static`.
    assert_eq!(v.decls[4].names[0].name, "static");
    assert_eq!(sty(&v.decls[5].ty), "static");
}

#[test]
fn var_overlay_sections() {
    // ref 3.14, p. 3-52.
    let d = decls("var (important) x : integer16; var ('a section') y : real;");
    match &var_part(&d[0]).section {
        Some(SectionName::Ident(i)) => assert_eq!(i.name, "important"),
        s => panic!("{s:?}"),
    }
    match &var_part(&d[1]).section {
        Some(SectionName::Str(s, _)) => assert_eq!(s, "a section"),
        s => panic!("{s:?}"),
    }
}

#[test]
fn var_initialisers() {
    // Examples from 3.10.4 (p. 3-21), 3.11.1.1-6 (pp. 3-39..3-43),
    // 3.9.2 (p. 3-15), 3.13.4 (p. 3-51).
    let d = decls(
        "var\n\
         very : messy := [c := 'X', [-1, -2, -3], rx := 123.456, 'Y', lb := 'a', hb := 'z'];\n\
         i2 : array[1..2, 1..6] of integer16 := [[1..6 := 0], [1..6 := 0]];\n\
         a : array[1..1024] of integer16 := [512 of 0, 512 OF -1];\n\
         b : array[1..400] of real := [x of 3.14, 400-x of 2.7];\n\
         q : array[1..2, 1..5] of integer16 := [[* of 0], [* of 0],];\n\
         c1 : array[1..7] of integer := [3..4 := 2, 7 := -1, 1 := 3];\n\
         ex : array[subr] of integer := [2 of -1, four..five := 47, ten..* := 814];\n\
         init6 : array[1..*] of integer16 := [1, -17, 35, 46];\n\
         letters : set of char := ['A', 'E', 'a'..'z'];\n\
         p : ptr_to_int16 := nil;\n\
         s1 : array[1..40] of char := 'Quoted strings are ok';",
    );
    let v = var_part(&d[0]);
    let got: Vec<String> = v
        .decls
        .iter()
        .map(|d| sinit(d.init.as_ref().unwrap()))
        .collect();
    assert_eq!(
        got,
        vec![
            "[c := 'X', [(neg 1), (neg 2), (neg 3)], rx := 123.456, 'Y', lb := 'a', hb := 'z']",
            "[[1..6 := 0], [1..6 := 0]]",
            "[512 of 0, 512 of (neg 1)]",
            "[x of 3.14, (- 400 x) of 2.7]",
            "[[* of 0], [* of 0]]",
            "[3..4 := 2, 7 := (neg 1), 1 := 3]",
            "[2 of (neg 1), four..five := 47, ten..* := 814]",
            "[1, (neg 17), 35, 46]",
            "['A', 'E', 'a'..'z']",
            "nil",
            "'Quoted strings are ok'",
        ]
    );
    assert_eq!(sty(&v.decls[7].ty), "array[1..*] of integer16");
}

#[test]
fn init_star_range_needs_value() {
    let e = errs("module m; var a : array[1..3] of integer := [1..*];");
    assert!(e[0].contains("must be followed by `:=`"), "{e:?}");
}

#[test]
fn define_part() {
    // ref 7.2.2 p. 7-5; 7.2.3 p. 7-6; 7.3.4 p. 7-11.
    let d = decls("var x : extern integer16;\ndefine x := 8; table1 := 'abc'; exponent;");
    let Decl::Define(p) = &d[1] else {
        panic!("{d:?}")
    };
    let v: Vec<String> = p
        .items
        .iter()
        .map(|i| match &i.init {
            Some(init) => format!("{}:={}", i.name.name, sinit(init)),
            None => i.name.name.clone(),
        })
        .collect();
    assert_eq!(v, vec!["x:=8", "table1:='abc'", "exponent"]);
}

#[test]
fn define_part_not_in_routine() {
    // ref 2.2.3.2, p. 2-16.
    let e = errs("module m; procedure p; define x; begin end;");
    assert!(e[0].contains("not allowed inside a routine"), "{e:?}");
}

#[test]
fn define_as_variable_name() {
    // `define` is a predeclared identifier (Table A-2), not reserved.
    let d = decls("var define : integer;");
    assert_eq!(var_part(&d[0]).decls[0].names[0].name, "define");
}

#[test]
fn attribute_part() {
    // ref 3.16, p. 3-78.
    let d = decls(
        "attribute integer_attributes = long, natural;\n\
         peb_page_attributes = address(16#ff7000), keyboard_attributes;\n\
         type int1 = [integer_attributes] integer32;",
    );
    let Decl::Attribute(a) = &d[0] else {
        panic!("{d:?}")
    };
    assert_eq!(a.defs.len(), 2);
    assert_eq!(a.defs[1].attrs[0].name.name, "address");
    assert_eq!(
        sx(&a.defs[1].attrs[0].args.as_ref().unwrap()[0]),
        "16740352"
    );
    assert!(matches!(&d[1], Decl::Type(_)));
}

#[test]
fn routine_option_part() {
    // ref 5.6.2 p. 5-22; 5.6.4 p. 5-22.
    let d = decls(
        "routine_option my_c_routine_options = a0_return, d0_return, c_param;\n\
         default_routine_options = val_param;\n\
         procedure calling_a_c_module; options(my_c_routine_options); extern;",
    );
    let Decl::RoutineOption(r) = &d[0] else {
        panic!("{d:?}")
    };
    assert_eq!(r.defs[0].options.len(), 3);
    assert_eq!(r.defs[1].name.name, "default_routine_options");
    let rt = routine(&d[1]);
    let o: Vec<&str> = rt.options.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(o, vec!["my_c_routine_options", "extern"]);
}

// ---------------------------------------------------------------------
// Types (chapter 3)
// ---------------------------------------------------------------------

#[test]
fn simple_types() {
    assert_eq!(ty("integer32"), "integer32");
    assert_eq!(ty("13..19"), "13..19");
    assert_eq!(ty("-5..5"), "(neg 5)..5");
    assert_eq!(ty("'A'..'Z'"), "'A'..'Z'");
    assert_eq!(ty("wachusett..washington"), "wachusett..washington");
    assert_eq!(ty("0..n-1"), "0..(- n 1)");
    assert_eq!(ty("(lemon, lime, orange)"), "(lemon,lime,orange)");
}

#[test]
fn set_and_file_types() {
    assert_eq!(ty("set of 0..31"), "set of 0..31");
    assert_eq!(ty("packed set of char"), "packed set of char");
    assert_eq!(ty("file of integer"), "file of integer");
}

#[test]
fn array_types() {
    assert_eq!(
        ty("array[1..100] of integer16"),
        "array[1..100] of integer16"
    );
    assert_eq!(
        ty("array[1..4, 1..2] of boolean"),
        "array[1..4, 1..2] of boolean"
    );
    assert_eq!(ty("array[h..be] of real"), "array[h..be] of real");
    assert_eq!(ty("array[colors] of real"), "array[colors] of real");
    assert_eq!(
        ty("packed array[1..25] of char"),
        "packed array[1..25] of char"
    );
    assert_eq!(
        ty("array[1..5] of varying[20] of char"),
        "array[1..5] of varying[20] of char"
    );
}

#[test]
fn star_bound_only_in_array_index() {
    let e = errs("module m; type t = 1..*;");
    assert!(e[0].contains("expected expression, found `*`"), "{e:?}");
}

#[test]
fn record_types() {
    assert_eq!(
        ty("record name : array[1..30] of char; id : integer16; end"),
        "record {name:array[1..30] of char; id:integer16}"
    );
    // No trailing semicolon before `end`, and an empty record.
    assert_eq!(ty("record a, b : integer end"), "record {a,b:integer}");
    assert_eq!(ty("record end"), "record {}");
    assert_eq!(
        ty("packed record ages : 10..20; graduating : boolean; end"),
        "packed record {ages:10..20; graduating:boolean}"
    );
    assert_eq!(
        ty("aligned record a : integer; end"),
        "aligned record {a:integer}"
    );
    assert_eq!(
        ty("UNALIGNED RECORD a : integer; END"),
        "unaligned record {a:integer}"
    );
}

#[test]
fn variant_records() {
    // ref 3.10.2, pp. 3-19..3-20.
    assert_eq!(
        ty("record employee : array[1..30] of char;\n\
            case wo : worker_groups of\n\
              exempt : (yearly_salary : integer32);\n\
              non_exempt : (hourly_wage : real);\n\
            end"),
        "record {employee:array[1..30] of char; case wo:worker_groups \
         exempt=>(yearly_salary:integer32) non_exempt=>(hourly_wage:real)}"
    );
    assert_eq!(
        ty("record case integer of\n\
              1: (all : array[1..4] of char);\n\
              2, 3: (x1 : integer16; x2 : boolean; x3 : char);\n\
              4: (case boolean of true: (r : single); false: ())\n\
            end"),
        "record {case integer 1=>(all:array[1..4] of char) \
         2 3=>(x1:integer16; x2:boolean; x3:char) \
         4=>(case boolean true=>(r:single) false=>())}"
    );
}

#[test]
fn pointer_types() {
    assert_eq!(ty("^integer16"), "^integer16");
    assert_eq!(ty("^student"), "^student");
    // ref 3.13.3, p. 3-50.
    assert_eq!(
        ty("^procedure (a, b, c : integer)"),
        "^procedure (a,b,c:integer)"
    );
    assert_eq!(
        ty("^function (x, y : real) : real"),
        "^function (x,y:real):real"
    );
    assert_eq!(ty("^procedure"), "^procedure ()");
}

#[test]
fn varying_type() {
    // ref 3.11.2, p. 3-44.
    assert_eq!(ty("varying[10] of char"), "varying[10] of char");
    // `varying` without `[` is an ordinary type name.
    assert_eq!(ty("varying"), "varying");
}

#[test]
fn attributed_types() {
    // ref 3.15, pp. 3-53..3-63.
    assert_eq!(
        ty("[volatile] array[1..10] of integer"),
        "[volatile] array[1..10] of integer"
    );
    assert_eq!(ty("[aligned(1)] integer32"), "[aligned(1)] integer32");
    let d = decls(
        "var peb_page : [address(16#ff7000), device(read)] char; x : [atomic, device] integer;",
    );
    let v = var_part(&d[0]);
    assert_eq!(
        sty(&v.decls[0].ty),
        "[address(16740352), device(read)] char"
    );
    assert_eq!(sty(&v.decls[1].ty), "[atomic, device] integer");
}

// ---------------------------------------------------------------------
// Routines (chapter 5)
// ---------------------------------------------------------------------

#[test]
fn procedure_and_function_headings() {
    let d = decls(
        "procedure simple; begin end;\n\
         procedure con(a : integer; b : real); begin end;\n\
         function anger(x, y : boolean) : integer16; begin end;",
    );
    let r = routine(&d[0]);
    assert_eq!(r.kind, RoutineKind::Procedure);
    assert!(r.params.is_none() && r.result.is_none());
    let r = routine(&d[1]);
    assert_eq!(sparams(r.params.as_ref().unwrap()), "a:integer; b:real");
    let r = routine(&d[2]);
    assert_eq!(r.kind, RoutineKind::Function);
    assert_eq!(sty(r.result.as_ref().unwrap()), "integer16");
}

#[test]
fn parameter_modes_and_univ() {
    // ref 5.3 p. 5-5; 5.3.2 p. 5-7; 5.3.3.
    let d = decls(
        "procedure p(var a : integer; in b : integer; out c : integer;\n\
                     in out d, e : integer; in b2 : univ big_array; f : univ;\n\
                     out : integer; in out : integer);\n\
         begin end;",
    );
    assert_eq!(
        sparams(routine(&d[0]).params.as_ref().unwrap()),
        "var a:integer; in b:integer; out c:integer; in out d,e:integer; \
         in b2:univ big_array; f:univ; out:integer; in out:integer"
    );
}

#[test]
fn routine_parameters() {
    // ref 5.4, p. 5-13.
    let d = decls(
        "procedure caller(procedure callee(a, b : integer)); begin end;\n\
         procedure c2(function callee : integer); begin end;\n\
         procedure c3(function a(procedure b) : integer); begin end;",
    );
    assert_eq!(
        sparams(routine(&d[0]).params.as_ref().unwrap()),
        "procedure callee(a,b:integer)"
    );
    assert_eq!(
        sparams(routine(&d[1]).params.as_ref().unwrap()),
        "function callee():integer"
    );
    assert_eq!(
        sparams(routine(&d[2]).params.as_ref().unwrap()),
        "function a(procedure b()):integer"
    );
}

#[test]
fn routine_options_long_and_short() {
    // ref 5.5.1, p. 5-15.
    let d = decls(
        "function eggs_and_ham(letter : char) : char; internal; begin end;\n\
         procedure sam_i_am(x, y : real); options(extern, abnormal);\n\
         procedure sam2(x, y : real); extern; val_param;\n\
         function f(lock_id : integer) : boolean; options(val_param, nosave); begin end;\n\
         procedure g; forward;\n\
         procedure g; begin end;",
    );
    let opts = |r: &Routine| -> Vec<String> { r.options.iter().map(|i| i.name.clone()).collect() };
    assert_eq!(opts(routine(&d[0])), vec!["internal"]);
    assert!(routine(&d[0]).body.is_some());
    assert_eq!(opts(routine(&d[1])), vec!["extern", "abnormal"]);
    assert!(routine(&d[1]).body.is_none());
    assert_eq!(opts(routine(&d[2])), vec!["extern", "val_param"]);
    assert!(routine(&d[2]).body.is_none());
    assert_eq!(opts(routine(&d[3])), vec!["val_param", "nosave"]);
    assert!(routine(&d[4]).body.is_none());
    assert!(routine(&d[5]).body.is_some());
}

#[test]
fn short_form_only_for_four_options() {
    // `abnormal;` is not a short-form option (p. 5-15), so it starts the
    // body: it is not a declaration and not `begin`.
    let e = errs("module m; procedure p; abnormal; begin end;");
    assert!(e[0].contains("expected `begin`"), "{e:?}");
}

#[test]
fn body_with_name_only_after_extern() {
    // ref 7.3.4, p. 7-11.
    let d = decls(
        "define exponent;\n\
         function exponent(number, power : integer16) : real; extern;\n\
         function exponent; begin exponent := 1 end;",
    );
    let r = routine(&d[2]);
    assert!(r.params.is_none() && r.result.is_none());
    assert!(r.body.is_some());
}

#[test]
fn section_attribute() {
    // ref 5.7.1, p. 5-25.
    let d = decls(
        "[section(npc, npd)] function foobar1 : real; begin end;\n\
         [SECTION(, error_d)] procedure xxx; begin end;",
    );
    match &routine(&d[0]).attrs[0] {
        RoutineAttr::Section { code, data, .. } => {
            assert_eq!(code.as_ref().unwrap().name, "npc");
            assert_eq!(data.as_ref().unwrap().name, "npd");
        }
    }
    match &routine(&d[1]).attrs[0] {
        RoutineAttr::Section { code, data, .. } => {
            assert!(code.is_none());
            assert_eq!(data.as_ref().unwrap().name, "error_d");
        }
    }
    let e = errs("module m; [inline] procedure p; begin end;");
    assert!(e[0].contains("routine attribute `section`"), "{e:?}");
}

#[test]
fn nested_routines() {
    // ref 2.2.3.3 p. 2-17; 2.4.
    let d = decls(
        "procedure outer(a : integer);\n\
         var l : integer;\n\
           procedure inner; begin l := a end;\n\
           function inner2 : integer; begin inner2 := 0 end;\n\
         begin inner end;",
    );
    let body = routine(&d[0]).body.as_ref().unwrap();
    assert_eq!(body.decls.len(), 3);
    assert_eq!(routine(&body.decls[1]).name.name, "inner");
    assert_eq!(routine(&body.decls[2]).name.name, "inner2");
}

#[test]
fn procedure_with_result_type_is_an_error() {
    let e = errs("module m; procedure p : integer; begin end;");
    assert!(e[0].contains("a procedure has no result type"), "{e:?}");
}

// ---------------------------------------------------------------------
// Statements (chapter 4)
// ---------------------------------------------------------------------

#[test]
fn assignment_and_calls() {
    let s = stmts("x := 5; writeln('hi'); simple; p^(y); a.b[1]^.c := f(x, y); disable");
    let k: Vec<String> = s
        .iter()
        .map(|s| match &s.kind {
            StmtKind::Assign { target, value } => format!("{} := {}", sx(target), sx(value)),
            StmtKind::Call(e) => format!("call {}", sx(e)),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        k,
        vec![
            "x := 5",
            "call (call writeln 'hi')",
            "call simple",
            "call (call (^ p) y)",
            "(. (^ (idx (. a b) 1)) c) := (call f x y)",
            "call disable",
        ]
    );
}

#[test]
fn compound_and_empty_statements() {
    // ref p. 4-188: `;;` is an empty statement.
    let s = stmts("begin x := 1; ; end;");
    assert_eq!(s.len(), 2);
    match &s[0].kind {
        StmtKind::Compound(b) => {
            assert_eq!(b.stmts.len(), 3);
            assert!(matches!(b.stmts[1].kind, StmtKind::Empty));
            assert!(matches!(b.stmts[2].kind, StmtKind::Empty));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn if_statement_dangling_else() {
    match stmt("if a then if b then x := 1 else x := 2") {
        StmtKind::If {
            else_branch: None,
            then_branch,
            ..
        } => assert!(matches!(
            then_branch.kind,
            StmtKind::If {
                else_branch: Some(_),
                ..
            }
        )),
        other => panic!("{other:?}"),
    }
}

#[test]
fn case_statement() {
    // ref pp. 4-36..4-38.
    match stmt(
        "case a_letter of\n\
           'y', 'Y' : sale := true;\n\
           'n', 'N' : begin sale := false end;\n\
           OTHERWISE writeln('x'); writeln('y'); return;\n\
         end",
    ) {
        StmtKind::Case {
            expr,
            arms,
            otherwise,
        } => {
            assert_eq!(sx(&expr), "a_letter");
            assert_eq!(arms.len(), 2);
            assert_eq!(list(&arms[0].labels), "'y' 'Y'");
            let o = otherwise.unwrap();
            assert!(matches!(o[2].kind, StmtKind::Return));
            assert!(matches!(o[3].kind, StmtKind::Empty));
        }
        other => panic!("{other:?}"),
    }
    // No `;` before `end`, no otherwise.
    match stmt("case n of 1: x := 1; 2: x := 2 end") {
        StmtKind::Case {
            arms, otherwise, ..
        } => {
            assert_eq!(arms.len(), 2);
            assert!(otherwise.is_none());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn case_label_range_rejected() {
    let e = errs("module m; procedure p; begin case n of 1..3: x := 1 end end;");
    assert!(
        e[0].contains("ranges are not allowed in case labels"),
        "{e:?}"
    );
}

#[test]
fn loops() {
    match stmt("while i < 10 do update") {
        StmtKind::While { cond, body } => {
            assert_eq!(sx(&cond), "(< i 10)");
            assert!(matches!(body.kind, StmtKind::Call(_)));
        }
        other => panic!("{other:?}"),
    }
    match stmt("repeat x := x + 1; next until x > 5") {
        StmtKind::Repeat { body, cond } => {
            assert_eq!(body.len(), 2);
            assert!(matches!(body[1].kind, StmtKind::Next));
            assert_eq!(sx(&cond), "(> x 5)");
        }
        other => panic!("{other:?}"),
    }
    match stmt("for i := n downto 1 do exit") {
        StmtKind::For { var, dir, body, .. } => {
            assert_eq!(var.name, "i");
            assert_eq!(dir, ForDir::Downto);
            assert!(matches!(body.kind, StmtKind::Exit));
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        stmt("for ice_cream := mint to beige do writeln(ice_cream)"),
        StmtKind::For {
            dir: ForDir::To,
            ..
        }
    ));
}

#[test]
fn with_statement() {
    // ref pp. 4-211..4-212.
    match stmt("with our_writers, their_name do first := 'x'") {
        StmtKind::With { items, .. } => {
            assert_eq!(items.len(), 2);
            assert!(items.iter().all(|i| i.alias.is_none()));
        }
        other => panic!("{other:?}"),
    }
    match stmt("WITH basketball_team : b, p^.x : c DO b.height := 1") {
        StmtKind::With { items, .. } => {
            assert_eq!(items[0].alias.as_ref().unwrap().name, "b");
            assert_eq!(sx(&items[1].record), "(. (^ p) x)");
            assert_eq!(items[1].alias.as_ref().unwrap().name, "c");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn goto_and_labels() {
    let s = stmts("goto 40; goto finish; 40: x := 1; finish:");
    assert!(matches!(s[0].kind, StmtKind::Goto(Label::Int(40, _))));
    assert!(matches!(&s[1].kind, StmtKind::Goto(Label::Ident(i)) if i.name == "finish"));
    assert!(matches!(
        &s[2].kind,
        StmtKind::Labeled {
            label: Label::Int(40, _),
            ..
        }
    ));
    match &s[3].kind {
        StmtKind::Labeled { stmt, .. } => assert!(matches!(stmt.kind, StmtKind::Empty)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn exit_next_return() {
    let s = stmts("exit; next; return; if a then return else exit");
    assert!(matches!(s[0].kind, StmtKind::Exit));
    assert!(matches!(s[1].kind, StmtKind::Next));
    assert!(matches!(s[2].kind, StmtKind::Return));
    // As ordinary names when followed by something else.
    // Before the `otherwise` of a case whose last arm has no `;`.
    match stmt("case x of 1: return otherwise exit end") {
        StmtKind::Case {
            arms, otherwise, ..
        } => {
            assert!(matches!(arms[0].stmt.kind, StmtKind::Return));
            assert!(matches!(otherwise.unwrap()[0].kind, StmtKind::Exit));
        }
        other => panic!("{other:?}"),
    }
    let s = stmts("next := 1; return(x)");
    assert!(matches!(&s[0].kind, StmtKind::Assign { .. }));
    assert!(matches!(&s[1].kind, StmtKind::Call(_)));
}

// ---------------------------------------------------------------------
// Expressions (Table 4-3, p. 4-4)
// ---------------------------------------------------------------------

#[test]
fn precedence_levels() {
    assert_eq!(ex("a + b * c"), "(+ a (* b c))");
    assert_eq!(ex("a * b + c"), "(+ (* a b) c)");
    assert_eq!(ex("a - b - c"), "(- (- a b) c)");
    assert_eq!(ex("a & b ! c"), "(! (& a b) c)");
    assert_eq!(ex("a ! b & c"), "(! a (& b c))");
    assert_eq!(ex("a div b mod c / d"), "(/ (mod (div a b) c) d)");
    assert_eq!(ex("a = b + c"), "(= a (+ b c))");
    assert_eq!(ex("a < b and then c in s"), "(and-then (< a b) (in c s))");
    assert_eq!(ex("a or else b and then c"), "(or-else a (and-then b c))");
    assert_eq!(ex("a and b or c"), "(or (and a b) c)");
    assert_eq!(ex("x = 1 or else y"), "(or-else (= x 1) y)");
}

#[test]
fn power_and_not() {
    // `not` binds tighter than `**`; `**` is right-associative (Table 4-2).
    assert_eq!(ex("x ** y ** z"), "(** x (** y z))");
    assert_eq!(ex("not a ** b"), "(** (not a) b)");
    assert_eq!(ex("2 * x ** 3"), "(* 2 (** x 3))");
    assert_eq!(ex("not a and b"), "(and (not a) b)");
    assert_eq!(ex("~x & 16#ff"), "(& (~ x) 255)");
}

#[test]
fn signs() {
    assert_eq!(ex("-a * b"), "(neg (* a b))");
    assert_eq!(ex("-a + b"), "(+ (neg a) b)");
    assert_eq!(ex("+1"), "(pos 1)");
    assert_eq!(ex("a < -1"), "(< a (neg 1))");
    // A sign only starts a simple expression: "You cannot follow a binary
    // operator with a unary operator of lower precedence" (p. 4-82, whose
    // examples `9 DIV -3` and `5 * -100` are improper).
    let e = errs("module m; procedure p; begin x := a * -b end;");
    assert!(e[0].contains("expected expression, found `-`"), "{e:?}");
    let e = errs("module m; procedure p; begin x := 9 div -3 end;");
    assert!(e[0].contains("expected expression, found `-`"), "{e:?}");
    assert_eq!(ex("9 div (-3)"), "(div 9 (paren (neg 3)))");
    // Undecided: `a + -b` (same level; see TODO(ref-question)).  Rejected.
    let e = errs("module m; procedure p; begin x := a + -b end;");
    assert!(e[0].contains("expected expression, found `-`"), "{e:?}");
}

#[test]
fn sets_and_selectors() {
    assert_eq!(ex("[]"), "[]");
    assert_eq!(ex("[a, b..c, 'x']"), "[a b..c 'x']");
    assert_eq!(ex("[a] + [b]"), "(+ [a] [b])");
    assert_eq!(
        ex("lock_id in proc1_$current_pcb^.resource_locks_held"),
        "(in lock_id (. (^ proc1_$current_pcb) resource_locks_held))"
    );
    assert_eq!(ex("a[i, j][k]"), "(idx (idx a i j) k)");
    assert_eq!(ex("(a + b) * c"), "(* (paren (+ a b)) c)");
    assert_eq!(ex("integer32(x)"), "(call integer32 x)");
    assert_eq!(ex("addr(f)^"), "(^ (call addr f))");
    assert_eq!(ex("3.14"), "3.14");
}

#[test]
fn relations_do_not_chain() {
    let e = errs("module m; procedure p; begin x := a = b = c end;");
    assert!(e[0].contains("expected `;`, found `=`"), "{e:?}");
}

// ---------------------------------------------------------------------
// Directives (ref 4.7, pp. 4-52..4-58)
// ---------------------------------------------------------------------

#[test]
fn pragmas_between_declarations() {
    let u = ok("%natural_alignment;\n\
         module m;\n\
         %push_alignment;\n\
         type a = integer;\n\
         %pop_alignment;\n\
         %begin_inline;\n\
         procedure p; begin %word_alignment; end;\n\
         %end_inline;");
    assert_eq!(u.leading_pragmas.len(), 1);
    assert_eq!(u.leading_pragmas[0].pragma, Pragma::NaturalAlignment);
    let k: Vec<String> = u
        .decls
        .iter()
        .map(|d| match d {
            Decl::Pragma(m) => format!("{:?}", m.pragma),
            Decl::Type(_) => "type".into(),
            Decl::Routine(_) => "routine".into(),
            other => format!("{other:?}"),
        })
        .collect();
    // A directive inside a statement part takes effect after the routine.
    assert_eq!(
        k,
        vec![
            "PushAlignment",
            "type",
            "PopAlignment",
            "BeginInline",
            "routine",
            "WordAlignment",
            "EndInline"
        ]
    );
}

#[test]
fn pragma_splits_a_part() {
    let d = decls("type a = ^b;\n%natural_alignment;\nb = record x : integer32 end;");
    match (&d[0], &d[1], &d[2]) {
        (Decl::Type(t1), Decl::Pragma(_), Decl::Type(t2)) => {
            assert!(!t1.continues_previous);
            assert!(t2.continues_previous);
        }
        other => panic!("{other:?}"),
    }
    // A var part keeps its section across the split.
    let d = decls("var (s) x : integer;\n%natural_alignment;\ny : integer32;");
    let v2 = var_part(&d[2]);
    assert!(v2.continues_previous);
    assert!(matches!(&v2.section, Some(SectionName::Ident(i)) if i.name == "s"));
}

#[test]
fn pragma_inside_declaration_rejected() {
    let e = errs("module m; type r = record %natural_alignment; x : integer32 end;");
    assert!(
        e[0].contains("t.pas:1:27: error: this directive is not supported inside a declaration"),
        "{e:?}"
    );
}

#[test]
fn pragmas_in_main_program_body() {
    let u = ok("program p; begin %natural_alignment; x := 1 end.");
    assert_eq!(u.trailing_pragmas.len(), 1);
}

// ---------------------------------------------------------------------
// Diagnostics and recovery
// ---------------------------------------------------------------------

#[test]
fn errors_carry_positions() {
    let e = errs("module m;\nvar x integer;\n");
    assert_eq!(
        e,
        vec!["t.pas:2:7: error: expected `:`, found identifier `integer`"]
    );
}

#[test]
fn recovery_reports_several_errors() {
    let e = errs(
        "module m;\n\
         var x integer;\n\
         y : integer;\n\
         const c 1;\n\
         procedure p;\n\
         begin\n\
           x := ;\n\
           y := 2 3;\n\
           if then z := 1;\n\
         end;\n\
         procedure q(a : ); begin end;\n\
         function f : integer; begin f := 1 end;\n",
    );
    assert_eq!(
        e,
        vec![
            "t.pas:2:7: error: expected `:`, found identifier `integer`",
            "t.pas:4:9: error: expected `=`, found integer",
            "t.pas:7:6: error: expected expression, found `;`",
            "t.pas:8:8: error: expected `;`, found integer",
            "t.pas:9:4: error: expected expression, found `then`",
            "t.pas:11:17: error: expected expression, found `)`",
        ]
    );
}

#[test]
fn missing_end() {
    let e = errs("module m; procedure p; begin x := 1;");
    assert_eq!(
        e,
        vec!["t.pas:1:37: error: expected `end`, found end of file"]
    );
}

// ---------------------------------------------------------------------
// Whole modules
// ---------------------------------------------------------------------

/// A reconstruction of PROC1_$TST_LOCK (0xE148CA); the first conformance
/// target (CLAUDE.md section 6.3).
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
fn whole_module_tst_lock() {
    let u = ok(TST_LOCK);
    assert_eq!(u.name.name, "proc1_tst");
    assert_eq!(u.decls.len(), 3);
    match &u.decls[0] {
        Decl::Type(t) => {
            assert_eq!(t.defs[0].name.name, "proc1_$pcb_t");
            assert_eq!(
                sty(&t.defs[0].ty),
                "record {pad:array[1..16] of integer32; resource_locks_held:set of 0..31}"
            );
        }
        other => panic!("{other:?}"),
    }
    let v = var_part(&u.decls[1]);
    assert_eq!(v.decls[0].alloc.unwrap().kind, AllocKind::Extern);
    assert_eq!(sty(&v.decls[0].ty), "^proc1_$pcb_t");
    let r = routine(&u.decls[2]);
    assert_eq!(r.name.name, "proc1_$tst_lock");
    assert_eq!(sparams(r.params.as_ref().unwrap()), "lock_id:integer");
    assert_eq!(sty(r.result.as_ref().unwrap()), "boolean");
    assert_eq!(r.options[0].name, "val_param");
    let body = &r.body.as_ref().unwrap().block.stmts;
    match &body[0].kind {
        StmtKind::Assign { target, value } => {
            assert_eq!(sx(target), "proc1_$tst_lock");
            assert_eq!(
                sx(value),
                "(in lock_id (. (^ proc1_$current_pcb) resource_locks_held))"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn whole_module_mixed() {
    let u = ok("{ A module touching most constructs. }
module kitchen_sink, wired_proc, wired_data;

const
  max_q = 16#20;
  msg = 'done'(13, 10);

type
  q_kind_t = (k_a, k_b);
  q_elem_t = record
    next: ^q_elem_t;
    key: integer32;
    case kind: q_kind_t of
      k_a: (a: integer);
      k_b: (b: packed array[0..3] of char);
  end;
  handler_t = ^procedure (in elem: q_elem_t; out status: status_$t);
  status_$t = integer32;

routine_option
  default_routine_options = val_param;

var (wired_data)
  queue: array[1..max_q] of q_elem_t;
  count: static integer := 0;
  flags: set of 0..15 := [0, 3..5];
  hook: extern handler_t;

define
  count;

procedure helper(var e: q_elem_t); internal;
var i: integer;
begin
  for i := 1 to max_q do begin
    if queue[i].key = 0 then next;
    with queue[i] : q do
      case q.kind of
        k_a: q.a := lshft(q.a, 1) ! 1;
        k_b: q.b[0] := 'x';
      end;
    if i >= max_q - 1 then exit;
  end;
end;

[section(wired_proc, wired_data)]
function q_$count(in which: integer): integer32;
  function inner: integer32;
  begin
    inner := count * 2;
  end;
begin
  disable;
  q_$count := inner;
  enable;
  if (which < 0) or else not (which in [1..max_q]) then
    return;
  repeat
    count := count - 1
  until count <= 0;
end;
");
    assert_eq!(u.decls.len(), 7);
    assert!(matches!(u.decls[3], Decl::Var(_)));
    let r = routine(&u.decls[6]);
    assert_eq!(r.attrs.len(), 1);
    assert_eq!(r.body.as_ref().unwrap().decls.len(), 1);
}

// ---------------------------------------------------------------------
// Robustness: deep nesting is a diagnostic, not a stack overflow
// ---------------------------------------------------------------------

/// Parse on a thread with the default 2 MB test-thread stack, as `cargo
/// test` gives each test.
fn parse_on_small_stack(src: String) -> Result<CompilationUnit, Vec<String>> {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || parse_src(&src))
        .unwrap()
        .join()
        .unwrap()
}

#[test]
fn deep_nesting_is_an_error_not_a_crash() {
    let n = 5_000;
    let cases = [
        format!("module m; const c = {}1{};", "(".repeat(n), ")".repeat(n)),
        format!("module m; const c = {}1;", "not ".repeat(n)),
        format!("module m; const c = {}1;", "a ** ".repeat(n)),
        format!("module m; const c = {}1;", "a[".repeat(n)),
        format!("module m; type t = {}integer;", "^".repeat(n)),
        format!("module m; var x : t := {};", "[".repeat(n)),
        format!(
            "module m; type t = record case a of 1: {};",
            "(case a of 1: ".repeat(n)
        ),
        format!("module m; procedure p; begin {}end;", "begin ".repeat(n)),
        format!(
            "module m; procedure p; begin {}x end;",
            "if a then ".repeat(n)
        ),
        format!("module m; procedure p; begin {}x end;", "l: ".repeat(n)),
        format!("module m; {}", "procedure p; ".repeat(n)),
        format!("module m; procedure p({};", "procedure q(".repeat(n)),
    ];
    for src in cases {
        let e = parse_on_small_stack(src).expect_err("must not parse");
        assert!(e[0].contains("nested too deeply"), "{:?}", &e[..1]);
    }
}

#[test]
fn reasonable_nesting_still_parses() {
    // Well past anything in real code, below the limit.
    let n = 60;
    let src = format!(
        "module m;\nprocedure p;\nbegin\n{}x := {}1{}{}\nend;\n",
        "begin ".repeat(n),
        "(".repeat(n),
        ")".repeat(n),
        " end".repeat(n),
    );
    parse_on_small_stack(src).unwrap();
}
