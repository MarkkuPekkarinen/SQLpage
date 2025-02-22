-- remove all existing members from this group
delete from group_members where group_id = 1;

-- add the selected members to this group
-- This query takes a JSON array and converts it to rows
-- :selected_items contains a JSON array of user IDs, e.g. ["1", "2", "3"], generated by SQLPage from the multiple-select box answers posted by the browser
-- json_table breaks down the JSON array into individual rows
-- '$[*]' means "look at each element in the root array"
-- columns (id int path '$') extracts each array element as an integer into a column named 'id'
-- The result is a temporary table with one integer column (id) and one row per array element
insert into group_members (group_id, user_id)
select 1, id
from json_table(
    :selected_items,
    '$[*]' columns (id int path '$')
) as submitted_items;

select 'alert' as component, 'Group members successfully updated !' as title, 'success' as color;

select 'list' as component, 'Users in this group' as title;

select name as title, email as description
from users
join group_members on users.id = group_members.user_id
where group_members.group_id = 1;

select 'button' as component;
select 'Go back' as title, 'index.sql' as link;