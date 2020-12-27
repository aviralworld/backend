UPDATE recordings SET url = $2, mime_type_id = $3, updated_at = NOW() WHERE id = $1;
