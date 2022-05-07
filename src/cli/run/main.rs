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
    str::FromStr,
};

#[derive(Debug)]
enum Layout {
    Plain,
    Small,
}

#[derive(Debug, Parser)]
#[clap(author, version, about = "A tool to run Cairo programs.", long_about = None)]
struct Args {
    #[clap(long, help = "The name of the program json file.")]
    program: PathBuf,
    #[clap(long, help = "The layout of the Cairo AIR.", default_value = "plain", possible_values = ["plain", "small"])]
    layout: Layout,
    #[clap(
        long,
        help = "Prints the program output (if the output builtin is used)."
    )]
    print_output: bool,
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

    let instance = match args.layout {
        Layout::Plain => CairoLayout::plain_instance(),
        Layout::Small => CairoLayout::small_instance(),
    };

    let mut runner = CairoRunner::new(
        Rc::new(program.into()),
        instance,
        MemoryDict::new(),
        false,
        false,
    )
    .unwrap();

    runner.initialize_segments().unwrap();
    let end = runner.initialize_main_entrypoint().unwrap();

    runner.initialize_vm(HashMap::new(), ()).unwrap();

    runner.run_until_pc(end.into(), None).unwrap();

    runner.end_run(false, false).unwrap();

    runner.read_return_values().unwrap();

    if args.print_output {
        runner.print_output().unwrap();
    }

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

impl FromStr for Layout {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "plain" => Ok(Layout::Plain),
            "small" => Ok(Layout::Small),
            _ => Err("unknown layout"),
        }
    }
}
