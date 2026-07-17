set my_var = (
    select sqlpage.fetch('this invalid fetch should never be executed')
    where 1 = 0
);
select null as expected, $my_var as actual;

set norow = (
    select sqlpage.fetch('this invalid fetch should never be executed')
    where 1 = 0
);
select null as expected, $norow as actual;
