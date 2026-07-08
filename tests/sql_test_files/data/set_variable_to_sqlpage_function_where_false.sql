set my_var = (
    select sqlpage.fetch('this invalid fetch should never be executed')
    where false
);
select null as expected, $my_var as actual;

set norow = (
    select sqlpage.fetch('this invalid fetch should never be executed')
        limit 0
);
select null as expected, $norow as actual;
