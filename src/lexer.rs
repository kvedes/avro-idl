use crate::ast::{HasDefault, Literal, RawField};
use crate::error::AvroError;
use chumsky::prelude::*;
use std::fs::read_to_string;
use std::path::PathBuf;

/// Parser for the Avro IDL language
pub struct AvroIdlLexer {
    path: PathBuf,
}

impl AvroIdlLexer {
    /// Create a new Avro IDL parser
    pub fn new(path: String) -> Self {
        let mut buf = PathBuf::new();
        buf.push(path);
        AvroIdlLexer { path: buf }
    }

    /// Parse the content of the path given when instantiating the IDLParser
    pub fn parse(&self) -> Result<RawField, AvroError> {
        let src = read_to_string(self.path.to_str().unwrap()).expect("Failed to Avro IDL file!");
        self.parse_idl(src, self.path.clone())
    }

    /// Parse a string containing Avro IDL
    fn parse_idl(&self, src: String, path: PathBuf) -> Result<RawField, AvroError> {
        let lexer = self.create_chumsky_parser();
        let parse_res = lexer.parse(src.clone());
        if parse_res.is_err() {
            // TODO: Fix this weird bit
            lexer.parse(src).unwrap();
        }
        let top_level_parse = parse_res.map_err(|err| {
            AvroError::FailedParsing(err.into_iter().map(|c| c.to_string()).collect())
        })?;

        let RawField::Protocol(name, values, namespace, docstring) = top_level_parse else {
            return Err(AvroError::InvalidASTDataType(
                "Didn't extract protocol".to_string(),
            ));
        };
        let mut res = vec![];
        for val in values.into_iter() {
            // If DataType is Import then load the Protocol and get the values
            match val {
                RawField::Import(import_path) => {
                    // Remove file name from path
                    let mut cur_path = path.parent().expect("No parent folder").to_path_buf();

                    cur_path.push(import_path);
                    let import_src = read_to_string(cur_path.to_str().unwrap())
                        .expect("Failed to load import reference");

                    let import = self.parse_idl(import_src, cur_path)?;

                    let RawField::Protocol(_, im_values, ..) = import else {
                        return Err(AvroError::InvalidASTDataType(
                            "Didn't extract protocol".to_string(),
                        ));
                    };
                    for v in im_values.into_iter() {
                        res.push(v);
                    }
                }
                // Attach protocol namespace
                RawField::Record(rname, rfields, _, ds) => {
                    res.push(RawField::Record(rname, rfields, namespace.clone(), ds))
                }
                // Attach protocol namespace
                RawField::Enum(ename, evalues, edefault, _, ds) => res.push(RawField::Enum(
                    ename,
                    evalues,
                    edefault,
                    namespace.clone(),
                    ds,
                )),
                _ => res.push(val),
            }
        }
        Ok(RawField::Protocol(name, res, namespace, docstring))
    }

