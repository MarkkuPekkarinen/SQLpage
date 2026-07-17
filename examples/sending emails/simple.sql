select 'shell' as component, 'Send a simple email' as title;

select
    'form' as component,
    'Simple email' as title,
    'send-simple.sql' as action,
    'post' as method,
    'Send email' as validate;

select 'email' as type, 'recipient' as name, 'To' as label, 'recipient@example.com' as value, true as required;
select 'subject' as name, 'Subject' as label, 'Simple SMTP demo' as value, true as required;
select 'textarea' as type, 'body' as name, 'Message' as label, 'Sent with sqlpage.send_mail' as value, true as required;

select 'button' as component;
select 'Back to examples' as title, 'index.sql' as link, 'arrow-left' as icon;
