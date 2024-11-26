use crate::{ast::HasDefault, error::AvroError};

/// Serialize a Protocol to avsc format
///
///
use super::ast::Field;
use serde_json::{json, Value};

pub struct AvprSerializer {
    protocol: Field,
}

impl AvprSerializer {
    pub fn new(protocol: Field) -> Self {
        Self { protocol }
    }

    pub fn serialize(&self) -> Result<Value, AvroError> {
        self.serialize_field(self.protocol.clone())
    }

    fn serialize_field(&self, field: Field) -> Result<Value, AvroError> {
        let cf = field.clone();
        match field {
            Field::Protocol(name, inner_fields, namespace, docstring) => match name {
                Some(n) => {
                    let mut json_data = json!({"protocol": n, "types": inner_fields.into_iter().map(|f| self.serialize_field(f).unwrap()).collect::<Vec<Value>>()});
                    if let Some(ns) = namespace {
                        json_data["namespace"] = json!(ns);
                    }
                    if let Some(ds) = docstring {
                        json_data["doc"] = json!(ds);
                    }
                    Ok(json_data)
                }
                None => Err(AvroError::MissingName(
                    "Protocol doesn't have a name, but this is required!".to_string(),
                )),
            },
            Field::Int(name, default, docstring) => {
                let mut json_data = json!({"name": name, "type": cf.get_avro_type_name().unwrap()});
                match default {
                    HasDefault::Default(Some(v)) => json_data["default"] = json!(v),
                    HasDefault::Default(None) => json_data["default"] = json!(None::<String>), // TODO: This case cannot happen, since this Field is not nullable
                    HasDefault::None => (),
                };
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::Long(name, default, docstring) => {
                let mut json_data = json!({"name": name, "type": cf.get_avro_type_name().unwrap()});
                match default {
                    HasDefault::Default(Some(v)) => json_data["default"] = json!(v),
                    HasDefault::Default(None) => json_data["default"] = json!(None::<String>), // TODO: This case cannot happen, since this Field is not nullable
                    HasDefault::None => (),
                };
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::Float(name, default, docstring) => {
                let mut json_data = json!({"name": name, "type": cf.get_avro_type_name().unwrap()});
                match default {
                    HasDefault::Default(Some(v)) => json_data["default"] = json!(v),
                    HasDefault::Default(None) => json_data["default"] = json!(None::<String>), // TODO: This case cannot happen, since this Field is not nullable
                    HasDefault::None => (),
                };
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::Double(name, default, docstring) => {
                let mut json_data = json!({"name": name, "type": cf.get_avro_type_name().unwrap()});
                match default {
                    HasDefault::Default(Some(v)) => json_data["default"] = json!(v),
                    HasDefault::Default(None) => json_data["default"] = json!(None::<String>), // TODO: This case cannot happen, since this Field is not nullable
                    HasDefault::None => (),
                };
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::Boolean(name, default, docstring) => {
                let mut json_data = json!({"name": name, "type": cf.get_avro_type_name().unwrap()});
                match default {
                    HasDefault::Default(Some(v)) => json_data["default"] = json!(v),
                    HasDefault::Default(None) => json_data["default"] = json!(None::<String>), // TODO: This case cannot happen, since this Field is not nullable
                    HasDefault::None => (),
                };
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::String(name, default, docstring) => {
                let mut json_data = json!({"name": name, "type": cf.get_avro_type_name().unwrap()});
                match default {
                    HasDefault::Default(Some(v)) => json_data["default"] = json!(v),
                    HasDefault::Default(None) => json_data["default"] = json!(None::<String>), // TODO: This case cannot happen, since this Field is not nullable
                    HasDefault::None => (),
                };
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::Record(name, inner_fields, namespace, docstring) => {
                let mut json_data = json!({"type": cf.get_avro_type_name().unwrap(), "name": name, "fields": inner_fields
                .into_iter()
                .map(|f| self.serialize_field(f).unwrap())
                .collect::<Vec<Value>>()});
                if let Some(ns) = namespace {
                    json_data["namespace"] = json!(ns);
                }
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::Enum(name, symbols, default, namespace, docstring) => {
                let mut json_data = json!({"type": cf.get_avro_type_name().unwrap(), "name": name, "symbols": symbols});
                match default {
                    HasDefault::Default(Some(v)) => json_data["default"] = json!(v),
                    HasDefault::Default(None) => json_data["default"] = json!(None::<String>), // TODO: This case cannot happen, since this Field is not nullable
                    HasDefault::None => (),
                };
                if let Some(ns) = namespace {
                    json_data["namespace"] = json!(ns);
                }
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::Union(name, inner_fields, default, docstring) => {
                let mut json_data = json!({
                    "type": "Union",
                    "name": name,
                    "type": inner_fields.into_iter()
                        .filter_map(|f| f.get_avro_type_name())
                        .collect::<Vec<String>>()
                });
                match default {
                    HasDefault::Default(Some(v)) => json_data["default"] = json!(v),
                    HasDefault::Default(None) => json_data["default"] = json!(None::<String>), // TODO: This case cannot happen, since this Field is not nullable
                    HasDefault::None => (),
                };
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::Array(name, inner_field, _, docstring) => {
                let mut json_data = json!({"type": "array", "name": name, "items": inner_field.get_avro_type_name().unwrap()});
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::RecordReference(name, type_, docstring) => {
                let mut json_data = json!({"name": name, "type": type_});
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::EnumReference(name, type_, default, docstring) => {
                let mut json_data = json!({"name": name, "type": type_});
                match default {
                    HasDefault::Default(Some(v)) => json_data["default"] = json!(v),
                    HasDefault::Default(None) => json_data["default"] = json!(None::<String>), // TODO: This case cannot happen, since this Field is not nullable
                    HasDefault::None => (),
                };
                if let Some(ds) = docstring {
                    json_data["doc"] = json!(ds);
                }
                Ok(json_data)
            }
            Field::Null => Err(AvroError::InvalidASTDataType(
                "Cannot serialize Field::Null!".to_string(),
            )),
        }
    }

    // fn serialize_enum(enum_: Enum) -> Value {
    //     json!({"type": "enum", "name": enum_.name, "symbols": enum_.values})
    // }

    // fn serialize_record(record: Record) -> Value {
    //     let fields: Vec<Value> = record
    //         .fields
    //         .into_iter()
    //         .map(|(field_name, field_type)| {
    //             AVSCSerializer::serialize_record_field(field_name, field_type)
    //         })
    //         .collect();
    //     json!({
    //         "name": record.name,
    //         "type": "record",
    //         "fields": fields})
    // }

    // fn serialize_record_field(name: String, type_: DataType) -> Value {
    //     match type_ {
    //         DataType::Int => json!({"name": name, "type": "int"}),
    //         DataType::Double => json!({"name": name, "type": "double"}),
    //         DataType::String => json!({"name": name, "type": "string"}),
    //         DataType::Array(inner) => {
    //             json!({"name": name, "type": "array", "items": AVSCSerializer::serialize_record_field(name, *inner)})
    //         }
    //         DataType::Record(rec) => AVSCSerializer::serialize_record(rec),
    //         DataType::Union(_) => json!({"name": name, "type": "union"}),
    //     }
    // }
}

// #[cfg(test)]
// mod tests {
//     use super::AVSCSerializer;
//     use crate::parser::typed::Enum;
//     use serde_json::json;

//     #[test]
//     fn test_serialize_enum() {
//         let en = Enum::new(
//             "hej".to_string(),
//             vec!["a".to_string(), "b".to_string(), "c".to_string()],
//         );
//         let expected = json!({"name": "hej", "symbols": ["a", "b", "c"], "type": "enum"});

//         assert_eq!(AVSCSerializer::serialize_enum(en), expected);
//     }
// }
