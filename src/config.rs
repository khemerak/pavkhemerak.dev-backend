/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub github_username: String,
    pub etherscan_api_key: Option<String>,
    pub admin_api_key: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3001".into())
                .parse()
                .expect("PORT must be a number"),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/pavkhemerak.db".into()),
            github_username: std::env::var("GITHUB_USERNAME")
                .unwrap_or_else(|_| "khemerak".into()),
            etherscan_api_key: std::env::var("ETHERSCAN_API_KEY").ok().filter(|s| !s.is_empty()),
            admin_api_key: std::env::var("ADMIN_API_KEY")
                .unwrap_or_else(|_| "change-me-in-production".into()),
        }
    }
}
