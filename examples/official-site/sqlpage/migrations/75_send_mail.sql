INSERT INTO sqlpage_functions (
        "name",
        "introduced_in_version",
        "icon",
        "description_md"
    )
VALUES (
        'send_mail',
        '0.45.0',
        'mail',
        'Sends an email using the SMTP server configured with `SMTP_HOST`.

`SMTP_HOST` contains the relay host name. Set `SMTP_PORT` when the relay does not use the default for the selected encryption mode: 587 for `starttls`, 465 for `tls`, or 25 for `none`.

`SMTP_TLS_MODE` defaults to `starttls`, which requires a STARTTLS upgrade before sending email or credentials. Set it to `tls` for implicit TLS, commonly used on port 465. Plaintext mode (`none`) is allowed only without credentials and should be used only for trusted local SMTP servers.

If your SMTP server requires authentication, configure `SMTP_USERNAME` and `SMTP_PASSWORD` as well.

The function accepts a single JSON object argument. The required properties are:

- `to`: email address to send to, optionally including a display name such as `"Jane Doe <jane@example.com>"`.
- `subject`: email subject.
- `body`: plain text email body.

Optional properties:

- `from`: sender address. It may be omitted when `SMTP_FROM` configures a default sender.
- `reply_to`: reply-to address.

The function returns `NULL` after the SMTP relay accepts the message and raises an error if the message cannot be sent. The argument is required; passing `NULL` is an error.

### Example

```sql
set message = json_object(
    ''to'', ''admin@example.com'',
    ''from'', ''contact@example.com'',
    ''subject'', ''New contact form message'',
    ''body'', ''Hello from SQLPage!''
);
select sqlpage.send_mail($message);
```

### Contact form example

```sql
select ''form'' as component, ''post'' as method;
select ''email'' as name, ''email'' as type, true as required;
select ''message'' as name, ''textarea'' as type, true as required;

set mail = json_object(
    ''to'', ''admin@example.com'',
    ''reply_to'', $email,
    ''subject'', ''Website contact form'',
    ''body'', $message
);
select sqlpage.send_mail($mail)
where $message is not null;
```
'
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
        'A JSON object containing the email to send. Required properties are `to`, `subject`, and `body`. Optional properties are `from` (required unless `SMTP_FROM` is configured) and `reply_to`. Unknown properties are rejected to catch misspellings.',
        'JSON'
    );
