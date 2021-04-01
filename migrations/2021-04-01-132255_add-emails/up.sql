CREATE TABLE "recording_management" (
       id uuid PRIMARY KEY,
       recording_id uuid UNIQUE NOT NULL REFERENCES "recordings" ("id"),
       email text
);
