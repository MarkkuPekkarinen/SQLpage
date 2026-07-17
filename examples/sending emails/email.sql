set message = json_object(
    'to', :recipient,
    'from', :sender,
    'subject', :subject,
    'body', :body
);
set result = sqlpage.send_mail($message);

select
    'alert' as component,
    case when json_extract($result, '$.status') = 'accepted' then 'success' else 'danger' end as color,
    case when json_extract($result, '$.status') = 'accepted' then 'Email sent successfully' else 'Email could not be sent' end as title,
    json_extract($result, '$.error') as description;

select 'button' as component;
select 'Send another email' as title, 'index.sql' as link;
