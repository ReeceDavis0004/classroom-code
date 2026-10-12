use std::sync::{Arc, Mutex};

use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox};

#[derive(Clone)]
pub enum Mailer {
    Smtp {
        transport: Box<AsyncSmtpTransport<Tokio1Executor>>,
        from: Mailbox,
    },
    Memory(Arc<Mutex<Vec<Sent>>>),
}

#[derive(Clone, Debug)]
pub struct Sent {
    pub to: String,
    pub subject: String,
    pub body: String,
}

impl Mailer {
    pub fn smtp(url: &str, from: &str) -> Result<Self, String> {
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(url)
            .map_err(|e| format!("SMTP_URL: {e}"))?
            .build()
            .into();
        let from = from.parse().map_err(|e| format!("MAIL_FROM: {e}"))?;
        Ok(Self::Smtp { transport, from })
    }

    pub fn memory() -> (Self, Arc<Mutex<Vec<Sent>>>) {
        let sent = Arc::new(Mutex::new(Vec::new()));
        (Self::Memory(sent.clone()), sent)
    }

    pub async fn send(&self, to: &str, subject: &str, body: String) -> Result<(), String> {
        match self {
            Self::Smtp { transport, from } => {
                let message = Message::builder()
                    .from(from.clone())
                    .to(to.parse().map_err(|e| format!("recipient: {e}"))?)
                    .subject(subject)
                    .body(body)
                    .map_err(|e| e.to_string())?;
                transport
                    .send(message)
                    .await
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            Self::Memory(sent) => {
                sent.lock().unwrap().push(Sent {
                    to: to.into(),
                    subject: subject.into(),
                    body,
                });
                Ok(())
            }
        }
    }
}
