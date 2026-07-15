set x = (select sqlpage.fetch('invalid') where false);
select null as expected, $x as actual;

set x = (select sqlpage.url_encode(' ') where true);
select '%20' as expected, $x as actual;

set x = (select sqlpage.url_encode(null) where true);
select null as expected, $x as actual;

select '%20' as expected, sqlpage.url_encode(' ') as actual
from (select 1 as n union all select 2 as n) result_rows;
