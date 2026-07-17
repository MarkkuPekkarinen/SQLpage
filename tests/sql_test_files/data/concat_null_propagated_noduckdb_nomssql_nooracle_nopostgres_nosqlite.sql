select null as expected, concat('x', null) as actual;
select null as expected, sqlpage.url_encode(concat('/', null)) as actual;
