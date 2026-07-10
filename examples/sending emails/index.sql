select
    'form' as component,
    'Send an email through SMTP' as title,
    'email.sql' as action,
    'post' as method;

select 'recipient' as name, 'To' as label, 'recipient@example.com' as value, true as required;
select 'sender' as name, 'From' as label, 'SQLPage <sqlpage@example.com>' as value, true as required;
select 'subject' as name, 'Subject' as label, 'Test email' as value, true as required;
select 'textarea' as type, 'body' as name, 'Message' as label, 'This is a test email' as value, true as required;
