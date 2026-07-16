select 'http_header' as component,
    printf('<%s>; rel="canonical"',
            iif($function is not null, sqlpage.link('functions', json_object('function', $function)), 'functions.sql')
    ) as "Link";

select 'dynamic' as component, json_patch(json_extract(properties, '$[0]'), json_object(
    'title', coalesce($function || ' - ', '') || 'SQLPage Functions Documentation'
)) as properties
FROM example WHERE component = 'shell' LIMIT 1;

select 'breadcrumb' as component;
select 'SQLPage' as title, '/' as link, 'Home page' as description;
select 'Functions' as title, '/functions.sql' as link, 'List of all functions' as description;
select $function as title, sqlpage.set_variable('function', $function) as link where $function IS NOT NULL;

select 'text' as component, 'SQLPage built-in functions' as title where $function IS NULL;
select '
In addition to normal SQL functions supported by your database,
SQLPage provides a few special functions to help you extract data from user requests.

These functions are special because they are not database functions.
SQLPage evaluates them itself, either before or after the database query.

When a SQLPage function call is the whole value of a selected column, SQLPage first lets the database decide which rows exist.
It selects the function arguments as hidden columns, then calls the SQLPage function once for each returned row.
For example, `SELECT sqlpage.url_encode(url) AS encoded FROM t` runs the database query first, then applies `url_encode` to every returned `url` value.
If the query returns no row, the function is not called.
This also applies inside scalar `SET` subqueries, for instance `SET body = (SELECT sqlpage.fetch(url) FROM cache_misses WHERE enabled)`.

In other positions, SQLPage functions run before the query, but only when their arguments can be evaluated without reading database columns.
For instance, `WHERE sqlpage.cookie(''x'') = ''1''` can run before the query, but `WHERE sqlpage.fetch(url) IS NOT NULL` cannot because `url` is a database column.

If a SQLPage function is expensive or has side effects and should run only once, store its result with `SET` first and reuse the variable.
If a function should run only when a database row exists, put it as a standalone selected column in that row-producing query.

For more information about how SQLPage functions are evaluated, and data types in SQLPage, read [the SQLPage data model documentation](/extensions-to-sql).
' as contents_md where $function IS NULL;

select 'list' as component, 'SQLPage functions' as title where $function IS NULL;
select name as title,
    icon,
    '?function=' || name || '#function' as link,
    $function = name as active
from sqlpage_functions
where $function IS NULL
order by name;

select 'text' as component, 'sqlpage.' || $function || '(' || string_agg(name, ', ') || ')' as title, 'function' as id
from sqlpage_function_parameters where $function IS NOT NULL and "function" = $function;

select 'text' as component;
select 'Introduced in SQLPage ' || introduced_in_version || '.' as contents, 1 as size from sqlpage_functions where name = $function;

SELECT description_md as contents_md FROM sqlpage_functions WHERE name = $function;

select 'title' as component, 3 as level, 'Parameters' as contents where $function IS NOT NULL AND EXISTS (SELECT 1 from sqlpage_function_parameters where "function" = $function);
select 'card' as component, 3 AS columns where $function IS NOT NULL;
select
    name as title,
    description_md as description,
    type as footer,
    'azure' as color
from sqlpage_function_parameters where "function" = $function
ORDER BY "index";

select
    'button' as component,
    'sm'     as size,
    'pill'   as shape;
select
    name as title,
    icon,
    sqlpage.set_variable('function', name) as link
from sqlpage_functions
where $function IS NOT NULL
order by name;
