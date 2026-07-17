select 'x' as expected, concat('x', null) as actual;
select '%2F' as expected, sqlpage.url_encode(concat('/', null)) as actual;
