set message = json_object(
    'to', :recipient,
    'subject', :subject,
    'body', :body
);
set result = sqlpage.send_mail($message);

select 'shell' as component, 'Simple email result' as title;

select
    'alert' as component,
    'send-result' as id,
    case when json_extract($result, '$.status') = 'accepted' then 'success' else 'danger' end as color,
    case when json_extract($result, '$.status') = 'accepted' then 'Email sent successfully' else 'Email could not be sent' end as title,
    json_extract($result, '$.error') as description;

select 'button' as component;
select 'Open Mailpit inbox' as title, 'http://localhost:8025' as link, '_blank' as target, 'inbox' as icon, 'primary' as color;
select 'Send another simple email' as title, 'simple.sql' as link;
select 'Back to examples' as title, 'index.sql' as link, 'secondary' as color;
