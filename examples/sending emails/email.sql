set message = json_object(
    'to', :recipient,
    'from', :sender,
    'subject', :subject,
    'body', :body
);
set sent_message = sqlpage.send_mail($message);

select
    'alert' as component,
    'success' as color,
    'Email sent successfully' as title;

select 'button' as component;
select 'Send another email' as title, 'index.sql' as link;
