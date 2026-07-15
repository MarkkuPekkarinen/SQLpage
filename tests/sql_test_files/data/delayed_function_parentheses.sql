select
    (sqlpage.url_encode(subquery.space)) as actual,
    '%20' as expected
from (
    select ' ' as space
) AS subquery;
