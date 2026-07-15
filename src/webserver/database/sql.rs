use std::fmt::Write as _;
use std::path::Path;

use async_trait::async_trait;
use sqlparser::ast::helpers::attached_token::AttachedToken;
use sqlparser::ast::{
    Expr, Ident, ObjectName, ObjectNamePart, SelectFlavor, SelectItem, Set, SetExpr, Spanned,
    Statement, Value,
};
use sqlparser::dialect::Dialect;
use sqlparser::parser::{Parser, ParserError};
use sqlparser::tokenizer::Token::{self, EOF, SemiColon};
use sqlparser::tokenizer::{Location, Span, TokenWithSpan, Tokenizer};

use super::csv_import::extract_csv_copy_statement;
use super::{Database, DbInfo, SupportedDatabase};
use crate::AppState;
use crate::file_cache::AsyncFromStrWithState;
use crate::webserver::database::error_highlighting::quote_source_with_highlight;

mod dialect;
mod rewrite;
mod statement;

#[cfg(test)]
pub(super) use statement::SourceLocation;
pub use statement::SqlFile;
pub(super) use statement::{
    DatabaseQuery, FileStatement, OutputColumn, Query, QueryBody, SingleRowQuery, SourceSpan,
    VariableName,
};

impl SqlFile {
    #[must_use]
    pub fn new(db: &Database, sql: &str, source_path: &Path) -> Self {
        let dialect = dialect::parser_dialect(db.info.database_type);
        log::debug!(
            "Parsing SQL file {} using dialect {:?}",
            source_path.display(),
            dialect
        );
        let statements = match parse_sql(&db.info, dialect.as_ref(), sql) {
            Ok(statements) => statements.collect::<Vec<_>>().into_boxed_slice(),
            Err(error) => {
                return Self::from_error(error, source_path);
            }
        };
        Self {
            statements,
            source_path: source_path.to_path_buf(),
        }
    }

    fn from_error(error: impl Into<anyhow::Error>, source_path: &Path) -> Self {
        Self {
            statements: vec![FileStatement::Error(
                error
                    .into()
                    .context(format!("Error parsing file {}", source_path.display())),
            )]
            .into_boxed_slice(),
            source_path: source_path.to_path_buf(),
        }
    }
}

#[async_trait(?Send)]
impl AsyncFromStrWithState for SqlFile {
    async fn from_str_with_state(
        app_state: &AppState,
        source: &str,
        source_path: &Path,
    ) -> anyhow::Result<Self> {
        Ok(Self::new(&app_state.db, source, source_path))
    }
}

fn parse_sql<'a>(
    database: &'a DbInfo,
    dialect: &'a dyn Dialect,
    sql: &'a str,
) -> anyhow::Result<impl Iterator<Item = FileStatement> + 'a> {
    log::trace!("Parsing {} SQL: {sql}", database.dbms_name);
    let tokens = Tokenizer::new(dialect, sql)
        .tokenize_with_location()
        .map_err(|error| {
            let location = error.location;
            anyhow::Error::new(error).context(format!(
                "The SQLPage parser could not understand the SQL file. Tokenization failed. Please check for syntax errors:\n{}",
                quote_source_with_highlight(sql, location.line, location.column)
            ))
        })?;
    let mut parser = Parser::new(dialect).with_tokens_with_locations(tokens);
    let mut has_error = false;
    Ok(std::iter::from_fn(move || {
        if has_error {
            return None;
        }
        let statement = parse_single_statement(&mut parser, database, sql);
        if matches!(statement, Some(FileStatement::Error(_))) {
            has_error = true;
        }
        statement
    }))
}

fn parse_single_statement(
    parser: &mut Parser<'_>,
    database: &DbInfo,
    source_sql: &str,
) -> Option<FileStatement> {
    if parser.peek_token() == EOF {
        return None;
    }
    let mut statement = match parser.parse_statement() {
        Ok(statement) => statement,
        Err(error) => return Some(syntax_error(error, parser, source_sql)),
    };
    let mut semicolon = false;
    while parser.consume_token(&SemiColon) {
        semicolon = true;
    }

    if let Some(statement) = extract_set_variable(&mut statement, database) {
        return Some(statement);
    }
    if let Some(csv_import) = extract_csv_copy_statement(&mut statement) {
        return Some(FileStatement::CsvImport(csv_import));
    }

    Some(
        match rewrite::rewrite_query(statement, database, semicolon) {
            Ok(query) => FileStatement::Query(query),
            Err(error) => FileStatement::Error(error),
        },
    )
}

