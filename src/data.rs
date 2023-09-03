#![forbid(unsafe_code)]

use rusqlite::{types::ToSqlOutput, ToSql};
use std::{borrow::Cow, fmt};

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ObjectId(i64);

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ObjectId {
    pub fn new(n: i64) -> Self {
        Self(n)
    }
    pub fn into_i64(&self) -> i64 {
        self.0
    }
}

impl From<i64> for ObjectId {
    fn from(n: i64) -> Self {
        ObjectId::new(n)
    }
}

impl ToSql for ObjectId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}
////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    String,
    Bytes,
    Int64,
    Float64,
    Bool,
}

impl From<DataType> for &'static str {
    fn from(data_type: DataType) -> Self {
        match data_type {
            DataType::String => "TEXT",
            DataType::Bytes => "BLOB",
            DataType::Int64 => "BIGINT",
            DataType::Float64 => "REAL",
            DataType::Bool => "TINYINT",
        }
    }
}

impl From<&str> for DataType {
    fn from(string_type: &str) -> Self {
        match string_type {
            "String" => DataType::String,
            "Vec < u8 >" => DataType::Bytes,
            "i64" => DataType::Int64,
            "f64" => DataType::Float64,
            "bool" => DataType::Bool,
            t => panic!("Not supported type {}", t),
        }
    }
}

// impl fmt::Display for DataType {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self {
//             DataType::String => write!(f, "TEXT"),
//             DataType::Bytes => write!(f, "BLOB"),
//             DataType::Int64 => write!(f, "BIGINT"),
//             DataType::Float64 => write!(f, "REAL"),
//             DataType::Bool => write!(f, "TINYINT"),
//         }
//     }
// }

////////////////////////////////////////////////////////////////////////////////

pub enum Value<'a> {
    String(Cow<'a, str>),
    Bytes(Cow<'a, [u8]>),
    Int64(i64),
    Float64(f64),
    Bool(bool),
}

impl<'a> From<String> for Value<'a> {
    fn from(str: String) -> Self {
        Value::String(Cow::from(str))
    }
}

impl<'a> From<Vec<u8>> for Value<'a> {
    fn from(bytes: Vec<u8>) -> Self {
        Value::Bytes(Cow::from(bytes))
    }
}

impl<'a> From<i64> for Value<'a> {
    fn from(num: i64) -> Self {
        Value::Int64(num)
    }
}

impl<'a> From<bool> for Value<'a> {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl<'a> From<f64> for Value<'a> {
    fn from(num: f64) -> Self {
        Value::Float64(num)
    }
}

impl<'a> ToSql for Value<'a> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self {
            Value::String(cow) => cow.to_sql(),
            Value::Bytes(cow) => cow.to_sql(),
            Value::Int64(n) => n.to_sql(),
            Value::Float64(n) => n.to_sql(),
            Value::Bool(b) => b.to_sql(),
        }
    }
}

impl<'a> From<Value<'a>> for String {
    fn from(value: Value<'a>) -> Self {
        match value {
            Value::String(cow) => cow.into_owned(),
            _ => panic!("Wrong type extracted from Value"),
        }
    }
}

impl<'a> From<Value<'a>> for Vec<u8> {
    fn from(value: Value<'a>) -> Self {
        match value {
            Value::Bytes(cow) => cow.into_owned(),
            _ => panic!("Wrong type extracted from Value"),
        }
    }
}

impl<'a> From<Value<'a>> for i64 {
    fn from(value: Value<'a>) -> Self {
        match value {
            Value::Int64(num) => num,
            _ => panic!("Wrong type extracted from Value"),
        }
    }
}

impl<'a> From<Value<'a>> for bool {
    fn from(value: Value<'a>) -> Self {
        match value {
            Value::Bool(num) => num,
            _ => panic!("Wrong type extracted from Value"),
        }
    }
}

impl<'a> From<Value<'a>> for f64 {
    fn from(value: Value<'a>) -> Self {
        match value {
            Value::Float64(num) => num,
            _ => panic!("Wrong type extracted from Value"),
        }
    }
}
