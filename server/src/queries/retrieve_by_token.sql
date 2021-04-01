SELECT "recordings"."id",
       "recordings"."url",
       "recordings"."mime_type_id",
       "recordings"."created_at",
       "recordings"."updated_at",
       "recordings"."deleted_at",
       "recordings"."category_id",
       "recordings"."parent_id",
       "recordings"."name",
       "recordings"."age_id",
       "recordings"."gender_id",
       "recordings"."location",
       "recordings"."occupation",
       "categories"."label" AS "category",
       "categories"."description" AS "category_description",
       "ages"."label" AS "age",
       "genders"."label" AS "gender",
       "mime_types"."essence" AS "mime_type"
FROM "recording_management"
LEFT JOIN "recordings" ON "recording_management"."recording_id" = "recordings"."id"
LEFT JOIN "categories" ON "categories"."id" = "recordings"."category_id"
LEFT JOIN "ages" ON "ages"."id" = "recordings"."age_id"
LEFT JOIN "genders" ON "genders"."id" = "recordings"."gender_id"
LEFT JOIN "mime_types" ON "mime_types"."id" = "recordings"."mime_type_id"
WHERE "recording_management"."id" = $1
LIMIT 1;
