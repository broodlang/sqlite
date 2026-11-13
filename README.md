# sqlite

[SQLite](https://sqlite.org) for [Brood](https://broodlang.org), as a sandboxed WebAssembly
component. No native code on the host: SQLite's C is compiled to `wasm32-wasip2`, committed as
`native/sqlite.wasm`, and run by the runtime's wasmtime host, which shows it the database's
directory and nothing else (ADR-453). It is also a database for
[store](https://github.com/broodlang/store) (`sqlite/store`, below). Needs Brood 0.51 or later.

## Usage

```brood
(def db (sqlite/open ":memory:"))        ; or a file: (sqlite/open "app.db")

(sqlite/execute-batch db "create table users (id integer primary key, name text unique)")
(sqlite/execute db "insert into users (name) values (?)" "ada")      ;=> 1
(sqlite/last-insert-rowid db)                                       ;=> 1
(sqlite/query db "select * from users where id = ?" 1)              ;=> [{:id 1 :name "ada"}]
(sqlite/query-one db "select count(*) as n from users")             ;=> {:n 1}
(sqlite/query-rows db "select id, name from users")                 ;=> {:columns ["id" "name"] :rows [[1 "ada"]]}

(sqlite/transaction db
  (fn () (sqlite/execute db "update users set name = ?" "grace")))  ; commits, or rolls back and re-raises

(sqlite/exclusive db
  (fn () (sqlite/query-one db "select count(*) as n from users")))  ;=> {:n 1}

(sqlite/close db)
```

Values map both ways: NULL is nil, INTEGER an int, REAL a float, TEXT a string, BLOB `bytes`. A
boolean binds as 1 or 0 and a keyword as its name. A bad statement raises with SQLite's message
(`sqlite: UNIQUE constraint failed: users.name`); the connection carries on.

## What a handle is

A process, holding the component and one SQLite connection, answering calls one at a time. Any
process that has the handle can use it, statements never interleave inside SQLite, and disk work
runs on the offload pool, so a slow write blocks its caller rather than a scheduler worker.
`close` (or the process exiting) frees the component.

**One handle per database file.** WASI has no file locks, so SQLite runs with its `unix-none`
VFS: two handles writing the same file at once can corrupt it. Share the handle instead — that
is what it is for. WAL is compiled out for the same reason.

**A run of statements holds the handle.** `transaction` and `exclusive` hand the connection to
one process until they return: a statement another process sends meanwhile waits rather than
landing in the middle. A process that dies holding it has whatever it began rolled back before
the next one gets it.

## With store

`sqlite/store/adaptor` is [store](https://github.com/broodlang/store)'s adaptor contract over a
handle, so a repo runs on SQLite: query-as-data, inserts and updates with RETURNING,
changesets, preloads, versioned migrations, and transactions with savepoints. Depend on both
packages:

```brood fragment
(def repo (repo/open (sqlite/store/adaptor) {:database "app.db"}))   ; ":memory:" when absent

(repo/all repo (-> (query/table :users) (query/where [:= :name "ada"])))
(repo/transaction repo (fn (tx) (repo/insert tx :users {:name "grace"})))
```

store's `$1` placeholders are bound as SQLite's `?1`, never spliced into the SQL. A unique,
foreign-key, check or not-null failure raises store's error map with its `:violation`; for a
unique one `:constraint` names the index that refused it, so
`(changeset/unique-constraint cs :email {:name "users_email_idx"})` turns it into "has already
been taken" on `:email`. There is no `:lock-sql`: one connection holds the database, so store
takes no advisory lock.

## Development

`nest test` runs the suite (store comes in as a dev dependency for `tests/store_test.blsp`).
`make wasm` rebuilds `native/sqlite.wasm` from `native/` (the `rusqlite` crate's bundled SQLite,
wrapped in a `connection` resource — `native/wit/sqlite.wit`); it needs the `wasm32-wasip2` Rust
target and wasi-sdk.
