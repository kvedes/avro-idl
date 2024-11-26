use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, PartialEq, Clone)]
pub enum HasDefault<T> {
    /// Default set to literal value (HasDefault::Default(Some(1))), default is set to null HasDefault::Default(None)
    Default(Option<T>),
    /// No default set
    None,
}

impl<T> HasDefault<T> {
    pub fn map<U>(self, mapping: fn(T) -> U) -> HasDefault<U> {
        match self {
            HasDefault::Default(Some(value)) => HasDefault::Default(Some(mapping(value))),
            HasDefault::Default(None) => HasDefault::Default(None),
            HasDefault::None => HasDefault::None,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum RawField {
    /// Name, fields, namespace, docstring
    Protocol(
        Option<String>,
        Vec<RawField>,
        Option<String>,
        Option<String>,
    ),
    /// Name, default, docstring
    Int(Option<String>, HasDefault<i32>, Option<String>), // TODO: Change HasDefault<i32> to Option<i32>
    /// Name, default, docstring
    Long(Option<String>, HasDefault<i64>, Option<String>), // TODO: Change HasDefault<..> to Option<..>
    /// Name, default, docstring
    Float(Option<String>, HasDefault<f32>, Option<String>), // TODO: Change HasDefault<..> to Option<..>
    /// Name, default, docstring
    Double(Option<String>, HasDefault<f64>, Option<String>), // TODO: Change HasDefault<..> to Option<..>
    /// Name, default, docstring
    Boolean(Option<String>, HasDefault<bool>, Option<String>), // TODO: Change HasDefault<..> to Option<..>
    /// Name, default, docstring
    String(Option<String>, HasDefault<String>, Option<String>), // TODO: Change HasDefault<..> to Option<..>
    /// Name, values, default, namespace, docstring
    Enum(
        Option<String>,
        Vec<String>,
        HasDefault<String>,
        Option<String>,
        Option<String>,
    ), // TODO: Name might not need to be optional,  // TODO: Change HasDefault<..> to Option<..>
    /// Name, subfields, namespace, docstring
    Record(
        Option<String>,
        Vec<RawField>,
        Option<String>,
        Option<String>,
    ), // TODO: Name might not need to be optional
    /// Name, data types in union, default value, docstring
    Union(
        Option<String>,
        Vec<RawField>,
        HasDefault<Literal>,
        Option<String>,
    ), // TODO: Name might not need to be optional
    /// Name, Field in Array, default value, docstring
    Array(
        Option<String>,
        Box<RawField>,
        HasDefault<Literal>,
        Option<String>,
    ), // TODO: Change HasDefault<..> to Option<..>
    /// Null type needed for representing null in unions
    Null,
    /// Name, Type, docstring
    Unresolved(Option<String>, String, Option<String>),
    /// Path
    Import(String),
}

impl RawField {
    pub fn is_resolved(&self) -> bool {
        matches!(self, RawField::Unresolved(..))
    }

    /// Returns a field by name and sets the default as HasDefault::None
    pub fn find_field_by_name(&self, field_name: String) -> Option<RawField> {
        match self {
            RawField::Protocol(_name, fields, ..) => {
                for field in fields.iter() {
                    let Some(cur_name) = field.name() else {
                        continue;
                    };
                    if cur_name == field_name {
                        return Some(field.clone().remove_default());
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn name(&self) -> Option<String> {
        match self {
            RawField::Protocol(name, ..) => name.clone(),
            RawField::Int(name, ..) => name.clone(),
            RawField::Long(name, ..) => name.clone(),
            RawField::Float(name, ..) => name.clone(),
            RawField::Double(name, ..) => name.clone(),
            RawField::Boolean(name, ..) => name.clone(),
            RawField::String(name, ..) => name.clone(),
            RawField::Record(name, ..) => name.clone(),
            RawField::Enum(name, ..) => name.clone(),
            RawField::Unresolved(name, ..) => name.clone(),
            RawField::Union(name, ..) => name.clone(),
            RawField::Array(name, ..) => name.clone(),
            RawField::Null => None,
            RawField::Import(_) => None,
        }
    }

    fn remove_default(self) -> RawField {
        match self {
            RawField::Int(name, _default, ..) => RawField::Int(name, HasDefault::None, None),
            RawField::Long(name, _default, ..) => RawField::Long(name, HasDefault::None, None),
            RawField::Float(name, _default, ..) => RawField::Float(name, HasDefault::None, None),
            RawField::Double(name, _default, ..) => RawField::Double(name, HasDefault::None, None),
            RawField::Boolean(name, _default, ..) => {
                RawField::Boolean(name, HasDefault::None, None)
            }
            RawField::String(name, _default, ..) => RawField::String(name, HasDefault::None, None),
            RawField::Enum(name, values, _, ns, ..) => {
                RawField::Enum(name, values, HasDefault::None, ns, None)
            }
            _ => self,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Literal {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Boolean(bool),
    String(String),
    Null,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Field {
    /// Name, fields, namespace, docstring
    Protocol(Option<String>, Vec<Field>, Option<String>, Option<String>),
    /// Name, default, docstring
    Int(Option<String>, HasDefault<i32>, Option<String>),
    /// Name, default, docstring
    Long(Option<String>, HasDefault<i64>, Option<String>),
    /// Name, default, docstring
    Float(Option<String>, HasDefault<f32>, Option<String>),
    /// Name, default, docstring
    Double(Option<String>, HasDefault<f64>, Option<String>),
    /// Name, default, docstring
    Boolean(Option<String>, HasDefault<bool>, Option<String>),
    /// Name, default, docstring
    String(Option<String>, HasDefault<String>, Option<String>),
    /// Name, values, default, namespace, docstring
    Enum(
        Option<String>,
        Vec<String>,
        HasDefault<String>,
        Option<String>,
        Option<String>,
    ),
    /// Name, subfields, namespace, docstring
    Record(Option<String>, Vec<Field>, Option<String>, Option<String>),
    /// Name, data types in union, default value, docstring
    Union(
        Option<String>,
        Vec<Field>,
        HasDefault<Literal>,
        Option<String>,
    ),
    /// Name, Field in Array, default value, docstring
    Array(
        Option<String>,
        Box<Field>,
        HasDefault<Literal>,
        Option<String>,
    ),
    /// Name, type, docstring
    RecordReference(Option<String>, String, Option<String>),
    /// Name, type, default, docstring
    EnumReference(Option<String>, String, HasDefault<String>, Option<String>),
    /// Null type needed for representing null in unions
    Null,
}

impl Field {
    pub fn get_avro_type_name(&self) -> Option<String> {
        match self {
            Field::Protocol(..) => Some("protocol".to_string()),
            Field::Int(..) => Some("int".to_string()),
            Field::Long(..) => Some("long".to_string()),
            Field::Float(..) => Some("float".to_string()),
            Field::Double(..) => Some("double".to_string()),
            Field::Boolean(..) => Some("boolean".to_string()),
            Field::String(..) => Some("string".to_string()),
            Field::Record(..) => Some("record".to_string()),
            Field::Enum(..) => Some("enum".to_string()),
            Field::Union(..) => Some("union".to_string()),
            Field::Array(..) => Some("array".to_string()),
            Field::RecordReference(_, type_, ..) => Some(type_.clone()),
            Field::EnumReference(_, type_, ..) => Some(type_.clone()),
            Field::Null => Some("null".to_string()),
        }
    }
}
