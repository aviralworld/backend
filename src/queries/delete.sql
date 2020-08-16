-- TODO under the GDPR, is it okay to store the timestamps, parent, category, and children when deleted?
UPDATE "recordings" SET "deleted_at" = NOW(), "url" = NULL, "name" = NULL, "age_id" = NULL, "gender_id" = NULL, "location" = NULL, "occupation" = NULL WHERE "id" = $1;
