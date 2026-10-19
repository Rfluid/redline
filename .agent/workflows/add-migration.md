# Workflow: database schema change

1. **Generate the migration:** `diesel migration generate <snake_name>`. Needs `diesel_cli` with `--features sqlite-bundled`.
2. **Write the SQL:**
   - `up.sql` and `down.sql`; `down` must undo `up` exactly.
   - **Never edit a migration that has shipped.** Add a new one.
3. **Check it on a scratch DB, never the real one:**
   ```sh
   diesel migration run --database-url /tmp/rl-scratch.db
   diesel migration redo --database-url /tmp/rl-scratch.db
   diesel print-schema --database-url /tmp/rl-scratch.db > src/store/schema.rs
   ```
4. **Fix the schema:** in `src/store/schema.rs`, set the `seq` columns back to `BigInt` (print-schema emits `Integer`).
5. **Update the code:** the row structs in `src/store/rows.rs`, then the `Store` methods, then the wire types in `src/model.rs` if the API changes.
6. **Test:** store unit tests. Migrations run automatically in `Store::open(":memory:")`.
7. **Docs:** the data model in `docs/ARCHITECTURE.md`, and `docs/PROTOCOL.md` if the wire types changed.
