use futures::future::BoxFuture;

use crate::errors::BackendError;
use crate::recording::{NewRecording, RecordingMetadata};

pub trait Db {
    fn insert(&self, metadata: RecordingMetadata) -> BoxFuture<Result<NewRecording, BackendError>>;
}

pub use self::postgres::*;

mod postgres {
    use futures::future::BoxFuture;
    use sqlx::{self, postgres::PgPool, Postgres, QueryAs};
    use time::OffsetDateTime;
    use uuid::Uuid;

    use crate::errors::BackendError;
    use crate::recording::{NewRecording, RecordingMetadata};

    pub struct PgDb {
        pool: PgPool,
    }

    impl PgDb {
        pub fn new(pool: PgPool) -> Self {
            PgDb { pool }
        }
    }

    impl super::Db for PgDb {
        fn insert(
            &self,
            metadata: RecordingMetadata,
        ) -> BoxFuture<Result<NewRecording, BackendError>> {
            use futures::FutureExt;

            insert(metadata, &self.pool).boxed()
        }
    }

    async fn insert(
        metadata: RecordingMetadata,
        pool: &PgPool,
    ) -> Result<NewRecording, BackendError> {
        use sqlx::prelude::*;

        let query: QueryAs<Postgres, (Uuid, OffsetDateTime, OffsetDateTime)> = sqlx::query_as(include_str!("queries/create.sql"));

        let (id, created_at, updated_at) = query
            .bind(&metadata.age_id)
            .bind(&metadata.gender_id)
            .bind(&metadata.location)
            .bind(&metadata.name)
            .bind(&metadata.occupation)
            .bind(&metadata.category_id)
            .bind(&metadata.unlisted)
            .fetch_one(pool)
            .await
            .map_err(|e| BackendError::Sqlx { source: e })?;

        Ok(NewRecording::new(id, created_at, updated_at, metadata))
    }
}
