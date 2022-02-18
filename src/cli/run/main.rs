use clap::Parser;
use oriac::cairo::lang::{
    compiler::program::FullProgram,
    instances::CairoLayout,
    vm::{cairo_runner::CairoRunner, memory_dict::MemoryDict},
};
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Debug, Parser)]
#[clap(author, version, about = "A tool to run Cairo programs.", long_about = None)]
struct Args {
    #[clap(long, help = "The name of the program json file.")]
    program: PathBuf,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(std::io::Error),
    #[error(transparent)]
    Json(serde_json::Error),
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let program = load_program(&args.program)?;

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

    Ok(())
}

fn load_program(program: &Path) -> Result<FullProgram, Error> {
    let mut file = File::open(program)?;
    Ok(serde_json::from_reader::<_, FullProgram>(&mut file)?)
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
