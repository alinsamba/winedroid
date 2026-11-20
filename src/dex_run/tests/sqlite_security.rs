use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::types::Value;

use super::super::*;

static SQLITE_SECURITY_TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

struct SqliteSecurityTestDirectory {
    path: PathBuf,
}

impl SqliteSecurityTestDirectory {
    fn new(name: &str) -> Self {
        let nonce = SQLITE_SECURITY_TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "winedroid-sqlite-security-{}-{nonce}-{name}",
            std::process::id()
        ));
        fs::remove_dir_all(&path).ok();
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SqliteSecurityTestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).ok();
    }
}

#[test]
fn sqlite_authorizer_blocks_host_filename_operations_and_preserves_plain_vacuum() {
    let directory = SqliteSecurityTestDirectory::new("filename-operations");
    let program = DexProgram::new(Vec::new());
    let database = DexValue::Reference(std::rc::Rc::from("database"));
    let database_path = directory.path().join("main.db");
    program
        .register_sqlite_database_path(&database, &database_path)
        .unwrap();

    assert_eq!(
        program
            .sqlite_execute_database_sql(&database, "CREATE TABLE records (value INTEGER)", &[],),
        Some(())
    );
    assert_eq!(
        program.sqlite_execute_database_sql(&database, "INSERT INTO records VALUES (1)", &[],),
        Some(())
    );
    assert_eq!(
        program.sqlite_execute_database_sql(&database, "VACUUM", &[]),
        Some(())
    );

    let literal_attachment = directory.path().join("literal.db");
    let literal_sql = format!(
        "ATTACH DATABASE {} AS literal",
        sqlite_test_path_literal(&literal_attachment)
    );
    assert_eq!(
        program.sqlite_execute_database_sql(&database, &literal_sql, &[]),
        None
    );

    let expression_attachment = directory.path().join("expression.db");
    let expression_stem = directory.path().join("expression");
    let expression_sql = format!(
        "ATTACH DATABASE ({} || '.db') AS expression",
        sqlite_test_path_literal(&expression_stem)
    );
    assert_eq!(
        program.sqlite_execute_database_sql(&database, &expression_sql, &[]),
        None
    );

    let bound_attachment = directory.path().join("bound.db");
    assert_eq!(
        program.sqlite_execute_database_sql(
            &database,
            "ATTACH DATABASE ? AS bound",
            &[Value::Text(bound_attachment.to_string_lossy().into_owned())],
        ),
        None
    );

    let uri_attachment = directory.path().join("uri.db");
    let uri = format!(
        "file:{}?mode=rwc&cache=shared",
        uri_attachment.to_string_lossy()
    );
    let uri_sql = format!(
        "ATTACH DATABASE {} AS uri",
        sqlite_test_string_literal(&uri)
    );
    assert_eq!(
        program.sqlite_execute_database_sql(&database, &uri_sql, &[]),
        None
    );

    let vacuum_output = directory.path().join("vacuum.db");
    let vacuum_sql = format!("VACUUM INTO {}", sqlite_test_path_literal(&vacuum_output));
    assert_eq!(
        program.sqlite_execute_database_sql(&database, &vacuum_sql, &[]),
        None
    );

    let bound_vacuum_output = directory.path().join("bound-vacuum.db");
    assert_eq!(
        program.sqlite_execute_database_sql(
            &database,
            "VACUUM INTO ?",
            &[Value::Text(
                bound_vacuum_output.to_string_lossy().into_owned(),
            )],
        ),
        None
    );

    let query_vacuum_output = directory.path().join("query-vacuum.db");
    let query_vacuum_sql = format!(
        "VACUUM INTO {}",
        sqlite_test_path_literal(&query_vacuum_output)
    );
    assert!(program
        .sqlite_raw_query_result(&database, &query_vacuum_sql, &[])
        .is_none());

    for path in [
        literal_attachment,
        expression_attachment,
        bound_attachment,
        uri_attachment,
        vacuum_output,
        bound_vacuum_output,
        query_vacuum_output,
    ] {
        assert!(!path.exists(), "SQLite created `{}`", path.display());
    }

    let result = program
        .sqlite_raw_query_result(&database, "SELECT COUNT(*) FROM records", &[])
        .unwrap();
    assert_eq!(result.rows, vec![vec![DexValue::Long(1)]]);
}

#[test]
fn sqlite_authorizer_blocks_host_path_pragmas_and_preserves_schema_pragmas() {
    let directory = SqliteSecurityTestDirectory::new("path-pragmas");
    let program = DexProgram::new(Vec::new());
    let database = DexValue::Reference(std::rc::Rc::from("database"));
    let database_path = directory.path().join("main.db");
    program
        .register_sqlite_database_path(&database, &database_path)
        .unwrap();
    assert_eq!(
        program
            .sqlite_execute_database_sql(&database, "CREATE TABLE records (value INTEGER)", &[],),
        Some(())
    );

    assert!(program
        .sqlite_raw_query_result(&database, "PrAgMa DaTaBaSe_LiSt", &[])
        .is_none());
    assert!(program
        .sqlite_raw_query_result(&database, "SELECT file FROM pragma_database_list", &[])
        .is_none());
    assert!(program
        .sqlite_raw_query_result(&database, "PRAGMA temp_store_directory", &[])
        .is_none());

    let directory_pragma = format!(
        "PRAGMA temp_store_directory = {}",
        sqlite_test_path_literal(directory.path())
    );
    assert_eq!(
        program.sqlite_execute_database_sql(&database, &directory_pragma, &[]),
        None
    );

    let table_info = program
        .sqlite_raw_query_result(&database, "PRAGMA table_info(records)", &[])
        .unwrap();
    assert_eq!(table_info.rows.len(), 1);
    assert_eq!(
        table_info.rows[0][1],
        DexValue::String(std::rc::Rc::from("value"))
    );
}

// cite: frameworks-base SQLiteDatabase.java#execSQL
// cite: frameworks-base SQLiteConnection.java#acquirePreparedStatement
#[test]
fn sqlite_exec_sql_ignores_statements_after_the_first_android_statement() {
    let directory = SqliteSecurityTestDirectory::new("single-statement");
    let program = DexProgram::new(Vec::new());
    let database = DexValue::Reference(std::rc::Rc::from("database"));
    program
        .register_sqlite_database_path(&database, &directory.path().join("main.db"))
        .unwrap();
    assert_eq!(
        program
            .sqlite_execute_database_sql(&database, "CREATE TABLE records (value INTEGER)", &[],),
        Some(())
    );
    assert_eq!(
        program.sqlite_execute_database_sql(
            &database,
            "INSERT INTO records VALUES (1); INSERT INTO records VALUES (2)",
            &[],
        ),
        Some(())
    );

    let result = program
        .sqlite_raw_query_result(&database, "SELECT value FROM records", &[])
        .unwrap();
    assert_eq!(result.rows, vec![vec![DexValue::Long(1)]]);
}

fn sqlite_test_path_literal(path: &Path) -> String {
    sqlite_test_string_literal(&path.to_string_lossy())
}

fn sqlite_test_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
