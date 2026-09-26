use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Ngo {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub needs_description: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Donation {
    pub id: Uuid,
    pub company_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub quantity: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
}
