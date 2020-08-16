-- PostgreSQL returns the SQLSTATE code 23505 in case of a unique key constraint violation.

INSERT INTO recordings (id, url, category_id, parent_id, unlisted, name, location, occupation, age_id, gender_id)
VALUES (uuid_generate_v4(), $1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id,
                                                                          created_at,
                                                                          updated_at;
