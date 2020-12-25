use futures::future::BoxFuture;
use url::Url;
use uuid::Uuid;

use crate::label::Label;
use crate::recording::{
    ChildRecording, NewRecording, PartialRecording, Recording, RecordingToken, UploadMetadata,
};
use crate::{audio::format::AudioFormat, errors::BackendError, mime_type::MimeType};

pub trait Db {
    fn children(&self, id: &Uuid) -> BoxFuture<Result<Vec<ChildRecording>, BackendError>>;

    fn count_all(&self) -> BoxFuture<Result<i64, BackendError>>;

    fn create_token(&self, parent_id: &Uuid) -> BoxFuture<Result<Uuid, BackendError>>;

    fn delete(&self, id: &Uuid) -> BoxFuture<Result<(), BackendError>>;

    fn lock_token(&self, token: &Uuid) -> BoxFuture<Result<Option<Uuid>, BackendError>>;

    fn insert(
        &self,
        parent_id: &Uuid,
        metadata: UploadMetadata,
    ) -> BoxFuture<Result<NewRecording, BackendError>>;

    fn retrieve(&self, id: &Uuid) -> BoxFuture<Result<Option<Recording>, BackendError>>;

    fn retrieve_ages(&self) -> BoxFuture<Result<Vec<Label>, BackendError>>;

    fn retrieve_categories(&self) -> BoxFuture<Result<Vec<Label>, BackendError>>;

    fn retrieve_format_essences(&self) -> BoxFuture<Result<Vec<String>, BackendError>>;

    fn retrieve_genders(&self) -> BoxFuture<Result<Vec<Label>, BackendError>>;

    fn retrieve_mime_type(
        &self,
        format: &AudioFormat,
    ) -> BoxFuture<Result<Option<MimeType>, BackendError>>;

    fn retrieve_random(&self, count: i16)
        -> BoxFuture<Result<Vec<PartialRecording>, BackendError>>;

    fn release_token(&self, token: &Uuid) -> BoxFuture<Result<(), BackendError>>;

    fn remove_token(&self, token: &Uuid) -> BoxFuture<Result<(), BackendError>>;

    fn retrieve_token(
        &self,
        token: &Uuid,
    ) -> BoxFuture<Result<Option<RecordingToken>, BackendError>>;

    fn update_url(
        &self,
        id: &Uuid,
        url: &Url,
        mime_type: MimeType,
    ) -> BoxFuture<Result<(), BackendError>>;
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

    use crate::label::{Id, Label};
    use crate::recording::{
        ChildRecording, NewRecording, PartialRecording, Recording, RecordingToken, Times,
        UploadMetadata,
    };
    use crate::{audio::format::AudioFormat, errors::BackendError, mime_type::MimeType};

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

        fn create_token(&self, parent: &Uuid) -> BoxFuture<Result<Uuid, BackendError>> {
            create_token(*parent, &self.pool).boxed()
        }

        fn delete(&self, id: &Uuid) -> BoxFuture<Result<(), BackendError>> {
            delete(*id, &self.pool).boxed()
        }

        fn insert(
            &self,
            parent_id: &Uuid,
            metadata: UploadMetadata,
        ) -> BoxFuture<Result<NewRecording, BackendError>> {
            insert(*parent_id, metadata, &self.pool).boxed()
        }

        fn lock_token(&self, token: &Uuid) -> BoxFuture<Result<Option<Uuid>, BackendError>> {
            lock_token(*token, &self.pool).boxed()
        }

        fn release_token(&self, token: &Uuid) -> BoxFuture<Result<(), BackendError>> {
            release_token(*token, &self.pool).boxed()
        }

        fn remove_token(&self, token: &Uuid) -> BoxFuture<Result<(), BackendError>> {
            remove_token(*token, &self.pool).boxed()
        }

        fn retrieve(&self, id: &Uuid) -> BoxFuture<Result<Option<Recording>, BackendError>> {
            retrieve(*id, &self.pool).boxed()
        }

