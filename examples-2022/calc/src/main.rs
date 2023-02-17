use ir::SourceProgram;

// ANCHOR: jar_struct
#[salsa::jar(db = Db)]
pub struct Jar(
    crate::compile::compile,
    crate::ir::SourceProgram,
    crate::ir::Program,
    crate::ir::VariableId,
    crate::ir::FunctionId,
    crate::ir::Function,
    crate::ir::Diagnostics,
    crate::parser::parse_source_program,
    crate::type_check::type_check_program,
    crate::type_check::type_check_function,
    crate::evaluate::evaluate_function,
    crate::ir::find_function,
);
// ANCHOR_END: jar_struct

// ANCHOR: jar_db
pub trait Db: salsa::DbWithJar<Jar> + PushLog {}
// ANCHOR_END: jar_db

// ANCHOR: jar_db_impl
impl<DB> Db for DB where DB: ?Sized + PushLog + salsa::DbWithJar<Jar> {}
// ANCHOR_END: jar_db_impl

// ANCHOR: PushLog
pub trait PushLog {
    /// When testing, invokes `message` to create a log string and
    /// pushes that string onto an internal list of logs.
    ///
    /// This list of logs can later be used to observe what got re-executed
    /// or modified during execution.
    fn push_log(&self, message: &mut dyn FnMut() -> String);
}
// ANCHOR_END: PushLog

use std::sync::atomic::{AtomicUsize, Ordering};

pub struct WorkCounts {
    evaluate_function_steps: AtomicUsize,
    compile_steps: AtomicUsize,
    parse_steps: AtomicUsize,
    resolve_steps: AtomicUsize,
    type_check_program_steps: AtomicUsize,
    type_check_function_steps: AtomicUsize,
}

#[derive(Debug)]
pub struct WorkSnapshot {
    compile_steps: usize,
    parse_steps: usize,
    resolve_steps: usize,
    type_check_program_steps: usize,
    type_check_function_steps: usize,
    evaluate_function_steps: usize,
}

impl WorkSnapshot {
    fn measure() -> Self {
        let w = &WORK_COUNTS;
        Self {
            evaluate_function_steps: w.evaluate_function_steps.load(R),
            compile_steps: w.compile_steps.load(R),
            parse_steps: w.parse_steps.load(R),
            resolve_steps: w.resolve_steps.load(R),
            type_check_program_steps: w.type_check_program_steps.load(R),
            type_check_function_steps: w.type_check_function_steps.load(R),
        }
    }
    fn delta(&self, newer: Self) -> Self {
        macro_rules! sub {
            ($f:ident) => {
                newer.$f.checked_sub(self.$f).unwrap()
            };
        }
        WorkSnapshot {
            evaluate_function_steps: sub!(evaluate_function_steps),
            compile_steps: sub!(compile_steps),
            parse_steps: sub!(parse_steps),
            resolve_steps: sub!(resolve_steps),
            type_check_program_steps: sub!(type_check_program_steps),
            type_check_function_steps: sub!(type_check_function_steps),
        }
    }
}

impl WorkCounts {
    pub const fn new() -> Self {
        Self {
            evaluate_function_steps: AtomicUsize::new(0),
            compile_steps: AtomicUsize::new(0),
            parse_steps: AtomicUsize::new(0),
            resolve_steps: AtomicUsize::new(0),
            type_check_program_steps: AtomicUsize::new(0),
            type_check_function_steps: AtomicUsize::new(0),
        }
    }
    fn evaluate_step(&self) {
        self.evaluate_function_steps.fetch_add(1, R);
    }
    fn compile_step(&self) {
        self.compile_steps.fetch_add(1, R);
    }
    fn parse_step(&self) {
        self.parse_steps.fetch_add(1, R);
    }
    fn resolve_step(&self) {
        self.resolve_steps.fetch_add(1, R);
    }
    fn type_check_program_step(&self) {
        self.type_check_program_steps.fetch_add(1, R);
    }
    fn type_check_function_step(&self) {
        self.type_check_function_steps.fetch_add(1, R);
    }
}
pub static WORK_COUNTS: WorkCounts = WorkCounts::new();
const R: Ordering = Ordering::Relaxed;

