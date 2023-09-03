#![forbid(unsafe_code)]

use crate::error::MissingColumnError;
use crate::{
    data::{DataType, Value},
    error::{Error, NotFoundError, Result, UnexpectedTypeError},
    object::Schema,
    ObjectId,
};
use rusqlite::types::FromSql;

////////////////////////////////////////////////////////////////////////////////

pub type Row<'a> = Vec<Value<'a>>;
pub type RowSlice<'a> = [Value<'a>];

////////////////////////////////////////////////////////////////////////////////

pub(crate) trait StorageTransaction {
    fn table_exists(&self, table: &str) -> Result<bool>;
    fn create_table(&self, schema: &Schema) -> Result<()>;

    fn insert_row(&self, schema: &Schema, row: &RowSlice) -> Result<ObjectId>;
    fn update_row(&self, id: ObjectId, schema: &Schema, row: &RowSlice) -> Result<()>;
    fn select_row(&self, id: ObjectId, schema: &Schema) -> Result<Row<'static>>;
    fn delete_row(&self, id: ObjectId, schema: &Schema) -> Result<()>;

    fn commit(&self) -> Result<()>;
    fn rollback(&self) -> Result<()>;
}

// rusqlite::Transaction.deref() == rusqlite::Connection
impl<'a> StorageTransaction for rusqlite::Transaction<'a> {
    fn table_exists(&self, table_name: &str) -> Result<bool> {
        let query = format!(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = \'{}\';",
            table_name
        );
        let mut stmt = self.prepare(&query)?;

        let mut rows = stmt.query([])?;
        Ok(rows.next().map_or(false, |o| o.is_some()))
    }

    fn create_table(&self, schema: &Schema) -> Result<()> {
        let query = format!(
            "CREATE TABLE {} ({});",
            schema.get_table_name(),
            schema.text_description()
        );
        let mut stmt = self.prepare(&query)?;
        if let Err(err) = stmt.execute([]) {
            return match err {
                rusqlite::Error::SqliteFailure(_, Some(str))
                    if str.contains("database is locked") =>
                {
                    Err(Error::LockConflict)
                }
                _ => Err(err.into()),
            };
        }
        Ok(())
    }

    fn insert_row(&self, schema: &Schema, row: &RowSlice) -> Result<ObjectId> {
        let query = if schema.columns_count() == 0 {
            format!("INSERT INTO {} (id) VALUES (NULL)", schema.get_table_name())
        } else {
            format!(
                "INSERT INTO {} ({}) VALUES ({});",
                schema.get_table_name(),
                schema.column_name_list(", "),
                repeat_questions(schema.columns_count()),
            )
        };

        let stmt = self.prepare(&query);
        if let Err(err) = stmt {
            return match err {
                rusqlite::Error::SqliteFailure(_, Some(str)) if has_missing_column_msg(&str) => {
                    Err(parse_missing_column(str, schema))
                }
                err => Err(err.into()),
            };
        }
        let id = stmt
            .unwrap()
            .insert(rusqlite::params_from_iter(row.iter()))?;
        Ok(ObjectId::new(id))
    }

    fn update_row(&self, id: ObjectId, schema: &Schema, row: &RowSlice) -> Result<()> {
        let query = format!(
            "UPDATE {} SET {} WHERE id = {}",
            schema.get_table_name(),
            schema.prepare_update_column_list(),
            id
        );
        let mut stmt = self.prepare(&query)?;
        stmt.execute(rusqlite::params_from_iter(row.iter()))?;
        Ok(())
    }

    fn select_row(&self, id: ObjectId, schema: &Schema) -> Result<Row<'static>> {
        let query = if schema.columns_count() == 0 {
            format!("SELECT * FROM {} WHERE id = ?", schema.get_table_name())
        } else {
            format!(
                "SELECT {} FROM {} WHERE id = ?;",
                schema.column_name_list(", "),
                schema.get_table_name()
            )
        };

        let stmt = self.prepare(&query);
        if let Err(err) = stmt {
            return match err {
                rusqlite::Error::SqliteFailure(_, Some(str)) if has_missing_column_msg(&str) => {
                    Err(parse_missing_column(str, schema))
                }
                err => Err(err.into()),
            };
        }

        let result_row = stmt
            .unwrap()
            .query_row([id], |row| Ok(parse_sqlite_row(schema, row)));
        match result_row {
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(Error::NotFound(Box::new(NotFoundError {
                    object_id: id,
                    type_name: schema.get_type_name(),
                })))
            }
            Ok(result) => result,
            _ => panic!("Not implemented from select row"),
        }
    }

    fn delete_row(&self, id: ObjectId, schema: &Schema) -> Result<()> {
        let query = format!("DELETE FROM {} WHERE id = ?", schema.get_table_name());
        self.execute(&query, [id])?;
        Ok(())
    }

    fn commit(&self) -> Result<()> {
        self.execute("COMMIT;", [])?;
        Ok(())
    }

    fn rollback(&self) -> Result<()> {
        self.execute("ROLLBACK;", [])?;
        Ok(())
    }
}

fn parse_sqlite_row(schema: &Schema, row: &rusqlite::Row) -> Result<Row<'static>> {
    let mut result = Vec::new();
    for (i, col_type) in schema.get_types().iter().enumerate() {
        let value = match col_type {
            DataType::Bool => get_value_from_row::<bool>(row, i, DataType::Bool, schema)?.into(),
            DataType::Float64 => {
                get_value_from_row::<f64>(row, i, DataType::Float64, schema)?.into()
            }
            DataType::Int64 => get_value_from_row::<i64>(row, i, DataType::Int64, schema)?.into(),
            DataType::Bytes => {
                get_value_from_row::<Vec<u8>>(row, i, DataType::Bytes, schema)?.into()
            }
            DataType::String => {
                get_value_from_row::<String>(row, i, DataType::String, schema)?.into()
            }
        };
        result.push(value)
    }
    Ok(result)
}

fn repeat_questions(count: usize) -> String {
    assert_ne!(count, 0);
    let mut string = "?,".repeat(count);
    string.pop();
    string
}

fn parse_missing_column(str: String, schema: &Schema) -> Error {
    let column_name = str.split(' ').last().unwrap();
    let (static_name_ref, i) = schema.get_same_column_name(column_name).expect("wrong");
    let static_attr_name = schema.get_nth_field_name(i);
    return Error::MissingColumn(Box::new(MissingColumnError {
        type_name: schema.get_type_name(),
        attr_name: static_attr_name,
        table_name: schema.get_table_name(),
        column_name: static_name_ref,
    }));
}

fn get_value_from_row<T: FromSql>(
    row: &rusqlite::Row,
    ind: usize,
    expected_type: DataType,
    schema: &Schema,
) -> Result<T> {
    let result = row.get::<_, T>(ind);
    match result {
        Err(rusqlite::Error::InvalidColumnType(_, _, c_type)) => {
            Err(Error::UnexpectedType(Box::new(UnexpectedTypeError {
                type_name: schema.get_type_name(),
                attr_name: schema.get_nth_field_name(ind),
                table_name: schema.get_table_name(),
                column_name: schema.get_nth_column_name(ind),
                expected_type,
                got_type: c_type.to_string(),
            })))
        }
        Ok(row) => Ok(row),
        _ => panic!("from get value from row"),
    }
}

fn has_missing_column_msg(str: &str) -> bool {
    str.contains("no such column:") || str.contains("has no column named")
}