    /// Create a parser which can handle a type and the same type as nullable
    /// TODO: Split the nullable and non-nullable case, since it doesn't make sense
    /// for the non-nullable case to have HasDefault<..> instead of  Option<..>. This
    /// is due to HasDefault being created in the first place to handle defaults equal
    /// to null, which can only happen in the nullable case i.e. 'int?' and not 'int'
    fn nullable_primitive_parser(
        &self,
        keyword: String,
        default_value_parser: impl Parser<char, HasDefault<String>, Error = Simple<char>> + Clone,
        primitive_field_factory: fn(String, HasDefault<String>, Option<String>) -> RawField,
        union_field_factory: fn(String, HasDefault<String>, Option<String>) -> RawField,
    ) -> impl Parser<char, RawField, Error = Simple<char>> {
        let docstring_parser = just('/')
            .then_ignore(just('*'))
            .then_ignore(just('*'))
            .then(just("*/").not().repeated().collect::<String>())
            .then_ignore(just('*'))
            .then_ignore(just('/'))
            .padded();

        let default_parser = just('=')
            .padded()
            .ignore_then(default_value_parser)
            .padded()
            .then_ignore(just(';').padded());

        let no_default_parser = just(';').ignored();

        let keyword_parser = text::keyword(keyword.clone())
            .padded()
            .ignore_then(text::ident().padded());

        // Parser for nullable shorthand : string?, int?, float? ...
        let keyword_nullable_parser = text::keyword(keyword)
            .then_ignore(just('?'))
            .padded()
            .ignore_then(text::ident().padded());

        // Regular primitive with default
        let primitive_with_default = docstring_parser
            .clone()
            .or_not()
            .then(keyword_parser.clone())
            .then(default_parser.clone())
            //then(text::ident())
            .map(move |((docstring, name), value)| {
                primitive_field_factory(name, value, docstring.map(|(_, x)| x.trim().to_string()))
            });

        // Regular primitive no default
        let primitive_no_default = docstring_parser
            .clone()
            .or_not()
            .then(keyword_parser)
            .then(no_default_parser)
            .map(move |((docstring, name), _)| {
                primitive_field_factory(
                    name,
                    HasDefault::None,
                    docstring.map(|(_, x)| x.trim().to_string()),
                )
            });

        // Nullable primitive with default
        let primitive_nullable_default = docstring_parser
            .clone()
            .or_not()
            .then(keyword_nullable_parser.clone())
            .then(default_parser)
            .map(move |((docstring, name), value)| {
                union_field_factory(name, value, docstring.map(|(_, x)| x.trim().to_string()))
            });

        // Nullable primitive no default
        let primitive_nullable_no_default = docstring_parser
            .clone()
            .or_not()
            .then(keyword_nullable_parser)
            .then(no_default_parser)
            .map(move |((docstring, name), _)| {
                union_field_factory(
                    name,
                    HasDefault::None,
                    docstring.map(|(_, x)| x.trim().to_string()),
                )
            });

        choice((
            primitive_nullable_default,
            primitive_nullable_no_default,
            primitive_no_default,
            primitive_with_default,
        ))
    }

    fn create_int_default_parser(&self) -> impl Parser<char, RawField, Error = Simple<char>> {
        self.nullable_primitive_parser(
            "int".to_string(),
            text::digits(10)
                .map(|v| HasDefault::Default(Some(v)))
                .or(text::keyword("null").to(HasDefault::Default(None)))
                .or_else(|_| Ok(HasDefault::None)),
            |name, value, docstring| {
                let default = value.map(|v| v.parse::<i32>().unwrap());
                RawField::Int(Some(name), default, docstring)
            },
            |name, value, docstring| {
                let default = value.map(|v| Literal::Int(v.parse::<i32>().unwrap()));
                RawField::Union(
                    Some(name),
                    vec![RawField::Int(None, HasDefault::None, None), RawField::Null],
                    default,
                    docstring,
                )
            },
        )
    }

    fn create_long_default_parser(&self) -> impl Parser<char, RawField, Error = Simple<char>> {
        self.nullable_primitive_parser(
            "long".to_string(),
            text::digits(10)
                .map(|v| HasDefault::Default(Some(v)))
                .or(text::keyword("null").to(HasDefault::Default(None)))
                .or_else(|_| Ok(HasDefault::None)),
            |name, value, docstring| {
                let default = value.map(|v| v.parse::<i64>().unwrap());
                RawField::Long(Some(name), default, docstring)
            },
            |name, value, docstring| {
                let default = value.map(|v| Literal::Long(v.parse::<i64>().unwrap()));
                RawField::Union(
                    Some(name),
                    vec![RawField::Long(None, HasDefault::None, None), RawField::Null],
                    default,
                    docstring,
                )
            },
        )
    }

