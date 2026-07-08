select 'dynamic' as component,
    coalesce(sqlpage.run_sql('tests/uploads/upload_file_test.sql'), '[]') as properties;
