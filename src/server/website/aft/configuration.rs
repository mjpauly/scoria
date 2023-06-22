use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgSslMode;

#[derive(Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(Clone, Debug)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
    pub base_url: String,
}

#[derive(Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            // Try an encrypted connection, fallback to unencrypted if it fails
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(&self.database_name)
    }
}

#[derive(PartialEq)]
pub enum Environment {
    Local,
    Production,
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. \
                Use either `local` or `production`.",
                other
            )),
        }
    }
}

pub fn get_configuration() -> Settings {
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");
    if environment == Environment::Local {
        let host = std::env::var("INSIDE_DOCKER")
            .map(|_| "0.0.0.0".to_string())
            .unwrap_or_else(|_| "127.0.0.1".into());
        Settings {
            application: ApplicationSettings {
                port: 8000,
                host: host.clone(),
                base_url: format!("http://{}", host),
            },
            database: DatabaseSettings {
                username: "postgres".into(),
                password: Secret::new("password".into()),
                port: 5432,
                host: "127.0.0.1".into(),
                database_name: "feedback".into(),
                require_ssl: false,
            },
        }
    } else {
        println!("db username: {:?}", std::env::var("APP_DATABASE__USERNAME"));
        println!("db host: {:?}", std::env::var("APP_DATABASE__HOST"));
        println!("db port: {:?}", std::env::var("APP_DATABASE__PORT"));
        Settings {
            application: ApplicationSettings {
                port: 8000,
                host: "0.0.0.0".into(),
                base_url: std::env::var("APP_APPLICATION__BASE_URL").unwrap(),
            },
            database: DatabaseSettings {
                username: "".into(),
                password: Secret::new("".into()),
                port: 0,
                host: "".into(),
                database_name: "".into(),
                require_ssl: true,
            },
            // TODO:
            /*
            database: DatabaseSettings {
                username: std::env::var("APP_DATABASE__USERNAME").unwrap(),
                password: Secret::new(
                    std::env::var("APP_DATABASE__PASSWORD").unwrap(),
                ),
                port: std::env::var("APP_DATABASE__PORT")
                    .unwrap()
                    .parse::<u16>()
                    .unwrap(),
                host: std::env::var("APP_DATABASE__HOST").unwrap(),
                database_name: std::env::var("APP_DATABASE__DATABASE_NAME")
                    .unwrap(),
                require_ssl: true,
            },
            */
        }
    }
}
