select 'cached' as expected,
    coalesce(cached_response, sqlpage.fetch('this invalid fetch must not run')) as actual
from (select 'cached' as cached_response) cache_row;

select '%20' as expected,
    coalesce(cached_response, sqlpage.url_encode(' ')) as actual
from (select null as cached_response) cache_row;
