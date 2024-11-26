use crate::{ast::Field, ast::RawField, error::AvroError};

pub struct LinkParser {}

impl LinkParser {
    pub fn new() -> Self {
        LinkParser {}
    }

    pub fn parse(&self, protocol: RawField) -> Result<Field, AvroError> {
        let dup_protocol = protocol.clone();

        let RawField::Protocol(name, fields, namespace, docstring) = protocol else {
            return Err(AvroError::InvalidASTDataType(
                "Expected a protocol".to_string(),
            ));
        };

        let linked_fields = fields
            .into_iter()
            .map(|field| self.parse_recurse(&dup_protocol, field).unwrap())
            .collect();
        Ok(Field::Protocol(name, linked_fields, namespace, docstring))
    }

    fn parse_recurse(&self, protocol: &RawField, field: RawField) -> Result<Field, AvroError> {
        match field {
            RawField::Int(name, default, docstring) => Ok(Field::Int(name, default, docstring)),
            RawField::Long(name, default, docstring) => Ok(Field::Long(name, default, docstring)),
            RawField::Float(name, default, docstring) => Ok(Field::Float(name, default, docstring)),
            RawField::Double(name, default, docstring) => {
                Ok(Field::Double(name, default, docstring))
            }
            RawField::Boolean(name, default, docstring) => {
                Ok(Field::Boolean(name, default, docstring))
            }
            RawField::String(name, default, docstring) => {
                Ok(Field::String(name, default, docstring))
            }
            RawField::Enum(name, values, default, namespace, docstring) => {
                Ok(Field::Enum(name, values, default, namespace, docstring))
            }
            RawField::Record(name, fields, namespace, docstring) => {
                let linked_fields = fields
                    .into_iter()
                    .map(|f| self.parse_recurse(protocol, f).unwrap())
                    .collect();
                Ok(Field::Record(name, linked_fields, namespace, docstring))
            }
            RawField::Unresolved(_name, value, docstring) => {
                let Some(ref_field) = protocol.find_field_by_name(value.clone()) else {
                    return Err(AvroError::UndefinedReference(format!(
                        "Field of type '{}' cannot be found!",
                        value
                    )));
                };
                match ref_field {
                    RawField::Record(..) => Ok(Field::RecordReference(_name, value, docstring)),
                    RawField::Enum(_, _, default, ..) => {
                        Ok(Field::EnumReference(_name, value, default, docstring))
                    }
                    _ => Err(AvroError::InvalidASTDataType(
                        "Only Record and Enum are valid references!".to_string(),
                    )),
                }
            }
            RawField::Union(name, fields, default, docstring) => {
                let linked_fields = fields
                    .into_iter()
                    .map(|f| self.parse_recurse(protocol, f).unwrap())
                    .collect();
                Ok(Field::Union(name, linked_fields, default, docstring))
            }
            RawField::Protocol(..) => Err(AvroError::InvalidASTDataType(
                "'Protocol' can only be declared once per file!".to_string(),
            )),
            RawField::Array(name, inner_field, default, docstring) => Ok(Field::Array(
                name,
                Box::new(self.parse_recurse(protocol, *inner_field).unwrap()),
                default,
                docstring,
            )),
            RawField::Import(_) => Err(AvroError::InvalidASTDataType(
                "'Import' should have been resolved previous to Linking!".to_string(),
            )),
            RawField::Null => Ok(Field::Null),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::ast::{Field, HasDefault, RawField};

    use super::LinkParser;

    #[test]
    fn test_record_reference_resolve() {
        let src = RawField::Protocol(
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
                        RawField::Unresolved(Some("meal".to_string()), "Meal".to_string(), None),
                    ],
                    None,
                    None,
                ),
            ],
            None,
            None,
        );

        let linker = LinkParser::new();
        let res = linker.parse(src).unwrap();
        let expected = Field::Protocol(
            Some("Event".to_string()),
            vec![
                Field::Enum(
                    Some("Meal".to_string()),
                    vec!["Dinner".to_string(), "Lunch".to_string()],
                    HasDefault::Default(Some("Dinner".to_string())),
                    None,
                    None,
                ),
                Field::Record(
                    Some("Lol".to_string()),
                    vec![
                        Field::Int(Some("a".to_string()), HasDefault::None, None),
                        Field::EnumReference(
                            Some("meal".to_string()),
                            "Meal".to_string(),
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
}
