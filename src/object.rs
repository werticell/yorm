#![forbid(unsafe_code)]

use crate::{data::DataType, storage::Row};
use std::any::Any;

////////////////////////////////////////////////////////////////////////////////

pub trait Object: Any + Sized {
    fn as_row(&self) -> Row;
    fn from_row(row: Row) -> Self;

    fn table_name() -> &'static str;
    fn type_name() -> &'static str;

    // Field name, Column name, Type
    fn field_names() -> Vec<&'static str>;
    fn column_names() -> Vec<&'static str>;
    fn column_types() -> Vec<DataType>;

    fn describe() -> Schema {
        Schema {
            table_name: Self::table_name(),
            field_names: Self::field_names(),
            column_names: Self::column_names(),
            column_types: Self::column_types(),
            type_name: Self::type_name(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait Store: Any {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any;
    fn as_row(&self) -> Row;
    fn describe(&self) -> Schema;
}

impl<T: Object> Store for T {
    fn as_any(&self) -> &dyn Any {
        self.as_any()
    }
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self.as_mut_any()
    }

    fn as_row(&self) -> Row {
        self.as_row()
    }
    fn describe(&self) -> Schema {
        Self::describe()
    }
}

////////////////////////////////////////////////////////////////////////////////

// TODO: maybe we could build the whole schema in Object trait
pub struct Schema {
    table_name: &'static str,
    field_names: Vec<&'static str>,
    column_names: Vec<&'static str>,
    column_types: Vec<DataType>,
    type_name: &'static str,
}

impl Schema {
    pub fn get_table_name(&self) -> &'static str {
        self.table_name
    }

    pub fn get_type_name(&self) -> &'static str {
        self.type_name
    }

    pub fn get_types(&self) -> &[DataType] {
        self.column_types.as_slice()
    }

    pub fn column_name_list(&self, separator: &str) -> String {
        self.column_names.join(separator)
    }

    pub fn prepare_update_column_list(&self) -> String {
        let mut result = String::new();
        for col_name in &self.column_names {
            result.push_str(col_name);
            result.push_str(" = ?,");
        }
        if !result.is_empty() {
            result.pop();
        }
        result
    }

    pub fn get_same_column_name(&self, str: &str) -> Option<(&'static str, usize)> {
        for i in 0..self.column_names.len() {
            if self.column_names[i].contains(str) {
                return Some((self.column_names[i], i));
            }
        }
        None
    }

    pub fn get_nth_column_name(&self, n: usize) -> &'static str {
        self.column_names[n]
    }

    pub fn get_nth_field_name(&self, n: usize) -> &'static str {
        self.field_names[n]
    }

    pub fn column_fields_name(&self, separator: &str) -> String {
        self.column_names.join(separator)
    }

    pub fn columns_count(&self) -> usize {
        self.column_types.len()
    }

    pub fn text_description(&self) -> String {
        let mut result = "id INTEGER PRIMARY KEY AUTOINCREMENT,".to_owned();
        for (col_name, col_type) in self.column_names.iter().zip(self.column_types.iter()) {
            result.push_str(col_name);
            result.push(' ');
            result.push_str((*col_type).into());
            result.push(',');
        }
        result.pop();
        result
    }
}
