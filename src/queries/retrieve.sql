SELECT age, gender, location, name, occupation, url, created, parent, category, unlisted FROM recordings WHERE id = $1 LIMIT 1;
