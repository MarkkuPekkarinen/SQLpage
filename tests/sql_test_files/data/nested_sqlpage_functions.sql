select 'It%20works%20%21' as expected, sqlpage.url_encode(sqlpage.read_file_as_text('tests/it_works.txt')) as actual;

select 'It%20works%20%21' as expected,
    sqlpage.url_encode(sqlpage.read_file_as_text(path)) as actual
from (select 'tests/it_works.txt' as path) paths;

select '%2Fvalue' as expected,
    coalesce(sqlpage.url_encode('/' || value), '') as actual
from (select 'value' as value) value_rows;
