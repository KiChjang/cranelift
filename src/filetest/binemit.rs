//! Test command for testing the binary machine code emission.
//!
//! The `binemit` test command generates binary machine code for every instruction in the input
//! functions and compares the results to the expected output.

use std::borrow::{Borrow, Cow};
use std::fmt::Write;
use cretonne::binemit;
use cretonne::ir;
use cretonne::ir::entities::AnyEntity;
use cton_reader::TestCommand;
use filetest::subtest::{SubTest, Context, Result};
use utils::match_directive;

struct TestBinEmit;

pub fn subtest(parsed: &TestCommand) -> Result<Box<SubTest>> {
    assert_eq!(parsed.command, "binemit");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestBinEmit))
    }
}

// Code sink that generates text.
struct TextSink {
    text: String,
}

impl binemit::CodeSink for TextSink {
    fn put1(&mut self, x: u8) {
        write!(self.text, "{:02x} ", x).unwrap();
    }

    fn put2(&mut self, x: u16) {
        write!(self.text, "{:04x} ", x).unwrap();
    }

    fn put4(&mut self, x: u32) {
        write!(self.text, "{:08x} ", x).unwrap();
    }

    fn put8(&mut self, x: u64) {
        write!(self.text, "{:016x} ", x).unwrap();
    }

    fn reloc_func(&mut self, _: binemit::Reloc, _: ir::FuncRef) {
        unimplemented!()
    }

    fn reloc_jt(&mut self, _: binemit::Reloc, _: ir::JumpTable) {
        unimplemented!()
    }
}

impl SubTest for TestBinEmit {
    fn name(&self) -> Cow<str> {
        Cow::from("binemit")
    }

    fn is_mutating(&self) -> bool {
        false
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> Result<()> {
        let isa = context.isa.expect("binemit needs an ISA");
        // TODO: Run a verifier pass over the code first to detect any bad encodings or missing/bad
        // value locations. The current error reporting is just crashing...
        let func = func.borrow();

        let mut sink = TextSink { text: String::new() };

        for comment in &context.details.comments {
            if let Some(want) = match_directive(comment.text, "bin:") {
                let inst = match comment.entity {
                    AnyEntity::Inst(inst) => inst,
                    _ => {
                        return Err(format!("annotation on non-inst {}: {}",
                                           comment.entity,
                                           comment.text))
                    }
                };
                sink.text.clear();
                isa.emit_inst(&func, inst, &mut sink);
                let have = sink.text.trim();
                if have != want {
                    return Err(format!("Bad machine code for {}: {}\nWant: {}\nGot:  {}",
                                       inst,
                                       func.dfg.display_inst(inst),
                                       want,
                                       have));
                }
            }
        }

        if sink.text.is_empty() {
            Err("No bin: directives found".to_string())
        } else {
            Ok(())
        }
    }
}