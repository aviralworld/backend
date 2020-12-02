CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS "ages" (
    id smallserial,
    label character varying(50) NOT NULL UNIQUE,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "genders" (
    id smallserial,
    label character varying(100) NOT NULL UNIQUE,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "categories" (
    id smallserial,
    label text NOT NULL UNIQUE,
    PRIMARY KEY (id)
);

-- the sizes seem like enough for the small set of MIME types we expect to encounter
CREATE TABLE IF NOT EXISTS "mime_types" (
       id smallserial,
       essence VARCHAR(50) UNIQUE,
       container VARCHAR(100),
       codec VARCHAR(100),
       extension VARCHAR(100),
       UNIQUE (container, codec),
       PRIMARY KEY (id)
       );

-- TODO under the GDPR, is it okay to store the timestamps, parent, and category when deleted?
CREATE TABLE IF NOT EXISTS "recordings" (
    id uuid NOT NULL,
    url TEXT,
    mime_type_id SMALLINT REFERENCES "mime_types" (id),
    created_at timestamp with time zone NOT NULL DEFAULT NOW(),
    updated_at timestamp with time zone NOT NULL DEFAULT NOW(),
    deleted_at timestamp with time zone,
    category_id smallint NOT NULL REFERENCES "categories" (id),
    parent_id uuid REFERENCES "recordings" (id),
    name text,
    age_id smallint REFERENCES "ages" (id),
    gender_id smallint REFERENCES "genders" (id),
    location text,
    occupation TEXT,
    CONSTRAINT recordings_deleted_or_has_name CHECK ("deleted_at" IS NOT NULL OR "name" IS NOT NULL),
    CONSTRAINT recordings_id_is_not_parent_id CHECK ("id" <> "parent_id"),
    CONSTRAINT recordings_primary_key PRIMARY KEY (ID),
    CONSTRAINT recordings_url_has_mime_type CHECK ("url" IS NULL OR "mime_type_id" IS NOT NULL),
    CONSTRAINT recordings_name UNIQUE (NAME)
);

CREATE INDEX "parent_index" ON "recordings" ("parent_id") WHERE "parent_id" IS NOT NULL;