    fn create_float_default_parser(&self) -> impl Parser<char, RawField, Error = Simple<char>> {
        let frac = just('.').chain(text::digits(10));

        let default_parser = just('-')
            .or_not()
            .chain::<char, _, _>(text::int(10))
            .chain::<char, _, _>(frac)
            .collect::<String>()
            .from_str()
            .unwrapped()
            .labelled("number");

        self.nullable_primitive_parser(
            "float".to_string(),
            default_parser
                .map(|v| HasDefault::Default(Some(v)))
                .or(text::keyword("null").to(HasDefault::Default(None)))
                .or_else(|_| Ok(HasDefault::None)),
            |name, value, docstring| {
                let default = value.map(|v| v.parse::<f32>().unwrap());
                RawField::Float(Some(name), default, docstring)
            },
            |name, value, docstring| {
                let default = value.map(|v| Literal::Float(v.parse::<f32>().unwrap()));
                RawField::Union(
                    Some(name),
                    vec![
                        RawField::Float(None, HasDefault::None, None),
                        RawField::Null,
                    ],
                    default,
                    docstring,
                )
            },
        )
    }

    fn create_double_default_parser(&self) -> impl Parser<char, RawField, Error = Simple<char>> {
        let frac = just('.').chain(text::digits(10));

        let default_parser = just('-')
            .or_not()
            .chain::<char, _, _>(text::int(10))
            .chain::<char, _, _>(frac)
            .collect::<String>()
            .from_str()
            .unwrapped()
            .labelled("number");

        self.nullable_primitive_parser(
            "double".to_string(),
            default_parser
                .map(|v| HasDefault::Default(Some(v)))
                .or(text::keyword("null").to(HasDefault::Default(None)))
                .or_else(|_| Ok(HasDefault::None)),
            |name, value, docstring| {
                let default = value.map(|v| v.parse::<f64>().unwrap());
                RawField::Double(Some(name), default, docstring)
            },
            |name, value, docstring| {
                let default = value.map(|v| Literal::Double(v.parse::<f64>().unwrap()));
                RawField::Union(
                    Some(name),
                    vec![
                        RawField::Double(None, HasDefault::None, None),
                        RawField::Null,
                    ],
                    default,
                    docstring,
                )
            },
        )
    }

    fn create_bool_default_parser(&self) -> impl Parser<char, RawField, Error = Simple<char>> {
        self.nullable_primitive_parser(
            "boolean".to_string(),
            text::keyword("true")
                .to("true".to_string())
                .or(text::keyword("false").to("false".to_string()))
                .map(|v| HasDefault::Default(Some(v)))
                .or(text::keyword("null").to(HasDefault::Default(None)))
                .or_else(|_| Ok(HasDefault::None)),
            |name, value, docstring| {
                let default = value.map(|v| v.parse::<bool>().unwrap());
                RawField::Boolean(Some(name), default, docstring)
            },
            |name, value, docstring| {
                let default = value.map(|v| Literal::Boolean(v.parse::<bool>().unwrap()));
                RawField::Union(
                    Some(name),
                    vec![
                        RawField::Boolean(None, HasDefault::None, None),
                        RawField::Null,
                    ],
                    default,
                    docstring,
                )
            },
        )
    }

    fn create_string_default_parser(&self) -> impl Parser<char, RawField, Error = Simple<char>> {
        let default_parser = just('"')
            .ignore_then(none_of('"').repeated().collect::<String>())
            .then_ignore(just('"'));

        self.nullable_primitive_parser(
            "string".to_string(),
            default_parser
                .map(|v| HasDefault::Default(Some(v)))
                .or(text::keyword("null").to(HasDefault::Default(None)))
                .or_else(|_| Ok(HasDefault::None)),
            |name, value, docstring| RawField::String(Some(name), value, docstring),
            |name, value, docstring| {
                let default = value.map(Literal::String);
                RawField::Union(
                    Some(name),
                    vec![
                        RawField::String(None, HasDefault::None, None),
                        RawField::Null,
                    ],
                    default,
                    docstring,
                )
            },
        )
    }

    /// Create a parser for primitive types (Anything not record or enum)
    fn create_primitive_parser(&self) -> impl Parser<char, RawField, Error = Simple<char>> {
        choice((
            self.create_int_default_parser(),
            self.create_long_default_parser(),
            self.create_float_default_parser(),
            self.create_double_default_parser(),
            self.create_bool_default_parser(),
            self.create_string_default_parser(),
        ))
    }

