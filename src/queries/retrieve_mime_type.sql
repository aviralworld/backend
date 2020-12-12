SELECT "mime_types"."id",
       "mime_types"."essence",
       "audio_formats"."container",
       "audio_formats"."codec",
       "audio_formats"."extension"
FROM "audio_formats" INNER JOIN "mime_types"
ON "audio_formats"."mime_type_id" = "mime_types"."id"
WHERE "audio_formats"."container" = $1 AND "audio_formats"."codec" = $2
LIMIT 1;
