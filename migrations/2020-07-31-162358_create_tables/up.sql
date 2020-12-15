CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS "ages" (
    id smallserial,
    label character varying(50) NOT NULL UNIQUE,
    enabled boolean NOT NULL DEFAULT TRUE,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "genders" (
    id smallserial,
    label character varying(100) NOT NULL UNIQUE,
    enabled boolean NOT NULL DEFAULT TRUE,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "categories" (
    id smallserial,
    label text NOT NULL UNIQUE,
    enabled boolean NOT NULL DEFAULT TRUE,
    PRIMARY KEY (id)
);

-- the sizes seem like enough for the small set of MIME types we expect to encounter
CREATE TABLE IF NOT EXISTS "mime_types" (
       id smallserial,
       essence VARCHAR(50) UNIQUE,
       PRIMARY KEY (id)
       );

CREATE TABLE IF NOT EXISTS "audio_formats" (
       id smallserial,
       container VARCHAR(100) NOT NULL,
       codec VARCHAR(100) NOT NULL,
       extension VARCHAR(100) NOT NULL,
       mime_type_id SMALLINT NOT NULL REFERENCES "mime_types" (id),
       UNIQUE (container, codec),
       PRIMARY KEY (id)
       );

-- TODO under the GDPR, is it okay to store the timestamps, parent, and category when deleted?
CREATE TABLE IF NOT EXISTS "recordings" (
    id uuid,
    created_at timestamp with time zone NOT NULL DEFAULT NOW(),
    updated_at timestamp with time zone NOT NULL DEFAULT NOW(),
    deleted_at timestamp with time zone,
    url TEXT,
    mime_type_id SMALLINT REFERENCES "mime_types" (id),
    parent_id uuid REFERENCES "recordings" (id),
    category_id smallint NOT NULL REFERENCES "categories" (id),
    name text,
    age_id smallint REFERENCES "ages" (id),
    gender_id smallint REFERENCES "genders" (id),
    location text,
    occupation TEXT,
    CONSTRAINT recordings_deleted_or_has_name CHECK ("deleted_at" IS NOT NULL OR "name" IS NOT NULL),
    CONSTRAINT recordings_id_is_not_parent_id CHECK ("id" <> "parent_id"),
    CONSTRAINT recordings_primary_key PRIMARY KEY (ID),
    CONSTRAINT recordings_url_is_unique UNIQUE (url),
    CONSTRAINT recordings_url_has_mime_type CHECK ("url" IS NULL OR "mime_type_id" IS NOT NULL),
    CONSTRAINT recordings_name UNIQUE (NAME)
);

CREATE INDEX "parent_index" ON "recordings" ("parent_id") WHERE "parent_id" IS NOT NULL;

CREATE TABLE IF NOT EXISTS "recording_tokens" (
   id uuid,
   parent_id uuid NOT NULL REFERENCES "recordings" (id),
   start TIMESTAMP WITH TIME ZONE,
   PRIMARY KEY (id)
);