fn extract_set_variable(statement: &mut Statement, database: &DbInfo) -> Option<FileStatement> {
    let Statement::Set(Set::SingleAssignment {
        variable: ObjectName(name),
        values,
        scope: None,
        hivevar: false,
    }) = statement
    else {
        return None;
    };
    let ([ObjectNamePart::Identifier(identifier)], [value]) =
        (name.as_mut_slice(), values.as_mut_slice())
    else {
        return None;
    };

    let mut target = std::mem::take(&mut identifier.value);
    if target.starts_with(['$', ':', '?']) {
        target.remove(0);
    }
    let expression = std::mem::replace(value, Expr::value(Value::Null));
    let value_statement = expression_to_query(expression);
    Some(
        match rewrite::rewrite_query(value_statement, database, false) {
            Ok(value) => FileStatement::SetVariable {
                target: VariableName(target),
                value,
            },
            Err(error) => FileStatement::Error(error),
        },
    )
}

fn syntax_error(error: ParserError, parser: &Parser, sql: &str) -> FileStatement {
    let Span {
        start: Location {
            line: start_line,
            column: start_column,
        },
        end: Location { line: end_line, .. },
    } = parser.peek_token_no_skip().span;
    let mut message = String::from(
        "Parsing failed: SQLPage couldn't understand the SQL file. Please check for syntax errors on ",
    );
    if start_line == end_line {
        write!(&mut message, "line {start_line}:").unwrap();
    } else {
        write!(&mut message, "lines {start_line} to {end_line}:").unwrap();
    }
    write!(
        &mut message,
        "\n{}",
        quote_source_with_highlight(sql, start_line, start_column)
    )
    .unwrap();
    FileStatement::Error(anyhow::Error::from(error).context(message))
}

const SQLPAGE_FUNCTION_NAMESPACE: &str = "sqlpage";

pub(super) fn is_sqlpage_func(parts: &[ObjectNamePart]) -> bool {
    matches!(
        parts,
        [
            ObjectNamePart::Identifier(Ident {
                value,
                quote_style: None,
                ..
            }),
            ObjectNamePart::Identifier(Ident { quote_style: None, .. })
        ] if value.eq_ignore_ascii_case(SQLPAGE_FUNCTION_NAMESPACE)
    )
}

pub(super) fn extract_json_columns(
    statement: &Statement,
    database: SupportedDatabase,
) -> Vec<String> {
    if matches!(
        database,
        SupportedDatabase::Postgres | SupportedDatabase::Mssql
    ) {
        return Vec::new();
    }
    let Statement::Query(query) = statement else {
        return Vec::new();
    };
    let SetExpr::Select(select) = query.body.as_ref() else {
        return Vec::new();
    };
    select
        .projection
        .iter()
        .filter_map(|item| match item {
            SelectItem::ExprWithAlias { expr, alias } if is_json_expression(expr) => {
                Some(alias.value.clone())
            }
            _ => None,
        })
        .collect()
}

pub(super) fn is_json_expression(expression: &Expr) -> bool {
    match expression {
        Expr::Function(function) => {
            let [ObjectNamePart::Identifier(name)] = function.name.0.as_slice() else {
                return false;
            };
            [
                "json_object",
                "json_array",
                "json_build_object",
                "json_build_array",
                "to_json",
                "to_jsonb",
                "json_agg",
                "jsonb_agg",
                "json_arrayagg",
                "json_objectagg",
                "json_group_array",
                "json_group_object",
                "json",
                "jsonb",
            ]
            .iter()
            .any(|candidate| name.value.eq_ignore_ascii_case(candidate))
        }
        Expr::Cast { data_type, .. } => matches!(
            data_type,
            sqlparser::ast::DataType::JSON | sqlparser::ast::DataType::JSONB
        ),
        _ => false,
    }
}

