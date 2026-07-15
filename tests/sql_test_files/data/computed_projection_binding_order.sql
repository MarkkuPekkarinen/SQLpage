set a = 'a';
set b = 'b';
set c = 'c';

select 'a' as "expected", $a as "actual",
    sqlpage.url_encode(upper(col || sqlpage.url_encode($b))) as "encoded",
    $c as "trailing"
from (select 'col' as col) values_table;

select 'COLB' as "expected", $a as "leading",
    sqlpage.url_encode(upper(col || sqlpage.url_encode($b))) as "actual",
    $c as "trailing"
from (select 'col' as col) values_table;

select 'c' as "expected", $a as "leading",
    sqlpage.url_encode(upper(col || sqlpage.url_encode($b))) as "encoded",
    $c as "actual"
from (select 'col' as col) values_table;
