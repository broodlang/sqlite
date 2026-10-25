//! SQLite as a component: one `connection` resource over `rusqlite`.
//!
//! WASI preview 2 has no advisory file locks, so the database is opened through SQLite's
//! `unix-none` VFS: no locking at all. One connection per database file is safe; two
//! instances writing the same file at once are not, and the Brood side keeps a connection
//! inside the process that opened it.

use std::cell::RefCell;

use rusqlite::types::{Value, ValueRef};
use rusqlite::{Connection as Sqlite, OpenFlags, params_from_iter};

wit_bindgen::generate!({ world: "sqlite", path: "wit" });

use exports::brood::sqlite::db::{Cell, Guest, GuestConnection, Rows};

struct Component;

struct Connection {
    inner: RefCell<Sqlite>,
}

fn to_value(cell: Cell) -> Value {
    match cell {
        Cell::Null => Value::Null,
        Cell::Integer(n) => Value::Integer(n),
        Cell::Real(x) => Value::Real(x),
        Cell::Text(s) => Value::Text(s),
        Cell::Blob(b) => Value::Blob(b),
    }
}

fn to_cell(value: ValueRef<'_>) -> Cell {
    match value {
        ValueRef::Null => Cell::Null,
        ValueRef::Integer(n) => Cell::Integer(n),
        ValueRef::Real(x) => Cell::Real(x),
        ValueRef::Text(t) => Cell::Text(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Cell::Blob(b.to_vec()),
    }
}

fn message(error: rusqlite::Error) -> String {
    error.to_string()
}

impl GuestConnection for Connection {
    fn open(path: String) -> Result<exports::brood::sqlite::db::Connection, String> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let connection = if path == ":memory:" {
            Sqlite::open_in_memory()
        } else {
            Sqlite::open_with_flags_and_vfs(&path, flags, c"unix-none")
        }
        .map_err(message)?;
        Ok(exports::brood::sqlite::db::Connection::new(Connection {
            inner: RefCell::new(connection),
        }))
    }

    fn execute(&self, sql: String, params: Vec<Cell>) -> Result<u64, String> {
        let connection = self.inner.borrow();
        let changed = connection
            .execute(&sql, params_from_iter(params.into_iter().map(to_value)))
            .map_err(message)?;
        Ok(changed as u64)
    }

    fn execute_batch(&self, sql: String) -> Result<(), String> {
        self.inner.borrow().execute_batch(&sql).map_err(message)
    }

    fn query(&self, sql: String, params: Vec<Cell>) -> Result<Rows, String> {
        let connection = self.inner.borrow();
        let mut statement = connection.prepare(&sql).map_err(message)?;
        let columns: Vec<String> = statement
            .column_names()
            .into_iter()
            .map(str::to_string)
            .collect();
        let width = columns.len();
        let mut cursor = statement
            .query(params_from_iter(params.into_iter().map(to_value)))
            .map_err(message)?;
        let mut rows = Vec::new();
        while let Some(row) = cursor.next().map_err(message)? {
            let mut cells = Vec::with_capacity(width);
            for index in 0..width {
                cells.push(to_cell(row.get_ref(index).map_err(message)?));
            }
            rows.push(cells);
        }
        Ok(Rows { columns, rows })
    }

    fn last_insert_rowid(&self) -> i64 {
        self.inner.borrow().last_insert_rowid()
    }
}

impl Guest for Component {
    type Connection = Connection;

    fn version() -> String {
        rusqlite::version().to_string()
    }
}

export!(Component);
