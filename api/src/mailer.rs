use html2text::from_read;
use lettre::message::{Mailbox, MultiPart};
use lettre::{Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use log::{info, warn};
use std::string::String;
use std::time::Duration;
use url::Url;

use crate::problems::Hook0Problem;

#[derive(Debug, Clone)]
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    sender: Mailbox,
    logo_url: Url,
    website_url: Url,
    app_url: String,
}

#[derive(Debug, Clone)]
pub enum Mail {
    VerifyUserEmail {
        url: String,
    },
    ResetPassword {
        url: String,
    },
    // Welcome { name: String },
    QuotaEventsPerDayWarning {
        pricing_url_hash: String,
        actual_consumption_percent: i32,
        current_events_per_day: i32,
        events_per_days_limit: i32,
        extra_variables: Vec<(String, String)>,
    },
    QuotaEventsPerDayReached {
        pricing_url_hash: String,
        current_events_per_day: i32,
        events_per_days_limit: i32,
        extra_variables: Vec<(String, String)>,
    },
}

impl Mail {
    pub fn template(&self) -> &'static str {
        match self {
            Mail::VerifyUserEmail { .. } => include_str!("mail_templates/verify_user_email.mjml"),
            Mail::ResetPassword { .. } => include_str!("mail_templates/reset_password.mjml"),
            // Mail::Welcome { .. } => include_str!("mail_templates/welcome.mjml"),
            Mail::QuotaEventsPerDayWarning { .. } => {
                include_str!("mail_templates/quotas/events_per_day_warning.mjml")
            }
            Mail::QuotaEventsPerDayReached { .. } => {
                include_str!("mail_templates/quotas/events_per_day_reached.mjml")
            }
        }
    }

    pub fn subject(&self) -> String {
        match self {
            Mail::VerifyUserEmail { .. } => "[Hook0] Verify your email address".to_owned(),
            Mail::ResetPassword { .. } => "[Hook0] Reset your password".to_owned(),
            // Mail::Welcome { .. } => "Welcome to our platform".to_owned(),
            Mail::QuotaEventsPerDayWarning { .. } => "[Hook0] Quota Warning".to_owned(),
            Mail::QuotaEventsPerDayReached { .. } => "[Hook0] Quota Reached".to_owned(),
        }
    }

    pub fn variables(&self) -> Vec<(String, String)> {
        match self {
            Mail::VerifyUserEmail { url } => vec![("url".to_owned(), url.to_owned())],
            Mail::ResetPassword { url } => vec![("url".to_owned(), url.to_owned())],
            // Mail::Welcome { name } => vec![("name".to_owned(), name.to_owned())],
            Mail::QuotaEventsPerDayWarning {
                pricing_url_hash,
                actual_consumption_percent,
                current_events_per_day,
                events_per_days_limit,
                extra_variables,
            } => {
                let mut vars = vec![
                    ("pricing_url_hash".to_owned(), pricing_url_hash.to_owned()),
                    (
                        "actual_consumption_percent".to_owned(),
                        actual_consumption_percent.to_string(),
                    ),
                    (
                        "current_events_per_day".to_owned(),
                        current_events_per_day.to_string(),
                    ),
                    (
                        "events_per_days_limit".to_owned(),
                        events_per_days_limit.to_string(),
                    ),
                ];
                vars.extend(extra_variables.clone());
                vars
            }
            Mail::QuotaEventsPerDayReached {
                pricing_url_hash,
                current_events_per_day,
                events_per_days_limit,
                extra_variables,
            } => {
                let mut vars = vec![
                    ("pricing_url_hash".to_owned(), pricing_url_hash.to_owned()),
                    (
                        "current_events_per_day".to_owned(),
                        current_events_per_day.to_string(),
                    ),
                    (
                        "events_per_days_limit".to_owned(),
                        events_per_days_limit.to_string(),
                    ),
                ];
                vars.extend(extra_variables.clone());
                vars
            }
        }
    }

    pub fn add_variable(&mut self, key: String, value: String) {
        match self {
            Mail::QuotaEventsPerDayWarning {
                extra_variables, ..
            } => {
                extra_variables.push((key, value));
            }
            Mail::QuotaEventsPerDayReached {
                extra_variables, ..
            } => {
                extra_variables.push((key, value));
            }
            _ => {}
        }
    }
}

impl Mailer {
    pub async fn new(
        smtp_connection_url: &str,
        smtp_timeout: Duration,
        sender_name: String,
        sender_address: Address,
        logo_url: Url,
        website_url: Url,
        app_url: String,
    ) -> Result<Mailer, lettre::transport::smtp::Error> {
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(smtp_connection_url)?
            .timeout(Some(smtp_timeout))
            .build();
        let sender = Mailbox::new(Some(sender_name), sender_address);

        let test = transport.test_connection().await;
        match test {
            Ok(true) => info!("SMTP server is up"),
            Ok(false) => warn!("SMTP server connection test failed"),
            Err(e) => warn!("SMTP server connection test failed: {e}"),
        }

        Ok(Mailer {
            transport,
            sender,
            logo_url,
            website_url,
            app_url,
        })
    }

    pub async fn send_mail(&self, mail: Mail, recipient: Mailbox) -> Result<(), Hook0Problem> {
        let template = mail.template();
        let mut mjml = template.to_owned();
        for (key, value) in mail.variables() {
            mjml = mjml.replace(&format!("{{ ${key} }}"), &value);
        }

        mjml = mjml.replace("{ $logo_url }", self.logo_url.as_str());
        mjml = mjml.replace("{ $website_url }", self.website_url.as_str());
        mjml = mjml.replace("{ $app_url }", self.app_url.as_str());

        let parsed = mrml::parse(mjml)?;
        let rendered = parsed.render(&Default::default())?;

        let text_mail = from_read(rendered.as_bytes(), 80)?;

        let email = Message::builder()
            .from(self.sender.to_owned())
            .to(recipient)
            .subject(mail.subject())
            .multipart(MultiPart::alternative_plain_html(text_mail, rendered))?;

        self.transport.send(email).await?;
        Ok(())
    }
}
