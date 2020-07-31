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

CREATE TABLE IF NOT EXISTS "recordings" (
    id uuid NOT NULL,
    url text,
    created_at timestamp with time zone NOT NULL DEFAULT NOW(),
    updated_at timestamp with time zone NOT NULL DEFAULT NOW(),
    category smallint NOT NULL REFERENCES "categories" (id),
    unlisted boolean NOT NULL,
    name text NOT NULL,
    age smallint REFERENCES "ages" (id),
    gender smallint REFERENCES "genders" (id),
    location text,
    occupation text,
    PRIMARY KEY (id)
);
