use futures::future::BoxFuture;
use url::Url;
use uuid::Uuid;

use crate::errors::BackendError;
use crate::recording::{ChildRecording, NewRecording, UploadMetadata};

pub trait Db {
    fn children(&self, id: &Uuid) -> BoxFuture<Result<Vec<ChildRecording>, BackendError>>;

    fn delete(&self, id: &Uuid) -> BoxFuture<Result<(), BackendError>>;

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
    use crate::recording::{ChildRecording, NewRecording, UploadMetadata};

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

    // all of these forward to async functions until async fn in traits is supported
    impl super::Db for PgDb {
        fn children(&self, id: &Uuid) -> BoxFuture<Result<Vec<ChildRecording>, BackendError>> {
            children(id.clone(), &self.pool).boxed()
        }

        fn delete(&self, id: &Uuid) -> BoxFuture<Result<(), BackendError>> {
            delete(id.clone(), &self.pool).boxed()
        }

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

    async fn children(id: Uuid, pool: &PgPool) -> Result<Vec<ChildRecording>, BackendError> {
        use sqlx::prelude::*;

        let query =
            sqlx::query_as::<_, ChildRecording>(include_str!("queries/retrieve_children.sql"));

        let results = query
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(results)
    }

    async fn delete(id: Uuid, pool: &PgPool) -> Result<(), BackendError> {
        let query = sqlx::query(include_str!("queries/delete.sql"));

        let count = query.bind(id).execute(pool).await.map_err(map_sqlx_error)?;

        if count == 0 {
            Err(BackendError::NonExistentId(id))
        } else {
            Ok(())
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
            .map_err(map_sqlx_error)?;

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
                BackendError::IdAlreadyExists
            }
            Error::Database(ref e) if e.constraint_name() == Some(RECORDINGS_NAME_CONSTRAINT) => {
                BackendError::NameAlreadyExists
            }
            _ => BackendError::Sqlx { source: error },
        }
    }
}
