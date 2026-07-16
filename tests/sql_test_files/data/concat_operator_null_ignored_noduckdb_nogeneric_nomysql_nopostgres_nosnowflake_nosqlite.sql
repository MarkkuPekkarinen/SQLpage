select '/' as expected, '/' || null as actual;

select '%2F' as expected, sqlpage.url_encode('/' || nullable_col) as actual
from (select cast(null as varchar(1)) as nullable_col) input_rows;
