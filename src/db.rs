use futures::future::BoxFuture;
use url::Url;
use uuid::Uuid;

use crate::errors::BackendError;
use crate::recording::{NewRecording, UploadMetadata};

pub trait Db {
    fn insert(&self, metadata: UploadMetadata) -> BoxFuture<Result<NewRecording, BackendError>>;

    fn update_url(&self, id: &Uuid, url: &Url) -> BoxFuture<Result<(), BackendError>>;
}

pub use self::postgres::*;

mod postgres {
    use futures::future::BoxFuture;
    use futures::FutureExt;
    use sqlx::{self, postgres::PgPool, Postgres, QueryAs};
    use time::OffsetDateTime;
    use url::Url;
    use uuid::Uuid;

    use crate::errors::BackendError;
    use crate::recording::{NewRecording, UploadMetadata};

    static DEFAULT_URL: Option<String> = None;

    const RECORDINGS_ID_CONSTRAINT: &str = "recordings_primary_key";
    const RECORDINGS_NAME_CONSTRAINT: &str = "recordings_name";

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
            metadata: UploadMetadata,
        ) -> BoxFuture<Result<NewRecording, BackendError>> {
            insert(metadata, &self.pool).boxed()
        }

        fn update_url(&self, id: &Uuid, url: &Url) -> BoxFuture<Result<(), BackendError>> {
            update_url(id.clone(), url.clone(), &self.pool).boxed()
        }
    }

    async fn insert(metadata: UploadMetadata, pool: &PgPool) -> Result<NewRecording, BackendError> {
        use sqlx::prelude::*;

        let query: QueryAs<Postgres, (Uuid, OffsetDateTime, OffsetDateTime)> =
            sqlx::query_as(include_str!("queries/create.sql"));

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
            .map_err(map_sqlx_error)?;

        Ok(())
    }

    fn map_sqlx_error(error: sqlx::Error) -> BackendError {
        use sqlx::Error;

        match error {
            Error::Database(ref e) if e.constraint_name() == Some(RECORDINGS_ID_CONSTRAINT) => {
                BackendError::DuplicateId
            }
            Error::Database(ref e) if e.constraint_name() == Some(RECORDINGS_NAME_CONSTRAINT) => {
                BackendError::DuplicateName
            }
            _ => BackendError::Sqlx { source: error },
        }
    }
}
