use std::fs::File;

use crate::error::AvroError;
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

    pub fn parse(&self) -> Result<(), AvroError> {
        let lexer = AvroIdlLexer::new(self.path.clone());
        let linker = LinkParser::new();

        let parsed_ast = lexer.parse()?;
        let linked_ast = linker.parse(parsed_ast)?;

        let content = match self.format {
            OutputFormat::AVPR => {
                let serializer = AvprSerializer::new(linked_ast);
                serializer.serialize()?
            }
        };

        let file = File::create(&self.output_path).map_err(|e| AvroError::Io(e.to_string()))?;
        serde_json::to_writer(file, &content).map_err(|e| AvroError::Io(e.to_string()))?;
        Ok(())
    }
}
