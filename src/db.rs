use futures::future::BoxFuture;
use url::Url;
use uuid::Uuid;

use crate::errors::BackendError;
use crate::recording::{NewRecording, RecordingMetadata};

pub trait Db {
    fn insert(&self, metadata: RecordingMetadata) -> BoxFuture<Result<NewRecording, BackendError>>;

    fn update_url(&self, id: Uuid, url: Url) -> BoxFuture<Result<(), BackendError>>;
}

pub use self::postgres::*;

mod postgres {
    use futures::FutureExt;
    use futures::future::BoxFuture;
    use sqlx::{self, postgres::PgPool, Postgres, QueryAs};
    use time::OffsetDateTime;
    use url::Url;
    use uuid::Uuid;

    use crate::errors::BackendError;
    use crate::recording::{NewRecording, RecordingMetadata};

    static DEFAULT_URL: Option<String> = None;

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
            insert(metadata, &self.pool).boxed()
        }

        fn update_url(&self, id: Uuid, url: Url) -> BoxFuture<Result<(), BackendError>> {
           update_url(id, url, &self.pool).boxed()
        }
    }

    async fn insert(
        metadata: RecordingMetadata,
        pool: &PgPool,
    ) -> Result<NewRecording, BackendError> {
        use sqlx::prelude::*;

        let query: QueryAs<Postgres, (Uuid, OffsetDateTime, OffsetDateTime)> = sqlx::query_as(include_str!("queries/create.sql"));

        let (id, created_at, updated_at) = query
            .bind(&DEFAULT_URL)
            .bind(&metadata.category_id)
            .bind(&metadata.parent_id)
            .bind(&metadata.unlisted)
            .bind(&metadata.name)
            .bind(&metadata.location)
            .bind(&metadata.occupation)
            .bind(&metadata.age_id)
            .bind(&metadata.gender_id)
            .fetch_one(pool)
            .await
            .map_err(|e| BackendError::Sqlx { source: e })?;

        Ok(NewRecording::new(id, created_at, updated_at, metadata))
    }

    async fn update_url(id: Uuid, url: Url, pool: &PgPool) -> Result<(), BackendError> {
        let query = sqlx::query(include_str!("queries/update_url.sql"));

        let _ = query
            .bind(id)
            .bind(url.as_str())
            .execute(pool)
            .await
            .map_err(|e| BackendError::Sqlx { source: e })?;

        Ok(())
    }
}
