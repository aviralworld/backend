use sqlx::{query, query_as, Postgres, Query, QueryAs};

use crate::recording::Recording;

/// Returns a query to insert a recording.
pub fn insertion() -> Query<'static, Postgres> {
    query("INSERT INTO recordings (age, gender, location, name, occupation, url, created, parent, category, unlisted) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)")
}

/// Returns a query to update a recording’s ‘unlisted’ status.
pub fn update_privacy() -> Query<'static, Postgres> {
    query("UPDATE recordings SET unlisted = $2 WHERE id = $1")
}

/// Returns a query to delete a recording.
pub fn deletion() -> Query<'static, Postgres> {
    query("DELETE FROM recordings WHERE id = $1")
}

/// Returns a query to retrieve a recording.
pub fn retrieval() -> Query<'static, Postgres> {
    query("SELECT age, gender, location, name, occupation, url, created, parent, category, unlisted FROM recordings WHERE id = $1 LIMIT 1")
}

/// Returns a query to count the existing recordings.
pub fn count() -> QueryAs<'static, Postgres, u64> {
    query_as("SELECT COUNT(*) FROM recordings")
}
