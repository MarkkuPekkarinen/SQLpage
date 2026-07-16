set x = (select 1 as a, sqlpage.fetch(upper('invalid')) as b);

select 'text' as component, 'It works !' as contents;