mod compile;
mod db;
mod evaluate;
mod ir;
mod parser;
mod type_check;

pub fn main() {
    let mut db = db::Database::default();
    let source_texts = SOURCE_TEXT_SEQ;
    let source_program = SourceProgram::new(&mut db, String::new());
    for source_text in source_texts {
        print!("```");
        print!("{}", source_text);
        println!("```");
        let before = WorkSnapshot::measure();

        // THE CORE OF THE REPL
        source_program.set_text(&mut db).to(source_text.to_string());
        let answer = evaluate::evaluate_source_program(&db, source_program);

        let after = WorkSnapshot::measure();
        let delta = before.delta(after);
        let WorkSnapshot {
            compile_steps,
            parse_steps,
            resolve_steps,
            type_check_program_steps,
            type_check_function_steps,
            evaluate_function_steps,
        } = delta;
        println!("work delta: parse: {parse_steps} resolve: {resolve_steps} check_program: {type_check_program_steps} check_fun: {type_check_function_steps} eval_fun: {evaluate_function_steps}");
        match answer {
            Ok(s) => println!("{s}"),
            Err(d) => eprintln!("{d:#?}"), // FIXME attach ariadne crate or something
        }
    }
}

static SOURCE_TEXT_SEQ: [&'static str; 17] = [
    r#"
fn area_rectangle(w, h) = h * w
fn area_circle(r) = 3.14 * r * r
"#,
    r#"
fn area_rectangle(w, h) = h * w
fn area_circle(r) = 3.14 * r * r
"#,
    r#"
fn area_rectangle(w, h) = h * w
fn area_circle(r) = 3.14 * r * r
print area_rectangle(3, 4)
"#,
    r#"
fn area_rectangle(w, h) = h * w
fn area_circle(r) = 3.14 * r * r
print area_rectangle(3, 4)
"#,
    r#"
fn area_rectangle(w, h) = w * h
fn area_circle(r) = 3.14 * r * r
print area_rectangle(3, 4)
"#,
    r#"
fn area_rectangle(w, h) = h * w
"#,
    r#"
fn area_rectangle(w, h) = h * w
"#,
    r#"
fn area_rectangle(w, h) = h * w
print area_rectangle(3, 4)
"#,
    r#"
fn area_rectangle(w, h) = h * w
print area_rectangle(3, 4)
"#,
    r#"
fn area_rectangle(w, h) = w * h
print area_rectangle(3, 4)
"#,
    r#"
fn area_triangle(b, h) = 0.5 * b * h
fn area_circle(r) = 3.14 * r * r
"#,
    r#"
fn area_triangle(b, h) = 0.5 * b * h
fn area_circle(r) = 3.14 * r * r
print area_triangle(3, 4)
print area_circle(1)
"#,
    r#"
fn area_rectangle(w, h) = w * h
fn area_circle(r) = 3.14 * r * r
print area_rectangle(3, 4)
print area_circle(1)
"#,
    r#"
fn area_rectangle(w, h) = w * h
fn area_circle(r) = 3.14 * r * r
print area_rectangle(3, 4)
print area_circle(1)
"#,
    r#"
fn area_rectangle(w, h) = w * h
fn area_circle(r) = 3.14 * r * r
print area_rectangle(3, 4)
print area_circle(1)
print 2 * 11
"#,
    r#"
fn area_rectangle(w, h) = w * h
fn area_circle(r) = 3.14 * r * r
print area_rectangle(3, 4)
print area_circle(1)
print 2 * 11
"#,
    r#"
fn area_rectangle(w, h) = w * h
fn area_circle(r) = 3.14 * r * r
print area_rectangle(2, 11)
print area_circle(1)
print 2 * 11
"#,
];
