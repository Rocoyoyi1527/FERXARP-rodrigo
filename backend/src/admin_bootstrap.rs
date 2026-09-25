use std::{env, fmt};

use bcrypt::{DEFAULT_COST, hash};
use sqlx::PgPool;

const MIN_PASSWORD_LENGTH: usize = 12;

pub struct ProvisionConfig {
    email: String,
    password: String,
}

impl ProvisionConfig {
    pub fn from_env() -> Result<Self, ProvisionError> {
        Self::from_values(
            env::var("FERXARP_ADMIN_EMAIL").ok(),
            env::var("FERXARP_ADMIN_PASSWORD").ok(),
        )
    }

    pub fn from_values(
        email: Option<String>,
        password: Option<String>,
    ) -> Result<Self, ProvisionError> {
        let email = email
            .ok_or(ProvisionError::MissingEmail)?
            .trim()
            .to_string();
        if email.is_empty() {
            return Err(ProvisionError::MissingEmail);
        }
        let (local, domain) = email.split_once('@').ok_or(ProvisionError::InvalidEmail)?;
        if local.is_empty()
            || domain.is_empty()
            || domain.contains('@')
            || email.len() > 254
            || email.chars().any(char::is_control)
            || email.chars().any(char::is_whitespace)
        {
            return Err(ProvisionError::InvalidEmail);
        }

        let password = password.ok_or(ProvisionError::MissingPassword)?;
        if password.is_empty() {
            return Err(ProvisionError::MissingPassword);
        }
        if password.chars().count() < MIN_PASSWORD_LENGTH {
            return Err(ProvisionError::WeakPassword);
        }

        Ok(Self { email, password })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProvisionOutcome {
    Created,
    AlreadyExists,
}

impl ProvisionOutcome {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Created => "Administrador provisionado correctamente.",
            Self::AlreadyExists => "El administrador ya existe; no se modificó su contraseña.",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProvisionError {
    MissingDatabaseUrl,
    MissingEmail,
    InvalidEmail,
    MissingPassword,
    WeakPassword,
    ExistingNonAdmin,
    DatabaseUnavailable,
    DatabaseFailure,
    HashFailure,
}

impl fmt::Display for ProvisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingDatabaseUrl => "Falta DATABASE_URL.",
            Self::MissingEmail => "Falta FERXARP_ADMIN_EMAIL.",
            Self::InvalidEmail => "FERXARP_ADMIN_EMAIL no es válido.",
            Self::MissingPassword => "Falta FERXARP_ADMIN_PASSWORD.",
            Self::WeakPassword => "FERXARP_ADMIN_PASSWORD debe tener al menos 12 caracteres.",
            Self::ExistingNonAdmin => {
                "El correo ya pertenece a un usuario no administrador; se rechaza la elevación."
            }
            Self::DatabaseUnavailable => "No se pudo conectar a PostgreSQL.",
            Self::DatabaseFailure => "No se pudo consultar o actualizar el administrador.",
            Self::HashFailure => "No se pudo proteger la contraseña.",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ProvisionError {}

async fn existing_role(pool: &PgPool, email: &str) -> Result<Option<String>, ProvisionError> {
    sqlx::query_scalar("SELECT role::text FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
        .map_err(|_| ProvisionError::DatabaseFailure)
}

fn existing_outcome(role: &str) -> Result<ProvisionOutcome, ProvisionError> {
    if role == "admin" {
        Ok(ProvisionOutcome::AlreadyExists)
    } else {
        Err(ProvisionError::ExistingNonAdmin)
    }
}

pub async fn provision_admin(
    pool: &PgPool,
    config: &ProvisionConfig,
) -> Result<ProvisionOutcome, ProvisionError> {
    if let Some(role) = existing_role(pool, &config.email).await? {
        return existing_outcome(&role);
    }

    let password_hash =
        hash(&config.password, DEFAULT_COST).map_err(|_| ProvisionError::HashFailure)?;
    let inserted = sqlx::query(
        "INSERT INTO users (email, password_hash, role) \
         VALUES ($1, $2, 'admin'::user_role) \
         ON CONFLICT (email) DO NOTHING RETURNING id",
    )
    .bind(&config.email)
    .bind(password_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_| ProvisionError::DatabaseFailure)?;

    if inserted.is_some() {
        return Ok(ProvisionOutcome::Created);
    }

    match existing_role(pool, &config.email).await? {
        Some(role) => existing_outcome(&role),
        None => Err(ProvisionError::DatabaseFailure),
    }
}
