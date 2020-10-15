SELECT "recordings"."id",
       "recordings"."url",
       "recordings"."mime_type_id",
       "recordings"."created_at",
       "recordings"."updated_at",
       "recordings"."deleted_at",
       "recordings"."category_id",
       "recordings"."unlisted",
       "recordings"."parent_id",
       "recordings"."name",
       "recordings"."age_id",
       "recordings"."gender_id",
       "recordings"."location",
       "recordings"."occupation",
       "categories"."label" AS "category",
       "ages"."label" AS "age",
       "genders"."label" AS "gender",
       "mime_types"."essence" AS "mime_type"
FROM "recordings"
LEFT JOIN "categories" ON "categories"."id" = "recordings"."category_id"
LEFT JOIN "ages" ON "ages"."id" = "recordings"."age_id"
LEFT JOIN "genders" ON "genders"."id" = "recordings"."gender_id"
LEFT JOIN "mime_types" ON "mime_types"."id" = "recordings"."mime_type_id"
WHERE "recordings"."id" = $1
LIMIT 1;
