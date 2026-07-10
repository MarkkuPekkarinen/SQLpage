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
        'Sends an email using the SMTP server configured with `SMTP_HOST`.

`SMTP_HOST` must contain only a host name or `host:port`; URL schemes and paths are rejected. When no port is specified, SQLPage uses port 25.

`SMTP_TLS_MODE` defaults to `starttls`, which requires a STARTTLS upgrade before sending email or credentials. Set it to `tls` for implicit TLS, commonly used on port 465. Plaintext mode (`none`) is allowed only without credentials and should be used only for trusted local SMTP servers.

If your SMTP server requires authentication, configure `SMTP_USERNAME` and `SMTP_PASSWORD` as well.

The function accepts a single JSON object argument. The required properties are:

- `recipient`: email address to send to, optionally including a display name such as `"Jane Doe <jane@example.com>"`.
- `subject`: email subject.
- `body`: plain text email body.

Optional properties:

- `sender`: sender address. Defaults to `SQLPage <sqlpage@localhost>`.
- `reply_to`: reply-to address.

After the SMTP server accepts the message, the function returns its JSON argument unchanged. It returns `NULL` when passed `NULL`, and raises an error if the message cannot be sent.

### Example

```sql
set message = json_object(
    ''recipient'', ''admin@example.com'',
    ''sender'', ''contact@example.com'',
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
    ''recipient'', ''admin@example.com'',
    ''reply_to'', $email,
    ''subject'', ''Website contact form'',
    ''body'', $message
);
select sqlpage.send_mail($mail)
where $message is not null;
```
',
        'JSON'
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
