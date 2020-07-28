use sqlx::{self, postgres::PgPool, Postgres, QueryAs};
use uuid::Uuid;

use crate::errors::BackendError;
use crate::recording::{NewRecording, RecordingMetadata};

pub async fn insert_recording_metadata(
    metadata: RecordingMetadata,
    pool: &PgPool,
) -> Result<NewRecording, BackendError> {
    use sqlx::prelude::*;

    let query: QueryAs<Postgres, (Uuid,)> = sqlx::query_as(include_str!("queries/create.sql"));

    let (id, ) = query
        .bind(&metadata.age)
        .bind(&metadata.gender)
        .bind(&metadata.location)
        .bind(&metadata.name)
        .bind(&metadata.occupation)
        .bind(&metadata.created)
        .bind(&metadata.category)
        .bind(&metadata.unlisted)
        .fetch_one(pool)
        .await
        .map_err(|e| BackendError::Sqlx { source: e })?;

    Ok(NewRecording::new(id, metadata))
}
