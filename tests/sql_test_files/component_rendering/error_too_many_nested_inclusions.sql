select 'debug' as component,
    coalesce(sqlpage.run_sql(
        'tests/sql_test_files/component_rendering/error_too_many_nested_inclusions.sql'
    ), '[]') as contents;