    /// Create float/double parser
    fn double_parser(&self) -> impl Parser<char, String, Error = Simple<char>> {
        let frac = just('.').chain(text::digits(10));

        just('-')
            .or_not()
            .chain::<char, _, _>(text::int(10))
            .chain::<char, _, _>(frac)
            .collect::<String>()
            .from_str()
            .unwrapped()
            .labelled("number")
    }

    /// Create the actual text parser
    fn create_chumsky_parser(&self) -> impl Parser<char, RawField, Error = Simple<char>> {
        // Parser for docstrings
        let docstring_parser = just('/')
            .then_ignore(just('*'))
            .then_ignore(just('*'))
            .then(just("*/").not().repeated().collect::<String>())
            .then_ignore(just('*'))
            .then_ignore(just('/'))
            .padded();

        // Parser to identify the protocol and get the protocol name
        let protocol_start = text::keyword("protocol")
            .padded()
            .ignored()
            .then(text::ident())
            .then_ignore(just('{').padded());

        // Annotations e.g. @namespace(), @logicalType()
        let namespace = just('@')
            .ignored()
            .then_ignore(text::keyword("namespace"))
            .then_ignore(just('('))
            .then_ignore(just('"'))
            .then(none_of('"').repeated().collect::<String>())
            .then_ignore(just('"'))
            .then_ignore(just(')'));

        let path = none_of('"').repeated();

        // Check for imports
        let import = text::keyword("import")
            .padded()
            .ignored()
            .then_ignore(text::keyword("idl").padded())
            .then_ignore(just('"'))
            .then(path)
            .then_ignore(just('"'))
            .then_ignore(just(';').padded())
            .map(|(_, path)| RawField::Import(path.into_iter().collect()));

        // Record/Enum reference parser: Handle references to other records
        let ref_parser = docstring_parser
            .clone()
            .or_not()
            .then(text::ident())
            .padded()
            .then(text::ident().padded())
            .then_ignore(just(';'))
            .map(|((docstring, type_), name)| {
                RawField::Unresolved(
                    Some(name),
                    type_,
                    docstring.map(|(_, x)| x.trim().to_string()),
                )
            });

        // Record/Enum optional reference parser: Handle references to other records
        let ref_parser_optional = docstring_parser
            .clone()
            .or_not()
            .then(text::ident())
            .then_ignore(just('?'))
            .padded()
            .then(text::ident().padded())
            .then_ignore(just(';'))
            .map(|((docstring, type_), name)| {
                RawField::Union(
                    Some(name.clone()),
                    vec![RawField::Unresolved(None, type_, None), RawField::Null],
                    HasDefault::None,
                    docstring.map(|(_, x)| x.trim().to_string()),
                )
            });

        // Enum (Working parser but with trailing comma)
        let enum_parser_plain = docstring_parser
            .clone()
            .or_not()
            .then_ignore(text::keyword("enum"))
            .padded()
            .then(text::ident()) // name
            .then_ignore(just('{').padded())
            .then(text::ident().padded().then_ignore(just(',')).repeated())
            .then(text::ident().padded()); // Inner enum values;

        let enum_parser = enum_parser_plain
            .clone()
            .then_ignore(just('}').padded())
            .then_ignore(just('=').padded())
            .then(text::ident().padded()) // Default value
            .then_ignore(just(';').padded())
            .map(
                |((((docstring, name), mut values), last_value), default_value)| {
                    values.push(last_value);
                    RawField::Enum(
                        Some(name),
                        values,
                        HasDefault::Default(Some(default_value)),
                        None,
                        docstring.map(|(_, x)| x.trim().to_string()),
                    )
                },
            )
            .or(enum_parser_plain.then_ignore(just('}').padded()).map(
                |(((docstring, name), mut values), last_value)| {
                    values.push(last_value);
                    RawField::Enum(
                        Some(name),
                        values,
                        HasDefault::None,
                        None,
                        docstring.map(|(_, x)| x.trim().to_string()),
                    )
                },
            ));

        // Unnamed type parser
        let unnamed_type_parser = text::keyword("int")
            .padded()
            .to(RawField::Int(None, HasDefault::None, None))
            .or(text::keyword("long")
                .padded()
                .to(RawField::Long(None, HasDefault::None, None)))
            .or(text::keyword("float")
                .padded()
                .to(RawField::Float(None, HasDefault::None, None)))
            .or(text::keyword("double")
                .padded()
                .to(RawField::Double(None, HasDefault::None, None)))
            .or(text::keyword("boolean").padded().to(RawField::Boolean(
                None,
                HasDefault::None,
                None,
            )))
            .or(text::keyword("string")
                .padded()
                .to(RawField::String(None, HasDefault::None, None)))
            .or(text::keyword("null").padded().to(RawField::Null))
            .or(text::ident()
                .padded()
                .map(|value| RawField::Unresolved(None, value, None)));

        // Multiple comma separated unnamed type parameters
        let mult_unnamed_type_parser = unnamed_type_parser.clone().separated_by(just(',').padded());

        // Array parser
        let array_parser_plain = docstring_parser
            .clone()
            .or_not()
            .then_ignore(text::keyword("array"))
            .then_ignore(just('<'))
            .then(unnamed_type_parser.clone())
            .then_ignore(just('>'));

        let array_parser = array_parser_plain
            .clone()
            .then(text::ident().padded())
            .then_ignore(just(';'))
            .map(|values| {
                let ((docstring, field), name) = values;
                RawField::Array(
                    Some(name),
                    Box::new(field),
                    HasDefault::None,
                    docstring.map(|(_, x)| x.trim().to_string()),
                )
            });

        // Union literal parser
        let union_literal_parser = text::keyword("false")
            .to(Literal::Boolean(false))
            .or(text::keyword("true").to(Literal::Boolean(true)))
            .or(text::keyword("null").to(Literal::Null))
            .or(self
                .double_parser()
                .map(|value| Literal::Double(value.parse::<f64>().unwrap())))
            .or(text::digits(10).map(|value: String| Literal::Int(value.parse::<i32>().unwrap())))
            .or(just('"')
                .ignore_then(text::ident())
                .then_ignore(just('"'))
                .map(Literal::String))
            .padded();

        // Union parser
        let union_parser_plain = docstring_parser
            .clone()
            .or_not()
            .then_ignore(text::keyword("union").padded())
            .then_ignore(just('{').padded())
            .then(mult_unnamed_type_parser)
            .then_ignore(just('}').padded())
            .then(text::ident().padded());

        let union_parser = union_parser_plain
            .clone()
            .then_ignore(just(';').padded())
            .map(|((docstring, values), name)| {
                RawField::Union(
                    Some(name),
                    values,
                    HasDefault::None,
                    docstring.map(|(_, x)| x.trim().to_string()),
                )
            })
            .or(union_parser_plain
                .then_ignore(just('=').padded())
                .then(union_literal_parser)
                .then_ignore(just(';').padded())
                .map(|(((docstring, values), name), default_value)| {
                    RawField::Union(
                        Some(name),
                        values,
                        if let Literal::Null = default_value {
                            HasDefault::Default(None)
                        } else {
                            HasDefault::Default(Some(default_value))
                        },
                        docstring.map(|(_, x)| x.trim().to_string()),
                    )
                }));

        // Record parser
        let record_parser = docstring_parser
            .clone()
            .or_not()
            .then_ignore(text::keyword("record"))
            .padded()
            .then(text::ident()) // Record name
            .then_ignore(just('{').padded())
            .then(
                self.create_primitive_parser()
                    .or(array_parser)
                    .or(ref_parser)
                    .or(ref_parser_optional)
                    .or(union_parser)
                    .repeated(),
            ) // Parse content
            .then_ignore(just('}').padded())
            .map(|((docstring, name), primitives)| {
                RawField::Record(
                    Some(name),
                    primitives,
                    None,
                    docstring.map(|(_, x)| x.trim().to_string()),
                )
            });

        // Put the whole thing together and notice check for ending of the file
        namespace
            .or_not()
            .then(protocol_start)
            .then(choice((import, record_parser, enum_parser)).repeated())
            .then_ignore(just('}').padded())
            .then_ignore(end())
            .map(|((namespace, (_, name)), fields)| {
                RawField::Protocol(Some(name), fields, namespace.map(|(_, ns)| ns), None)
                // TODO: Docstring
            })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::{HasDefault, Literal, RawField};

    use super::AvroIdlLexer;

    #[test]
    fn test_single_protocol() {
        let src = "protocol Event {
    
    enum Meal {
        Dinner,
        Lunch
    } = Dinner;

    record Lol {
        int a;
        Tob tob;
    }

}";
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![
                RawField::Enum(
                    Some("Meal".to_string()),
                    vec!["Dinner".to_string(), "Lunch".to_string()],
                    HasDefault::Default(Some("Dinner".to_string())),
                    None,
                    None,
                ),
                RawField::Record(
                    Some("Lol".to_string()),
                    vec![
                        RawField::Int(Some("a".to_string()), HasDefault::None, None),
                        RawField::Unresolved(Some("tob".to_string()), "Tob".to_string(), None),
                    ],
                    None,
                    None,
                ),
            ],
            None,
            None,
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn test_enum() {
        let src = "protocol Event {
        enum Meal {
            Dinner,
            Lunch
        } = Dinner;

        enum House {
            Apartment,
            Cottage
        }
    }";
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![
                RawField::Enum(
                    Some("Meal".to_string()),
                    vec!["Dinner".to_string(), "Lunch".to_string()],
                    HasDefault::Default(Some("Dinner".to_string())),
                    None,
                    None,
                ),
                RawField::Enum(
                    Some("House".to_string()),
                    vec!["Apartment".to_string(), "Cottage".to_string()],
                    HasDefault::None,
                    None,
                    None,
                ),
            ],
            None,
            None,
        );