        fn retrieve_ages(&self) -> BoxFuture<Result<Vec<Label>, BackendError>> {
            retrieve_ages(&self.pool).boxed()
        }

        fn retrieve_categories(&self) -> BoxFuture<Result<Vec<Label>, BackendError>> {
            retrieve_categories(&self.pool).boxed()
        }

        fn retrieve_format_essences(&self) -> BoxFuture<Result<Vec<String>, BackendError>> {
            retrieve_format_essences(&self.pool).boxed()
        }

        fn retrieve_genders(&self) -> BoxFuture<Result<Vec<Label>, BackendError>> {
            retrieve_genders(&self.pool).boxed()
        }

        fn retrieve_mime_type(
            &self,
            format: &AudioFormat,
        ) -> BoxFuture<Result<Option<MimeType>, BackendError>> {
            retrieve_mime_type(format.clone(), &self.pool).boxed()
        }

        fn retrieve_random(
            &self,
            count: i16,
        ) -> BoxFuture<Result<Vec<PartialRecording>, BackendError>> {
            retrieve_random(count, &self.pool).boxed()
        }

        fn retrieve_token(
            &self,
            token: &Uuid,
        ) -> BoxFuture<Result<Option<RecordingToken>, BackendError>> {
            retrieve_token(*token, &self.pool).boxed()
        }

        fn update_url(
            &self,
            id: &Uuid,
            url: &Url,
            mime_type: MimeType,
        ) -> BoxFuture<Result<(), BackendError>> {
            update_url(*id, url.clone(), mime_type, &self.pool).boxed()
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

        let query = sqlx::query_as::<_, (i64,)>(include_str!("queries/count.sql"));

        let (count,) = query.fetch_one(pool).await.map_err(map_sqlx_error)?;

        Ok(count)
    }

