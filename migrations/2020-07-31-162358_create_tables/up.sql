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

-- TODO under the GDPR, is it okay to store the timestamps, parent, and category when deleted?
CREATE TABLE IF NOT EXISTS "recordings" (
    id uuid NOT NULL,
    url text,
    created_at timestamp with time zone NOT NULL DEFAULT NOW(),
    updated_at timestamp with time zone NOT NULL DEFAULT NOW(),
    deleted_at timestamp with time zone,
    category_id smallint NOT NULL REFERENCES "categories" (id),
    unlisted boolean NOT NULL,
    parent_id uuid REFERENCES "recordings" (id),
    name text,
    age_id smallint REFERENCES "ages" (id),
    gender_id smallint REFERENCES "genders" (id),
    location text,
    occupation TEXT,
    CONSTRAINT recordings_deleted_or_has_name CHECK ("deleted_at" IS NOT NULL OR "name" IS NOT NULL),
    CONSTRAINT recordings_id_is_not_parent_id CHECK ("id" <> "parent_id"),
    CONSTRAINT recordings_primary_key PRIMARY KEY (id),
    CONSTRAINT recordings_name UNIQUE (NAME)
);
