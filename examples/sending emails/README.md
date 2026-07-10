# Sending Emails with SQLPage

This example sends plain-text email with [`sqlpage.send_mail`](https://sql-page.com/functions.sql?function=send_mail). The included Docker Compose setup uses [Mailpit](https://mailpit.axllent.org/) as a local SMTP server, so no email leaves your computer.

Run the example:

```sh
docker compose up
```

Open http://localhost:8080 to send an email, then inspect it in the Mailpit inbox at http://localhost:8025.

The SMTP server is configured in [`docker-compose.yml`](./docker-compose.yml) with `SMTP_HOST=mailpit`, `SMTP_PORT=1025`, and `SMTP_TLS_MODE=none`. Plaintext mode is intended only for trusted local SMTP servers such as Mailpit.

For a remote SMTP relay, keep the default `SMTP_TLS_MODE=starttls`, or set it to `tls` when the relay requires implicit TLS. Configure `SMTP_USERNAME` and `SMTP_PASSWORD` when authentication is required; SQLPage rejects credentials in plaintext mode.

The form handler sends the message with a single function call:

```sql
set message = json_object(
    'to', :recipient,
    'from', :sender,
    'subject', :subject,
    'body', :body
);
set _ = sqlpage.send_mail($message);
```

`sqlpage.send_mail` returns `NULL` after the SMTP relay accepts the message. It raises an error when the relay rejects the message or cannot be reached, so statements after the call run only on success.

Do not expose an unrestricted form like this publicly. In production, authenticate users, restrict recipients, validate input, and add rate limiting to prevent abuse.
