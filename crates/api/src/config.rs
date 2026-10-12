pub struct Config {
    pub api_addr: String,
    pub site_origin: String,
    pub smtp_url: String,
    pub mail_from: String,
}

fn var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} is not set"))
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let site_origin = var("SITE_ORIGIN")?.trim_end_matches('/').to_string();
        if !site_origin.starts_with("https://") && !site_origin.starts_with("http://") {
            return Err("SITE_ORIGIN must start with https:// or http://".into());
        }
        Ok(Self {
            api_addr: std::env::var("API_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into()),
            site_origin,
            smtp_url: var("SMTP_URL")?,
            mail_from: var("MAIL_FROM")?,
        })
    }
}
