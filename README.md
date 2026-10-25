# sqlite

[SQLite](https://sqlite.org) for [Brood](https://broodlang.org), as a sandboxed WebAssembly
component. No native code on the host: SQLite's C is compiled to `wasm32-wasip2`, committed as
`native/sqlite.wasm`, and run by the runtime's wasmtime host, which shows it the database's
directory and nothing else (ADR-453). Needs Brood 0.44 or later.

## Usage

```brood
(def db (sqlite/open "app.db"))          ; or ":memory:"

(sqlite/execute-batch db "create table users (id integer primary key, name text unique)")
(sqlite/execute db "insert into users (name) values (?)" "ada")      ; => 1
(sqlite/last-insert-rowid db)                                       ; => 1
(sqlite/query db "select * from users where id = ?" 1)              ; => [{:id 1 :name "ada"}]
(sqlite/query-one db "select count(*) as n from users")             ; => {:n 1}
(sqlite/query-rows db "select id, name from users")                 ; => {:columns ["id" "name"] :rows [[1 "ada"]]}

(sqlite/transaction db
  (fn () (sqlite/execute db "update users set name = ?" "grace")))  ; commits, or rolls back and re-raises

(sqlite/close db)
```

Values map both ways: NULL is nil, INTEGER an int, REAL a float, TEXT a string, BLOB `bytes`. A
boolean binds as 1 or 0 and a keyword as its name. A bad statement raises with SQLite's message;
the connection carries on.

## What a handle is

A process, holding the component and one SQLite connection, answering calls one at a time. Any
process that has the handle can use it, statements never interleave inside SQLite, and disk work
runs on the offload pool, so a slow write blocks its caller rather than a scheduler worker.
`close` (or the process exiting) frees the component.

**One handle per database file.** WASI has no file locks, so SQLite runs with its `unix-none`
VFS: two handles writing the same file at once can corrupt it. Share the handle instead — that
is what it is for. A transaction on a shared handle also takes in whatever another process sends
the handle while it runs; give a transaction its own handle when that matters. WAL is compiled
out for the same reason.

## Development

`nest test` runs the suite. `make wasm` rebuilds `native/sqlite.wasm` from `native/` (the
`rusqlite` crate's bundled SQLite, wrapped in a `connection` resource — `native/wit/sqlite.wit`);
it needs the `wasm32-wasip2` Rust target and wasi-sdk.
