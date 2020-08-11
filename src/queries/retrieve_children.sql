SELECT "id", "name" FROM "recordings" WHERE "parent_id" = $1 AND "deleted_at" IS NULL AND "unlisted" = FALSE ORDER BY "created_at" ASC;
