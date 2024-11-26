use std::fs::File;

use crate::lexer::AvroIdlLexer;
use crate::linker::LinkParser;
use crate::serializer::AvprSerializer;
use clap::ValueEnum;

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    AVPR,
    // AVSC, TODO: Implement avsc serializer
}

pub struct AvroIdlParser {
    path: String,
    format: OutputFormat,
    output_path: String,
}

impl AvroIdlParser {
    pub fn new(path: String, output_path: String, format: OutputFormat) -> Self {
        Self {
            path,
            format,
            output_path,
        }
    }

    pub fn parse(&self) {
        let lexer = AvroIdlLexer::new(self.path.clone());
        let linker = LinkParser::new();

        let parsed_ast = lexer.parse().unwrap();
        let linked_ast = linker.parse(parsed_ast).unwrap();

        let content = match self.format {
            OutputFormat::AVPR => {
                let serializer = AvprSerializer::new(linked_ast);
                serializer.serialize().unwrap()
            } //OutputFormat::AVSC => panic!("Not supported AVSC"),
        };

        let file = File::create(self.output_path.clone()).unwrap();

        serde_json::to_writer(file, &content).unwrap();
    }
}
