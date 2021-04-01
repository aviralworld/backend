INSERT INTO "recording_management" ("id", "recording_id", "email") VALUES (uuid_generate_v4(), $1, $2) RETURNING "id";
