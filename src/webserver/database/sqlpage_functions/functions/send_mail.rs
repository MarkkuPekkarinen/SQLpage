use std::borrow::Cow;

use anyhow::Context;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use serde::Deserialize;

use crate::{
    app_config::{AppConfig, parse_stmp_host},
    webserver::http_request_info::RequestInfo,
};

#[derive(Deserialize)]
struct MailRequest<'a> {
    #[serde(borrow)]
    recipient: Cow<'a, str>,
    #[serde(borrow)]
    subject: Cow<'a, str>,
    #[serde(borrow)]
    body: Cow<'a, str>,
    #[serde(borrow, default)]
    sender: Option<Cow<'a, str>>,
    #[serde(borrow, default)]
    reply_to: Option<Cow<'a, str>>,
}

/// Sends an email through the SMTP server configured with `STMP_HOST`.
pub(super) async fn send_mail(
    request: &RequestInfo,
    mail_request: Option<Cow<'_, str>>,
) -> anyhow::Result<Option<&'static str>> {
    send_mail_with_config(&request.app_state.config, mail_request).await
}

async fn send_mail_with_config(
    config: &AppConfig,
    mail_request: Option<Cow<'_, str>>,
) -> anyhow::Result<Option<&'static str>> {
    let Some(mail_request) = mail_request else {
        return Ok(None);
    };
    let stmp_host = config
        .stmp_host
        .as_deref()
        .context("The sqlpage.send_mail() function requires the STMP_HOST configuration option")?;
    let (host, port) = parse_stmp_host(stmp_host)?;
    let mail_request: MailRequest<'_> = serde_json::from_str(&mail_request)
        .context("sqlpage.send_mail() expects a JSON object argument")?;

    let sender = mail_request
        .sender
        .as_deref()
        .unwrap_or("SQLPage <sqlpage@localhost>")
        .parse::<Mailbox>()
        .context("Invalid sender email address")?;
    let recipient = mail_request
        .recipient
        .parse::<Mailbox>()
        .context("Invalid recipient email address")?;

    let mut email = Message::builder()
        .from(sender)
        .to(recipient)
        .subject(mail_request.subject.as_ref())
        .header(ContentType::TEXT_PLAIN);
    if let Some(reply_to) = mail_request.reply_to {
        email = email.reply_to(
            reply_to
                .parse::<Mailbox>()
                .context("Invalid reply_to email address")?,
        );
    }
    let email = email
        .body(mail_request.body.into_owned())
        .context("Unable to build email message")?;

    let mut mailer_builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host).port(port);
    if let Some(username) = &config.stmp_username {
        mailer_builder = mailer_builder.credentials(Credentials::new(
            username.clone(),
            config.stmp_password.clone().unwrap_or_default(),
        ));
    }
    let mailer = mailer_builder.build();
    mailer
        .send(email)
        .await
        .with_context(|| format!("Unable to send email through {stmp_host}"))?;
    Ok(Some("sent"))
}

#[cfg(test)]
mod tests {
    use std::{
        borrow::Cow,
        io::{BufRead, BufReader, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc,
        thread,
    };

    use super::send_mail_with_config;
    use crate::app_config::tests::test_config;

    #[tokio::test]
    async fn send_mail_authenticates_to_smtp_server() {
        let (host, received) = start_authenticated_smtp_server("user", "secret");
        let mut config = test_config();
        config.stmp_host = Some(host);
        config.stmp_username = Some("user".to_string());
        config.stmp_password = Some("secret".to_string());

        let result = send_mail_with_config(
            &config,
            Some(Cow::Borrowed(
                r#"{
                    "recipient": "admin@example.com",
                    "sender": "contact@example.com",
                    "subject": "Authenticated SMTP",
                    "body": "hello authenticated smtp"
                }"#,
            )),
        )
        .await
        .unwrap();

        assert_eq!(result, Some("sent"));
        let smtp_session = received.recv().unwrap();
        assert!(smtp_session.authenticated, "SMTP AUTH was not used");
        assert!(smtp_session.data.contains("Authenticated SMTP"));
        assert!(smtp_session.data.contains("hello authenticated smtp"));
    }

    struct SmtpSession {
        authenticated: bool,
        data: String,
    }

    fn start_authenticated_smtp_server(username: &str, password: &str) -> (String, mpsc::Receiver<SmtpSession>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let username = username.to_string();
        let password = password.to_string();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let session = handle_smtp_connection(stream, &username, &password);
            sender.send(session).unwrap();
        });
        (address.to_string(), receiver)
    }

    fn handle_smtp_connection(mut stream: TcpStream, username: &str, password: &str) -> SmtpSession {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        write_response(&mut stream, "220 localhost ESMTP test server");
        let mut authenticated = false;
        let mut data = String::new();
        loop {
            let line = read_line(&mut reader);
            let command = line.trim_end_matches(['\r', '\n']);
            if command.starts_with("EHLO") || command.starts_with("HELO") {
                write!(
                    stream,
                    "250-localhost\r\n250-AUTH PLAIN LOGIN\r\n250 OK\r\n"
                )
                .unwrap();
            } else if let Some(auth) = command.strip_prefix("AUTH PLAIN ") {
                authenticated = auth == expected_plain_auth(username, password);
                write_response(&mut stream, if authenticated { "235 Authentication successful" } else { "535 Authentication failed" });
            } else if command == "DATA" {
                write_response(&mut stream, "354 End data with <CR><LF>.<CR><LF>");
                loop {
                    let line = read_line(&mut reader);
                    if line == ".\r\n" || line == ".\n" {
                        break;
                    }
                    data.push_str(&line);
                }
                write_response(&mut stream, "250 Message accepted");
            } else if command == "QUIT" {
                write_response(&mut stream, "221 Bye");
                break;
            } else if authenticated && (command.starts_with("MAIL FROM") || command.starts_with("RCPT TO")) {
                write_response(&mut stream, "250 OK");
            } else {
                write_response(&mut stream, "530 Authentication required");
            }
        }
        SmtpSession { authenticated, data }
    }

    fn read_line(reader: &mut BufReader<TcpStream>) -> String {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        line
    }

    fn write_response(stream: &mut TcpStream, response: &str) {
        write!(stream, "{response}\r\n").unwrap();
    }

    fn expected_plain_auth(username: &str, password: &str) -> String {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        STANDARD.encode(format!("\0{username}\0{password}"))
    }
}
