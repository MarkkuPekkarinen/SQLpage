INSERT INTO sqlpage_functions (
        "name",
        "introduced_in_version",
        "icon",
        "description_md",
        "return_type"
    )
VALUES (
        'send_mail',
        '0.45.0',
        'mail',
        'Sends an email using the SMTP server configured with `STMP_HOST`.

`STMP_HOST` must contain only a host name or `host:port`; URL schemes and paths are rejected. When no port is specified, SQLPage uses port 25.

If your SMTP server requires authentication, configure `STMP_USERNAME` and `STMP_PASSWORD` as well.

The function accepts a single JSON object argument. The required properties are:

- `recipient`: email address to send to, optionally including a display name such as `"Jane Doe <jane@example.com>"`.
- `subject`: email subject.
- `body`: plain text email body.

Optional properties:

- `sender`: sender address. Defaults to `SQLPage <sqlpage@localhost>`.
- `reply_to`: reply-to address.

The function returns `sent` after the SMTP server accepts the message, and `NULL` when passed `NULL`.

### Example

```sql
select sqlpage.send_mail(json_object(
    ''recipient'', ''admin@example.com'',
    ''sender'', ''contact@example.com'',
    ''subject'', ''New contact form message'',
    ''body'', ''Hello from SQLPage!''
));
```

### Contact form example

```sql
select ''form'' as component, ''post'' as method;
select ''email'' as name, ''email'' as type, true as required;
select ''message'' as name, ''textarea'' as type, true as required;

select sqlpage.send_mail(json_object(
    ''recipient'', ''admin@example.com'',
    ''reply_to'', $email,
    ''subject'', ''Website contact form'',
    ''body'', $message
))
where $message is not null;
```
',
        'TEXT'
    );

INSERT INTO sqlpage_function_parameters (
        "function",
        "index",
        "name",
        "description_md",
        "type"
    )
VALUES (
        'send_mail',
        1,
        'message',
        'A JSON object containing the email to send. Required properties are `recipient`, `subject`, and `body`. Optional properties are `sender` and `reply_to`.',
        'JSON'
    );
