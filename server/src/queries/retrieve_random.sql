SELECT "recordings"."id",
       "recordings"."name",
       "recordings"."location"
FROM "recordings"
WHERE "recordings"."deleted_at" IS NULL
ORDER BY RANDOM()
LIMIT $1;
