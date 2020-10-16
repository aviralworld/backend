SELECT "mime_types"."id",
       "mime_types"."essence",
       "mime_types"."container",
       "mime_types"."codec",
       "mime_types"."extension"
FROM "mime_types"
WHERE "container" = $1 AND "codec" = $2
LIMIT 1;
