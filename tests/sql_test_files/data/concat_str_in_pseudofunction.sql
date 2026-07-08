select '%2F1' as expected, sqlpage.url_encode('/' || $x) as actual;
select '%2F1' as expected, sqlpage.url_encode(CONCAT('/', $x)) as actual;
select 'fallback' as expected, coalesce(sqlpage.url_encode(CONCAT('/', $thisisnull)), 'fallback') as actual;
