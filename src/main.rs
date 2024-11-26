use avro_idl::runner::{AvroIdlParser, OutputFormat};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    //#[arg(value_enum, short, long, default_value_t = OutputFormat::AVPR)]
    path: String,
    output_path: String,
    format: Option<OutputFormat>,
}

fn main() {
    let args = Args::parse();

    let runner = AvroIdlParser::new(
        args.path,
        args.output_path,
        args.format.unwrap_or(OutputFormat::AVPR),
    );
    runner.parse();
}
