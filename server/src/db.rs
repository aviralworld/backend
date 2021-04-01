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

    fn create_management_token(&self, id: &Uuid, email: Option<String>) -> BoxFuture<Result<Uuid, BackendError>>;

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

    // these can be simplified once async functions in traits are stabilized
    impl super::Db for PgDb {
        fn children(&self, id: &Uuid) -> BoxFuture<Result<Vec<ChildRecording>, BackendError>> {
            let id = *id;

            async move {
                let query = sqlx::query_as::<_, ChildRecording>(include_str!(
                    "queries/retrieve_children.sql"
                ));

                let results = query
                    .bind(id)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(results)
            }
            .boxed()
        }

        fn count_all(&self) -> BoxFuture<Result<i64, BackendError>> {
            async move {
                let query = sqlx::query_as::<_, (i64,)>(include_str!("queries/count.sql"));

                let (count,) = query.fetch_one(&self.pool).await.map_err(map_sqlx_error)?;

                Ok(count)
            }
            .boxed()
        }

        fn create_management_token(&self, id: &Uuid, email: Option<String>) -> BoxFuture<Result<Uuid, BackendError>> {
            let id = *id;

            async move {
                let query = sqlx::query_as(include_str!("queries/create_management_token.sql"));

                let (token,): (Uuid,) = query
                    .bind(id)
                    .bind(email)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(token)
            }.boxed()
        }

        fn create_token(&self, parent: &Uuid) -> BoxFuture<Result<Uuid, BackendError>> {
            let parent_id = *parent;

            async move {
                let query = sqlx::query_as(include_str!("queries/create_token.sql"));

                let (token,): (Uuid,) = query
                    .bind(parent_id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(token)
            }
            .boxed()
        }

        fn delete(&self, id: &Uuid) -> BoxFuture<Result<(), BackendError>> {
            let id = *id;

            async move {
                let query = sqlx::query(include_str!("queries/delete.sql"));

                let count = query
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?
                    .rows_affected();

                if count == 0 {
                    Err(BackendError::NonExistentId(id))
                } else {
                    Ok(())
                }
            }
            .boxed()
        }

        fn insert(
            &self,
            parent_id: &Uuid,
            metadata: UploadMetadata,
        ) -> BoxFuture<Result<NewRecording, BackendError>> {
            let parent_id = *parent_id;

            async move {
                let query = sqlx::query_as(include_str!("queries/create.sql"));

                let (id, created_at, updated_at): (Uuid, OffsetDateTime, OffsetDateTime) = query
                    .bind(&DEFAULT_URL)
                    .bind(None::<Option<i16>>)
                    .bind(&metadata.category_id)
                    .bind(parent_id)
                    .bind(&metadata.name)
                    .bind(&metadata.location)
                    .bind(&metadata.occupation)
                    .bind(&metadata.age_id)
                    .bind(&metadata.gender_id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(NewRecording::new(id, created_at, updated_at, metadata))
            }
            .boxed()
        }

        fn lock_token(&self, token: &Uuid) -> BoxFuture<Result<Option<Uuid>, BackendError>> {
            let token = *token;

            async move {
                let query = sqlx::query_as(include_str!("queries/lock_token.sql"));

                let parent_id: Option<Uuid> = query
                    .bind(token)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?
                    .map(|(parent_id,)| parent_id);

                Ok(parent_id)
            }
            .boxed()
        }

        fn release_token(&self, token: &Uuid) -> BoxFuture<Result<(), BackendError>> {
            let token = *token;

            async move {
                let query = sqlx::query(include_str!("queries/release_token.sql"));

                query
                    .bind(token)
                    .execute(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(())
            }
            .boxed()
        }

        fn remove_token(&self, token: &Uuid) -> BoxFuture<Result<(), BackendError>> {
            let token = *token;

            async move {
                let query = sqlx::query(include_str!("queries/remove_token.sql"));

                query
                    .bind(token)
                    .execute(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(())
            }
            .boxed()
        }

        fn retrieve(&self, id: &Uuid) -> BoxFuture<Result<Option<Recording>, BackendError>> {
            let id = *id;

            async move {
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
                            Some(deleted_at) => {
                                new_deleted_recording(id, times, deleted_at, parent_id)
                            }
                            None => new_active_recording(id, times, parent_id, &row)?,
                        })
                    })
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(recording)
            }
            .boxed()
        }

        fn retrieve_ages(&self) -> BoxFuture<Result<Vec<Label>, BackendError>> {
            async move {
                let query = sqlx::query(include_str!("queries/retrieve_ages.sql"));

                let ages: Vec<Label> = query
                    .try_map(|row: PgRow| {
                        Ok(Label::new(
                            try_get(&row, "id")?,
                            try_get(&row, "label")?,
                            try_get_option(&row, "description")?,
                        ))
                    })
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(ages)
            }
            .boxed()
        }

        fn retrieve_categories(&self) -> BoxFuture<Result<Vec<Label>, BackendError>> {
            async move {
                let query = sqlx::query(include_str!("queries/retrieve_categories.sql"));

                let categories: Vec<Label> = query
                    .try_map(|row: PgRow| {
                        Ok(Label::new(
                            try_get(&row, "id")?,
                            try_get(&row, "label")?,
                            try_get_option(&row, "description")?,
                        ))
                    })
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(categories)
            }
            .boxed()
        }

        #[allow(clippy::needless_question_mark)]
        fn retrieve_format_essences(&self) -> BoxFuture<Result<Vec<String>, BackendError>> {
            async move {
                let query = sqlx::query(include_str!("queries/retrieve_format_essences.sql"));

                let essences: Vec<String> = query
                    .try_map(|row: PgRow| Ok(try_get(&row, "essence")?))
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(essences)
            }
            .boxed()
        }

        fn retrieve_genders(&self) -> BoxFuture<Result<Vec<Label>, BackendError>> {
            async move {
                let query = sqlx::query(include_str!("queries/retrieve_genders.sql"));

                let genders: Vec<Label> = query
                    .try_map(|row: PgRow| {
                        Ok(Label::new(
                            try_get(&row, "id")?,
                            try_get(&row, "label")?,
                            try_get_option(&row, "description")?,
                        ))
                    })
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(genders)
            }
            .boxed()
        }

        fn retrieve_mime_type(
            &self,
            format: &AudioFormat,
        ) -> BoxFuture<Result<Option<MimeType>, BackendError>> {
            let format = format.clone();

            async move {
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
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(mime_type)
            }
            .boxed()
        }

        fn retrieve_random(
            &self,
            count: i16,
        ) -> BoxFuture<Result<Vec<PartialRecording>, BackendError>> {
            async move {
                let query = sqlx::query(include_str!("queries/retrieve_random.sql"));

                let recordings = query
                    .bind(count)
                    .try_map(|row: PgRow| {
                        let id: Uuid = try_get(&row, "id")?;
                        let name: String = try_get(&row, "name")?;
                        let location: Option<String> = try_get(&row, "location")?;

                        Ok(PartialRecording::new(id, name, location))
                    })
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(recordings)
            }
            .boxed()
        }

        fn retrieve_token(
            &self,
            token: &Uuid,
        ) -> BoxFuture<Result<Option<RecordingToken>, BackendError>> {
            let token = *token;

            async move {
                let query = sqlx::query(include_str!("queries/retrieve_token.sql"));

                let result: Option<RecordingToken> = query
                    .bind(token)
                    .try_map(|row: PgRow| {
                        let id: Uuid = try_get(&row, "id")?;
                        let parent_id: Uuid = try_get(&row, "parent_id")?;

                        Ok(RecordingToken::new(id, parent_id))
                    })
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(result)
            }
            .boxed()
        }

        fn update_url(
            &self,
            id: &Uuid,
            url: &Url,
            mime_type: MimeType,
        ) -> BoxFuture<Result<(), BackendError>> {
            let id = *id;
            let url = url.clone();

            async move {
                let query = sqlx::query(include_str!("queries/update_url.sql"));

                let _ = query
                    .bind(id)
                    .bind(url.as_str())
                    .bind(mime_type.id)
                    .execute(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;

                Ok(())
            }
            .boxed()
        }
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
                    Some(id) => Ok(Some(Label::new(id, try_get(&row, label_column)?, None))),
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

        let mime_type = Label::new(
            try_get(&row, "mime_type_id")?,
            try_get(&row, "mime_type")?,
            None,
        );
        let category = Label::new(
            try_get(&row, "category_id")?,
            try_get(&row, "category")?,
            try_get_option(&row, "category_description")?,
        );
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
    ) -> Recording {
        use crate::recording::DeletedRecording;

        Recording::Deleted(DeletedRecording::new(id, times, deleted_at, parent_id))
    }

    fn try_get<'a, T: sqlx::Type<sqlx::Postgres> + sqlx::decode::Decode<'a, sqlx::Postgres>>(
        row: &'a PgRow,
        column: &str,
    ) -> Result<T, sqlx::Error> {
        use sqlx::prelude::*;

        row.try_get(column)
    }

    fn try_get_option<
        'a,
        T: sqlx::Type<sqlx::Postgres> + sqlx::decode::Decode<'a, sqlx::Postgres> + std::fmt::Debug,
    >(
        row: &'a PgRow,
        column: &str,
    ) -> Result<Option<T>, sqlx::Error> {
        use sqlx::prelude::*;

        let result: Result<Option<T>, sqlx::error::Error> = row.try_get(column);

        match result {
            Err(sqlx::Error::ColumnNotFound(_)) => Ok(None),
            x => x,
        }
    }

    fn map_sqlx_error(error: sqlx::Error) -> BackendError {
        use sqlx::Error;

        match error {
            Error::Database(ref e) if e.constraint() == Some(RECORDINGS_ID_CONSTRAINT) => {
                BackendError::IdAlreadyExists
            }
            Error::Database(ref e) if e.constraint() == Some(RECORDINGS_NAME_CONSTRAINT) => {
                BackendError::NameAlreadyExists
            }
            _ => BackendError::Sqlx { source: error },
        }
    }
}
