GET http://127.0.0.1:8080/
HTTP 200
[Asserts]
xpath "string(//form/@action)" == "email.sql"
xpath "string(//form/@method)" == "post"
xpath "string(//input[@name='recipient']/@value)" == "recipient@example.com"
xpath "string(//input[@name='subject']/@value)" == "Test email"
xpath "string(//textarea[@name='body'])" == "This is a test email"

POST http://127.0.0.1:8080/email.sql
[FormParams]
recipient: recipient@example.com
sender: SQLPage <sqlpage@example.com>
subject: SMTP demo
body: Sent with sqlpage.send_mail
HTTP 200
[Asserts]
xpath "string(//*[contains(@class, 'alert') and contains(., 'Email sent successfully')])" contains "Email sent successfully"

GET http://127.0.0.1:8025/api/v1/search?query=subject%3A%22SMTP%20demo%22
HTTP 200
[Asserts]
jsonpath "$.messages[0].Subject" == "SMTP demo"
jsonpath "$.messages[0].To[0].Address" == "recipient@example.com"
