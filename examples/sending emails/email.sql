set message = json_object(
    'recipient', :recipient,
    'sender', :sender,
    'subject', :subject,
    'body', :body
);
set sent_message = sqlpage.send_mail($message);

select
    'alert' as component,
    'success' as color,
    'Email sent successfully' as title
where $sent_message is not null;

select 'button' as component;
select 'Send another email' as title, 'index.sql' as link;
