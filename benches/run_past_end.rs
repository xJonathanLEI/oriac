#![allow(clippy::unit_arg)]

use std::{collections::HashMap, rc::Rc};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oriac::cairo::lang::{
    compiler::program::{FullProgram, Program},
    instances::CairoLayout,
    vm::{cairo_runner::CairoRunner, memory_dict::MemoryDict},
};

pub fn criterion_benchmark(c: &mut Criterion) {
    let program: Rc<Program> = Rc::new(
        serde_json::from_str::<FullProgram>(include_str!(
            "../test-data/artifacts/run_past_end.json"
        ))
        .unwrap()
        .into(),
    );

    c.bench_function("run_past_end", |b| {
        b.iter(|| {
            black_box({
                let mut runner = CairoRunner::new(
                    program.clone(),
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
            });
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
