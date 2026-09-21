use dotenv::dotenv;
use libsql::{Builder, Connection};
use std::env;

const START_FROM_SCRATCH: bool = false;

const DATABASE_SCHEMA: &str = include_str!("../../Database_SQL_Files/database_schema.sql");

pub fn truncate_to_char_limit(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect()
}

pub fn truncate_option_to_char_limit(value: Option<&str>, max_chars: usize) -> Option<String> {
    value.map(|v| truncate_to_char_limit(v, max_chars))
}

pub fn parse_lazy_flag(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("true")
}

pub fn utc_now_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub async fn connect_to_database() -> Result<Connection, Box<dyn std::error::Error>> {
    dotenv().ok();
    let db = Builder::new_local("./zigistry.db");
    let client = db.build().await.unwrap();
    let pool = client.connect().unwrap();
    if START_FROM_SCRATCH {
        pool.execute_batch(DATABASE_SCHEMA).await.unwrap();
    }
    Ok(pool)
}
