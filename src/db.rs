use futures::future::BoxFuture;
use url::Url;
use uuid::Uuid;

use crate::errors::BackendError;
use crate::recording::{ChildRecording, NewRecording, Recording, UploadMetadata};

pub trait Db {
    fn children(&self, id: &Uuid) -> BoxFuture<Result<Vec<ChildRecording>, BackendError>>;

    fn count_all(&self) -> BoxFuture<Result<i64, BackendError>>;

    fn delete(&self, id: &Uuid) -> BoxFuture<Result<(), BackendError>>;

    fn hide(&self, id: &Uuid) -> BoxFuture<Result<(), BackendError>>;

    fn insert(&self, metadata: UploadMetadata) -> BoxFuture<Result<NewRecording, BackendError>>;

    fn retrieve(&self, id: &Uuid) -> BoxFuture<Result<Option<Recording>, BackendError>>;

    fn update_url(&self, id: &Uuid, url: &Url) -> BoxFuture<Result<(), BackendError>>;
}

pub use self::postgres::*;

mod postgres {
    use futures::future::BoxFuture;
    use futures::FutureExt;
    use sqlx::{
        self,
        postgres::{PgPool, PgRow},
        Postgres, QueryAs,
    };
    use time::OffsetDateTime;
    use url::Url;
    use uuid::Uuid;

    use crate::errors::BackendError;
    use crate::recording::{ChildRecording, Id, NewRecording, Recording, Times, UploadMetadata};

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
            children(*id, &self.pool).boxed()
        }

        fn count_all(&self) -> BoxFuture<Result<i64, BackendError>> {
            count_all(&self.pool).boxed()
        }

        fn delete(&self, id: &Uuid) -> BoxFuture<Result<(), BackendError>> {
            delete(*id, &self.pool).boxed()
        }

        fn hide(&self, id: &Uuid) -> BoxFuture<Result<(), BackendError>> {
            hide(*id, &self.pool).boxed()
        }

        fn insert(
            &self,
            metadata: UploadMetadata,
        ) -> BoxFuture<Result<NewRecording, BackendError>> {
            insert(metadata, &self.pool).boxed()
        }

        fn retrieve(&self, id: &Uuid) -> BoxFuture<Result<Option<Recording>, BackendError>> {
            retrieve(*id, &self.pool).boxed()
        }

        fn update_url(&self, id: &Uuid, url: &Url) -> BoxFuture<Result<(), BackendError>> {
            update_url(*id, url.clone(), &self.pool).boxed()
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

    async fn count_all(pool: &PgPool) -> Result<i64, BackendError> {
        use sqlx::prelude::*;

        let query = sqlx::query_as::<_, (i64, )>(include_str!("queries/count.sql"));

        let (count, ) = query
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(count)
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

    async fn hide(id: Uuid, pool: &PgPool) -> Result<(), BackendError> {
        let query = sqlx::query(include_str!("queries/update_privacy.sql"));

        let _ = query
            .bind(id)
            .bind(true)
            .execute(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(())
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

    async fn retrieve(id: Uuid, pool: &PgPool) -> Result<Option<Recording>, BackendError> {
        let query = sqlx::query(include_str!("queries/retrieve.sql"));

        let recording: Option<Recording> = query
            .bind(id)
            .try_map(|row: PgRow| {
                let id: Uuid = try_get(&row, "id")?;
                let created_at: OffsetDateTime = try_get(&row, "created_at")?;
                let updated_at: OffsetDateTime = try_get(&row, "updated_at")?;
                let deleted_at: Option<OffsetDateTime> = try_get(&row, "deleted_at")?;
                let parent_id: Option<Uuid> = try_get(&row, "parent_id")?;

                let times = Times {
                    created_at,
                    updated_at,
                };

                Ok(match deleted_at {
                    Some(deleted_at) => new_deleted_recording(id, times, deleted_at, parent_id)?,
                    None => new_active_recording(id, times, parent_id, &row)?,
                })
            })
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(recording)
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

    fn new_active_recording(
        id: Uuid,
        times: Times,
        parent_id: Option<Uuid>,
        row: &PgRow,
    ) -> Result<Recording, sqlx::Error> {
        use crate::recording::{ActiveRecording, Label};

        let make_optional_label =
            |id_column: &str, label_column: &str| -> Result<Option<Label>, sqlx::Error> {
                let id: Option<Id> = try_get(&row, id_column)?;

                // we use `match` here instead of `id.map` so that we can use `?`
                match id {
                    Some(id) => Ok(Some(Label::new(id, try_get(&row, label_column)?))),
                    None => Ok(None),
                }
            };

        let name: String = try_get(&row, "name")?;
        let unlisted: bool = try_get(&row, "unlisted")?;
        let url: String = try_get(&row, "url")?;
        let url: Url = Url::parse(&url).map_err(|source| {
            // this should never happen, since we control the URLs
            // that go into the database, but just for completeness...
            sqlx::Error::Decode(Box::new(BackendError::UnableToParseUrl { url, source }))
        })?;

        let category = Label::new(try_get(&row, "category_id")?, try_get(&row, "category")?);
        let gender = make_optional_label("gender_id", "gender")?;
        let age = make_optional_label("age_id", "age")?;

        let location: Option<String> = try_get(&row, "location")?;
        let occupation: Option<String> = try_get(&row, "occupation")?;

        Ok(Recording::Active(ActiveRecording::new(
            id, times, name, parent_id, url, category, gender, age, location, occupation, unlisted,
        )))
    }

    fn new_deleted_recording(
        id: Uuid,
        times: Times,
        deleted_at: OffsetDateTime,
        parent_id: Option<Uuid>,
    ) -> Result<Recording, sqlx::Error> {
        use crate::recording::DeletedRecording;

        Ok(Recording::Deleted(DeletedRecording::new(
            id, times, deleted_at, parent_id,
        )))
    }

    fn try_get<'a, T: sqlx::Type<sqlx::Postgres> + sqlx::decode::Decode<'a, sqlx::Postgres>>(
        row: &'a PgRow,
        column: &str,
    ) -> Result<T, sqlx::Error> {
        use sqlx::prelude::*;

        row.try_get(column)
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