    async fn create_token(parent_id: Uuid, pool: &PgPool) -> Result<Uuid, BackendError> {
        use sqlx::prelude::*;

        let query: QueryAs<Postgres, (Uuid,)> =
            sqlx::query_as(include_str!("queries/create_token.sql"));

        let (token,) = query
            .bind(parent_id)
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(token)
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

    async fn insert(
        parent_id: Uuid,
        metadata: UploadMetadata,
        pool: &PgPool,
    ) -> Result<NewRecording, BackendError> {
        use sqlx::prelude::*;

        let query: QueryAs<Postgres, (Uuid, OffsetDateTime, OffsetDateTime)> =
            sqlx::query_as(include_str!("queries/create.sql"));

        let (id, created_at, updated_at) = query
            .bind(&DEFAULT_URL)
            .bind(None::<Option<i16>>)
            .bind(&metadata.category_id)
            .bind(parent_id)
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

    async fn lock_token(token: Uuid, pool: &PgPool) -> Result<Option<Uuid>, BackendError> {
        use sqlx::prelude::*;

        let query: QueryAs<Postgres, (Uuid,)> =
            sqlx::query_as(include_str!("queries/lock_token.sql"));

        let parent_id = query
            .bind(token)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_error)?
            .map(|(parent_id,)| parent_id);

        Ok(parent_id)
    }

    async fn release_token(token: Uuid, pool: &PgPool) -> Result<(), BackendError> {
        let query = sqlx::query(include_str!("queries/release_token.sql"));

        query
            .bind(token)
            .execute(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn remove_token(token: Uuid, pool: &PgPool) -> Result<(), BackendError> {
        let query = sqlx::query(include_str!("queries/remove_token.sql"));

        query
            .bind(token)
            .execute(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(())
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

    async fn retrieve_ages(pool: &PgPool) -> Result<Vec<Label>, BackendError> {
        let query = sqlx::query(include_str!("queries/retrieve_ages.sql"));

        let ages: Vec<Label> = query
            .try_map(|row: PgRow| Ok(Label::new(try_get(&row, "id")?, try_get(&row, "label")?)))
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(ages)
    }

    async fn retrieve_categories(pool: &PgPool) -> Result<Vec<Label>, BackendError> {
        let query = sqlx::query(include_str!("queries/retrieve_categories.sql"));

        let categories: Vec<Label> = query
            .try_map(|row: PgRow| Ok(Label::new(try_get(&row, "id")?, try_get(&row, "label")?)))
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(categories)
    }

    async fn retrieve_format_essences(pool: &PgPool) -> Result<Vec<String>, BackendError> {
        let query = sqlx::query(include_str!("queries/retrieve_format_essences.sql"));

        let essences: Vec<String> = query
            .try_map(|row: PgRow| Ok(try_get(&row, "essence")?))
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(essences)
    }

    async fn retrieve_genders(pool: &PgPool) -> Result<Vec<Label>, BackendError> {
        let query = sqlx::query(include_str!("queries/retrieve_genders.sql"));

        let genders: Vec<Label> = query
            .try_map(|row: PgRow| Ok(Label::new(try_get(&row, "id")?, try_get(&row, "label")?)))
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(genders)
    }

    async fn retrieve_mime_type(
        format: AudioFormat,
        pool: &PgPool,
    ) -> Result<Option<MimeType>, BackendError> {
        let query = sqlx::query(include_str!("queries/retrieve_mime_type.sql"));

        let mime_type: Option<MimeType> = query
            .bind(format.container)
            .bind(format.codec)
            .try_map(|row: PgRow| {
                let id: i16 = try_get(&row, "id")?;
                let essence: String = try_get(&row, "essence")?;
                let container: String = try_get(&row, "container")?;
                let codec: String = try_get(&row, "codec")?;
                let extension: String = try_get(&row, "extension")?;

                Ok(MimeType::new(
                    id,
                    AudioFormat::new(container, codec),
                    essence,
                    extension,
                ))
            })
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(mime_type)
    }

    async fn retrieve_random(
        count: i16,
        pool: &PgPool,
    ) -> Result<Vec<PartialRecording>, BackendError> {
        let query = sqlx::query(include_str!("queries/retrieve_random.sql"));

        let recordings = query
            .bind(count as i16)
            .try_map(|row: PgRow| {
                let id: Uuid = try_get(&row, "id")?;
                let name: String = try_get(&row, "name")?;
                let location: Option<String> = try_get(&row, "location")?;

                Ok(PartialRecording::new(id, name, location))
            })
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(recordings)
    }

    async fn retrieve_token(
        token: Uuid,
        pool: &PgPool,
    ) -> Result<Option<RecordingToken>, BackendError> {
        let query = sqlx::query(include_str!("queries/retrieve_token.sql"));

        let result: Option<RecordingToken> = query
            .bind(token)
            .try_map(|row: PgRow| {
                let id: Uuid = try_get(&row, "id")?;
                let parent_id: Uuid = try_get(&row, "parent_id")?;

                Ok(RecordingToken::new(id, parent_id))
            })
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(result)
    }

    async fn update_url(
        id: Uuid,
        url: Url,
        mime_type: MimeType,
        pool: &PgPool,
    ) -> Result<(), BackendError> {
        let query = sqlx::query(include_str!("queries/update_url.sql"));

        let _ = query
            .bind(id)
            .bind(url.as_str())
            .bind(mime_type.id)
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
        use crate::recording::ActiveRecording;

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
        let url: String = try_get(&row, "url")?;
        let url: Url = Url::parse(&url).map_err(|source| {
            // this should never happen, since we control the URLs
            // that go into the database, but just for completeness...
            sqlx::Error::Decode(Box::new(BackendError::UnableToParseUrl { url, source }))
        })?;

        let mime_type = Label::new(try_get(&row, "mime_type_id")?, try_get(&row, "mime_type")?);
        let category = Label::new(try_get(&row, "category_id")?, try_get(&row, "category")?);
        let gender = make_optional_label("gender_id", "gender")?;
        let age = make_optional_label("age_id", "age")?;

        let location: Option<String> = try_get(&row, "location")?;
        let occupation: Option<String> = try_get(&row, "occupation")?;

        Ok(Recording::Active(ActiveRecording::new(
            id, times, name, parent_id, url, mime_type, category, gender, age, location, occupation,
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
