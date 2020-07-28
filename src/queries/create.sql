-- PostgreSQL returns the SQLSTATE code 23505 in case of a unique key constraint violation.  
INSERT INTO recordings (id, age, gender, location, name, occupation, url, created, parent, category, unlisted) VALUES (uuid_generate_v4(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id;
