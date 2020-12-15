INSERT INTO "recording_tokens" ("id", "parent_id") VALUES (uuid_generate_v4(), $1) RETURNING "id";
