UPDATE "recording_tokens" SET start = NOW() WHERE id = $1 AND start IS NULL RETURNING "parent_id";
