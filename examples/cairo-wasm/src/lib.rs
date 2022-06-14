#![allow(clippy::unused_unit)]

use oriac::cairo::lang::{
    compiler::program::FullProgram,
    instances::CairoLayout,
    vm::{cairo_runner::CairoRunner, memory_dict::MemoryDict},
};
use std::{collections::HashMap, rc::Rc};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_program(program: &str) {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    let program: FullProgram = serde_json::from_str(program).unwrap();

    let mut runner = CairoRunner::new(
        Rc::new(program.into()),
        CairoLayout::plain_instance(),
        MemoryDict::new(),
        false,
        false,
    )
    .unwrap();

    runner.initialize_segments();
    let end = runner.initialize_main_entrypoint().unwrap();

    runner.initialize_vm(HashMap::new(), None).unwrap();

    runner.run_until_pc(end.into(), None).unwrap();

    runner.end_run(false, false).unwrap();
}
