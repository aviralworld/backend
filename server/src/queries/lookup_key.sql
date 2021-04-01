SELECT "recordings"."id" FROM "recording_management" INNER JOIN "recordings" ON "recording_management"."recording_id" = "recordings"."id" WHERE "recording_management"."id" = $1 LIMIT 1;