fn expression_to_query(expression: Expr) -> Statement {
    if let Expr::Subquery(query) = expression {
        return Statement::Query(query);
    }
    Statement::Query(Box::new(sqlparser::ast::Query {
        with: None,
        body: Box::new(SetExpr::Select(Box::new(sqlparser::ast::Select {
            select_token: AttachedToken(TokenWithSpan::new(
                Token::make_keyword("SELECT"),
                expression.span(),
            )),
            distinct: None,
            top: None,
            projection: vec![SelectItem::ExprWithAlias {
                expr: expression,
                alias: Ident::new("sqlpage_set_expr"),
            }],
            into: None,
            from: vec![],
            lateral_views: vec![],
            selection: None,
            group_by: sqlparser::ast::GroupByExpr::Expressions(vec![], vec![]),
            cluster_by: vec![],
            distribute_by: vec![],
            sort_by: vec![],
            having: None,
            named_window: vec![],
            qualify: None,
            top_before_distinct: false,
            prewhere: None,
            window_before_qualify: false,
            value_table_mode: None,
            connect_by: Vec::new(),
            optimizer_hints: vec![],
            select_modifiers: None,
            flavor: SelectFlavor::Standard,
            exclude: None,
        }))),
        order_by: None,
        limit_clause: None,
        fetch: None,
        locks: vec![],
        for_clause: None,
        settings: None,
        format_clause: None,
        pipe_operators: Vec::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect};
    use sqlx::any::AnyKind;

    fn database(database_type: SupportedDatabase) -> DbInfo {
        let kind = match database_type {
            SupportedDatabase::Postgres => AnyKind::Postgres,
            SupportedDatabase::Mssql => AnyKind::Mssql,
            SupportedDatabase::MySql => AnyKind::MySql,
            SupportedDatabase::Sqlite => AnyKind::Sqlite,
            _ => AnyKind::Odbc,
        };
        DbInfo {
            dbms_name: database_type.display_name().to_owned(),
            database_type,
            kind,
        }
    }

    fn one(sql: &str) -> FileStatement {
        parse_sql(
            &database(SupportedDatabase::Postgres),
            &PostgreSqlDialect {},
            sql,
        )
        .unwrap()
        .next()
        .unwrap()
    }

    #[test]
    fn database_only_parent_forces_binding() {
        let FileStatement::Query(Query {
            body: QueryBody::Database(query),
            ..
        }) = one("select upper(sqlpage.url_encode('x'))")
        else {
            panic!("expected database query");
        };
        assert_eq!(query.bindings.len(), 1);
        assert!(query.computed_columns.is_empty());
        assert!(query.sql.contains("upper(CAST($1 AS TEXT))"));
    }

    #[test]
    fn emulated_parent_keeps_nested_call_per_row() {
        let FileStatement::Query(Query {
            body: QueryBody::Database(query),
            ..
        }) = one("select coalesce(sqlpage.url_encode(value), '') as encoded from t")
        else {
            panic!("expected database query");
        };
        assert!(query.bindings.is_empty());
        assert_eq!(query.row_input_json.len(), 1);
        assert_eq!(query.computed_columns.len(), 1);
        assert!(!query.sql.contains("sqlpage."));
    }

    #[test]
    fn row_value_cannot_cross_database_only_parent() {
        let FileStatement::Error(error) = one("select upper(sqlpage.url_encode(value)) from t")
        else {
            panic!("expected rewrite error");
        };
        assert!(format!("{error:#}").contains("required before the query"));
    }

    #[test]
    fn positional_bindings_follow_source_order() {
        let database = database(SupportedDatabase::MySql);
        let mut statements = parse_sql(
            &database,
            &MySqlDialect {},
            "select $a, upper(sqlpage.url_encode($b))",
        )
        .unwrap();
        let FileStatement::Query(Query {
            body: QueryBody::Database(query),
            ..
        }) = statements.next().unwrap()
        else {
            panic!("expected database query");
        };
        assert_eq!(query.bindings.len(), 2);
        assert_eq!(query.sql.matches('?').count(), 2);
    }

    #[test]
    fn database_cannot_order_by_computed_column() {
        let FileStatement::Error(error) =
            one("select sqlpage.url_encode(value) as encoded from t order by encoded")
        else {
            panic!("expected rewrite error");
        };
        assert!(error.to_string().contains("ORDER BY"));
    }

    #[test]
    fn placeholder_like_literal_is_not_rewritten() {
        let database = database(SupportedDatabase::MySql);
        let statement = parse_sql(
            &database,
            &MySqlDialect {},
            "select '@SQLPAGE_TEMP1' as value from t where id = $id",
        )
        .unwrap()
        .next()
        .unwrap();
        let FileStatement::Query(Query {
            body: QueryBody::Database(query),
            ..
        }) = statement
        else {
            panic!("expected database query");
        };
        assert!(query.sql.contains("'@SQLPAGE_TEMP1'"));
        assert_eq!(query.bindings.len(), 1);
    }

    #[test]
    fn effectful_bindings_are_not_deduplicated() {
        let FileStatement::Query(Query {
            body: QueryBody::Database(query),
            ..
        }) = one("select 1 as value where sqlpage.random_string(1) <> sqlpage.random_string(1)")
        else {
            panic!("expected database query");
        };
        assert_eq!(query.bindings.len(), 2);
    }

    #[test]
    fn nested_run_sql_requires_buffering() {
        let FileStatement::Query(Query {
            body: QueryBody::Database(query),
            ..
        }) = one("select coalesce(sqlpage.run_sql(path), '') from files")
        else {
            panic!("expected database query");
        };
        assert!(query.must_buffer_rows());
    }

    #[test]
    fn standalone_projection_has_no_database_query() {
        let FileStatement::Query(Query {
            body: QueryBody::SingleRow(query),
            ..
        }) = one("select sqlpage.url_encode('a b') as value")
        else {
            panic!("expected a single SQLPage-owned row");
        };
        assert_eq!(query.columns.len(), 1);
    }

    #[test]
    fn unquoted_sqlpage_names_are_case_insensitive() {
        let FileStatement::Query(Query {
            body: QueryBody::SingleRow(query),
            ..
        }) = one("select SQLPAGE.URL_ENCODE('a b') as value")
        else {
            panic!("expected a single SQLPage-owned row");
        };
        assert_eq!(query.columns.len(), 1);
    }
}
