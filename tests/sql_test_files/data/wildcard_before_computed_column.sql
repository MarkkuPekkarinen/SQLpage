select
    users.*,
    'Alice%20Smith' as expected,
    sqlpage.url_encode(name) as actual
from (select 'first' as first, 'Alice Smith' as name) users;
