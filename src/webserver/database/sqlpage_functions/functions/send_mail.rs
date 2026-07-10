use std::borrow::Cow;

use anyhow::Context;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::{
        authentication::Credentials,
        client::{Certificate, CertificateStore, Tls, TlsParameters},
    },
};
use serde::Deserialize;

use crate::{
    app_config::{AppConfig, SmtpTlsMode},
    webserver::{http_client::native_certificate_der, http_request_info::RequestInfo},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MailRequest<'a> {
    #[serde(borrow)]
    to: Cow<'a, str>,
    #[serde(borrow)]
    subject: Cow<'a, str>,
    #[serde(borrow)]
    body: Cow<'a, str>,
    #[serde(borrow, default, rename = "from")]
    from: Option<Cow<'a, str>>,
    #[serde(borrow, default)]
    reply_to: Option<Cow<'a, str>>,
}

/// Sends an email through the configured SMTP relay.
pub(super) async fn send_mail(
    request: &RequestInfo,
    mail_request: Cow<'_, str>,
) -> anyhow::Result<()> {
    send_mail_with_config(&request.app_state.config, &mail_request).await
}

async fn send_mail_with_config(config: &AppConfig, mail_request: &str) -> anyhow::Result<()> {
    let host = config
        .smtp_host
        .as_deref()
        .context("sqlpage.send_mail() requires the smtp_host configuration option")?;
    let parsed: MailRequest<'_> = serde_json::from_str(mail_request)
        .context("sqlpage.send_mail() expects a JSON object")?;

    let sender = parsed
        .from
        .as_deref()
        .or(config.smtp_from.as_deref())
        .context("Email has no from address; set its from property or configure smtp_from")?
        .parse::<Mailbox>()
        .context("Invalid from email address")?;
    let recipient = parsed
        .to
        .parse::<Mailbox>()
        .context("Invalid to email address")?;

    let mut email = Message::builder()
        .from(sender)
        .to(recipient)
        .subject(parsed.subject.as_ref())
        .header(ContentType::TEXT_PLAIN);
    if let Some(reply_to) = parsed.reply_to {
        email = email.reply_to(
            reply_to
                .parse::<Mailbox>()
                .context("Invalid reply_to email address")?,
        );
    }
    let email = email
        .body(parsed.body.into_owned())
        .context("Unable to build email message")?;

    let tls = smtp_tls(config, host)?;
    let port = config
        .smtp_port
        .unwrap_or_else(|| config.smtp_tls_mode.default_port());
    let mut mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
        .port(port)
        .tls(tls);
    if let (Some(username), Some(password)) = (&config.smtp_username, &config.smtp_password) {
        mailer = mailer.credentials(Credentials::new(username.clone(), password.clone()));
    }

    let response = mailer
        .build()
        .send(email)
        .await
        .with_context(|| format!("Unable to send email through {host}:{port}"))?;
    log::debug!("SMTP relay accepted email: {response:?}");
    Ok(())
}

fn smtp_tls(config: &AppConfig, host: &str) -> anyhow::Result<Tls> {
    if config.smtp_tls_mode == SmtpTlsMode::None {
        return Ok(Tls::None);
    }

    let mut parameters = TlsParameters::builder(host.to_string());
    if config.system_root_ca_certificates {
        parameters = parameters.certificate_store(CertificateStore::None);
        for certificate in native_certificate_der()? {
            parameters = parameters.add_root_certificate(
                Certificate::from_der(certificate.as_ref().to_vec())
                    .context("Unable to configure an SMTP root certificate")?,
            );
        }
    } else {
        parameters = parameters.certificate_store(CertificateStore::WebpkiRoots);
    }
    let parameters = parameters
        .build_rustls()
        .context("Unable to configure SMTP TLS")?;
    Ok(match config.smtp_tls_mode {
        SmtpTlsMode::Starttls => Tls::Required(parameters),
        SmtpTlsMode::Tls => Tls::Wrapper(parameters),
        SmtpTlsMode::None => unreachable!(),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc,
        thread,
    };

    use super::{SmtpTlsMode, send_mail_with_config};
    use crate::app_config::tests::test_config;

    #[tokio::test]
    async fn sends_plain_text_email_to_configured_relay() {
        let (host, port, received) = start_smtp_server();
        let mut config = test_config();
        config.smtp_host = Some(host);
        config.smtp_port = Some(port);
        config.smtp_tls_mode = SmtpTlsMode::None;

        send_mail_with_config(
            &config,
            r#"{
                "to": "admin@example.com",
                "from": "contact@example.com",
                "subject": "SMTP test",
                "body": "hello smtp"
            }"#,
        )
        .await
        .unwrap();

        let data = received.recv().unwrap();
        assert!(data.contains("Subject: SMTP test"));
        assert!(data.contains("hello smtp"));
    }

    #[tokio::test]
    async fn rejects_unknown_message_fields() {
        let mut config = test_config();
        config.smtp_host = Some("localhost".to_string());
        let error = send_mail_with_config(
            &config,
            r#"{"recipient":"admin@example.com","subject":"test","body":"hello"}"#,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("expects a JSON object"));
    }

    fn start_smtp_server() -> (String, u16, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            sender.send(handle_smtp_connection(stream)).unwrap();
        });
        (address.ip().to_string(), address.port(), receiver)
    }

    fn handle_smtp_connection(mut stream: TcpStream) -> String {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        write_response(&mut stream, "220 localhost ESMTP test server");
        let mut data = String::new();
        loop {
            let line = read_line(&mut reader);
            let command = line.trim_end_matches(['\r', '\n']);
            if command.starts_with("EHLO") || command.starts_with("HELO") {
                write_response(&mut stream, "250 localhost");
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
            } else {
                write_response(&mut stream, "250 OK");
            }
        }
        data
    }

    fn read_line(reader: &mut BufReader<TcpStream>) -> String {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        line
    }

    fn write_response(stream: &mut TcpStream, response: &str) {
        write!(stream, "{response}\r\n").unwrap();
    }
}