        assert_eq!(res, expected);
    }

    #[test]
    fn test_int() {
        let src = "protocol Event {
    
        record Tob {
            int? a = 3;
            int? b;
            int? c = null;
        }    
    }";
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![RawField::Record(
                Some("Tob".to_string()),
                vec![
                    RawField::Union(
                        Some("a".to_string()),
                        vec![RawField::Int(None, HasDefault::None, None), RawField::Null],
                        HasDefault::Default(Some(Literal::Int(3))),
                        None,
                    ),
                    RawField::Union(
                        Some("b".to_string()),
                        vec![RawField::Int(None, HasDefault::None, None), RawField::Null],
                        HasDefault::None,
                        None,
                    ),
                    RawField::Union(
                        Some("c".to_string()),
                        vec![RawField::Int(None, HasDefault::None, None), RawField::Null],
                        HasDefault::Default(None),
                        None,
                    ),
                ],
                None,
                None,
            )],
            None,
            None,
        );
        assert_eq!(res, expected);
    }
    #[test]
    fn test_long() {
        let src = "protocol Event {
    
        record Tob {
            long? a = 3;
            long? b;
            long? c = null;
            long d = 1;
            long e;
        }    
    }";
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![RawField::Record(
                Some("Tob".to_string()),
                vec![
                    RawField::Union(
                        Some("a".to_string()),
                        vec![RawField::Long(None, HasDefault::None, None), RawField::Null],
                        HasDefault::Default(Some(Literal::Long(3))),
                        None,
                    ),
                    RawField::Union(
                        Some("b".to_string()),
                        vec![RawField::Long(None, HasDefault::None, None), RawField::Null],
                        HasDefault::None,
                        None,
                    ),
                    RawField::Union(
                        Some("c".to_string()),
                        vec![RawField::Long(None, HasDefault::None, None), RawField::Null],
                        HasDefault::Default(None),
                        None,
                    ),
                    RawField::Long(Some("d".to_string()), HasDefault::Default(Some(1)), None),
                    RawField::Long(Some("e".to_string()), HasDefault::None, None),
                ],
                None,
                None,
            )],
            None,
            None,
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn test_float() {
        let src = "protocol Event {
    
        record Numbers {
            float? a = 3.0;
            float? b;
            float? c = null;
            float d = 1.0;
            float e;
        }    
    }";
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![RawField::Record(
                Some("Numbers".to_string()),
                vec![
                    RawField::Union(
                        Some("a".to_string()),
                        vec![
                            RawField::Float(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::Default(Some(Literal::Float(3.0))),
                        None,
                    ),
                    RawField::Union(
                        Some("b".to_string()),
                        vec![
                            RawField::Float(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::None,
                        None,
                    ),
                    RawField::Union(
                        Some("c".to_string()),
                        vec![
                            RawField::Float(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::Default(None),
                        None,
                    ),
                    RawField::Float(Some("d".to_string()), HasDefault::Default(Some(1.0)), None),
                    RawField::Float(Some("e".to_string()), HasDefault::None, None),
                ],
                None,
                None,
            )],
            None,
            None,
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn test_double() {
        let src = "protocol Event {
    
        record Numbers {
            double? a = 3.0;
            double? b;
            double? c = null;
            double d = 1.0;
            double e;
        }    
    }";
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![RawField::Record(
                Some("Numbers".to_string()),
                vec![
                    RawField::Union(
                        Some("a".to_string()),
                        vec![
                            RawField::Double(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::Default(Some(Literal::Double(3.0))),
                        None,
                    ),
                    RawField::Union(
                        Some("b".to_string()),
                        vec![
                            RawField::Double(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::None,
                        None,
                    ),
                    RawField::Union(
                        Some("c".to_string()),
                        vec![
                            RawField::Double(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::Default(None),
                        None,
                    ),
                    RawField::Double(Some("d".to_string()), HasDefault::Default(Some(1.0)), None),
                    RawField::Double(Some("e".to_string()), HasDefault::None, None),
                ],
                None,
                None,
            )],
            None,
            None,
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn test_bool() {
        let src = "protocol Event {
    
        record Numbers {
            boolean? a = true;
            boolean? b;
            boolean? c = null;
            boolean d = false;
            boolean e;
        }    
    }";
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![RawField::Record(
                Some("Numbers".to_string()),
                vec![
                    RawField::Union(
                        Some("a".to_string()),
                        vec![
                            RawField::Boolean(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::Default(Some(Literal::Boolean(true))),
                        None,
                    ),
                    RawField::Union(
                        Some("b".to_string()),
                        vec![
                            RawField::Boolean(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::None,
                        None,
                    ),
                    RawField::Union(
                        Some("c".to_string()),
                        vec![
                            RawField::Boolean(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::Default(None),
                        None,
                    ),
                    RawField::Boolean(
                        Some("d".to_string()),
                        HasDefault::Default(Some(false)),
                        None,
                    ),
                    RawField::Boolean(Some("e".to_string()), HasDefault::None, None),
                ],
                None,
                None,
            )],
            None,
            None,
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn test_string() {
        let src = "protocol Event {
    
        record Numbers {
            string? a = \"hello there\";
            string? b;
            string? c = null;
            string d = \" what ?? is !! going //on<<z\";
            string e;
        }    
    }";
        //
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![RawField::Record(
                Some("Numbers".to_string()),
                vec![
                    RawField::Union(
                        Some("a".to_string()),
                        vec![
                            RawField::String(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::Default(Some(Literal::String("hello there".to_string()))),
                        None,
                    ),
                    RawField::Union(
                        Some("b".to_string()),
                        vec![
                            RawField::String(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::None,
                        None,
                    ),
                    RawField::Union(
                        Some("c".to_string()),
                        vec![
                            RawField::String(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::Default(None),
                        None,
                    ),
                    RawField::String(
                        Some("d".to_string()),
                        HasDefault::Default(Some(" what ?? is !! going //on<<z".to_string())),
                        None,
                    ),
                    RawField::String(Some("e".to_string()), HasDefault::None, None),
                ],
                None,
                None,
            )],
            None,
            None,
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn test_record_reference() {
        let src = "protocol Event {
    
        record A {
            string e;
        }

        record B {
            A a;
            A? b;
        }    
    }";
        //
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![
                RawField::Record(
                    Some("A".to_string()),
                    vec![RawField::String(
                        Some("e".to_string()),
                        HasDefault::None,
                        None,
                    )],
                    None,
                    None,
                ),
                RawField::Record(
                    Some("B".to_string()),
                    vec![
                        RawField::Unresolved(Some("a".to_string()), "A".to_string(), None),
                        RawField::Union(
                            Some("b".to_string()),
                            vec![
                                RawField::Unresolved(None, "A".to_string(), None),
                                RawField::Null,
                            ],
                            HasDefault::None,
                            None,
                        ),
                    ],
                    None,
                    None,
                ),
            ],
            None,
            None,
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn test_single_array_in_record() {
        let src = "protocol Event {
    
        record A {
            array<int> myints;
        }
    }";
        //
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![RawField::Record(
                Some("A".to_string()),
                vec![RawField::Array(
                    Some("myints".to_string()),
                    Box::new(RawField::Int(None, HasDefault::None, None)),
                    HasDefault::None,
                    None,
                )],
                None,
                None,
            )],
            None,
            None,
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn test_multiple_arrays_in_record() {
        let src = "protocol Event {
    
        record A {
            array<int> myints;
            /** hi */
            array<string> mystrs;
            /** hi */
            array<float> myfloats;
        }
    }";
        //
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![RawField::Record(
                Some("A".to_string()),
                vec![
                    RawField::Array(
                        Some("myints".to_string()),
                        Box::new(RawField::Int(None, HasDefault::None, None)),
                        HasDefault::None,
                        None,
                    ),
                    RawField::Array(
                        Some("mystrs".to_string()),
                        Box::new(RawField::String(None, HasDefault::None, None)),
                        HasDefault::None,
                        Some(String::from("hi")),
                    ),
                    RawField::Array(
                        Some("myfloats".to_string()),
                        Box::new(RawField::Float(None, HasDefault::None, None)),
                        HasDefault::None,
                        Some(String::from("hi")),
                    ),
                ],
                None,
                None,
            )],
            None,
            None,
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn test_single_union_in_record() {
        let src = "protocol Event {
    
        record A {
            union{int, float} mynum;
        }
    }";
        //
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![RawField::Record(
                Some("A".to_string()),
                vec![RawField::Union(
                    Some("mynum".to_string()),
                    vec![
                        RawField::Int(None, HasDefault::None, None),
                        RawField::Float(None, HasDefault::None, None),
                    ],
                    HasDefault::None,
                    None,
                )],
                None,
                None,
            )],
            None,
            None,
        );
        assert_eq!(res, expected);
    }

    #[test]
    fn test_multiple_unions_in_record() {
        let src = "protocol Event {
    
        record A {
            union{int, float} mynum;
            union{int, string} myval;
            union{int, null} myval2;
            union{double, null} myval3 = null;
            union{double, null} myval4 = -0.21;
        }
    }";
        //
        let idl = AvroIdlLexer::new("none".to_string());
        let res = idl.parse_idl(src.to_string(), PathBuf::new()).unwrap();
        let expected = RawField::Protocol(
            Some("Event".to_string()),
            vec![RawField::Record(
                Some("A".to_string()),
                vec![
                    RawField::Union(
                        Some("mynum".to_string()),
                        vec![
                            RawField::Int(None, HasDefault::None, None),
                            RawField::Float(None, HasDefault::None, None),
                        ],
                        HasDefault::None,
                        None,
                    ),
                    RawField::Union(
                        Some("myval".to_string()),
                        vec![
                            RawField::Int(None, HasDefault::None, None),
                            RawField::String(None, HasDefault::None, None),
                        ],
                        HasDefault::None,
                        None,
                    ),
                    RawField::Union(
                        Some("myval2".to_string()),
                        vec![RawField::Int(None, HasDefault::None, None), RawField::Null],
                        HasDefault::None,
                        None,
                    ),
                    RawField::Union(
                        Some("myval3".to_string()),
                        vec![
                            RawField::Double(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::Default(None),
                        None,
                    ),
                    RawField::Union(
                        Some("myval4".to_string()),
                        vec![
                            RawField::Double(None, HasDefault::None, None),
                            RawField::Null,
                        ],
                        HasDefault::Default(Some(Literal::Double(-0.21))),
                        None,
                    ),
                ],
                None,
                None,
            )],
            None,
            None,
        );
        assert_eq!(res, expected);
    }
}
