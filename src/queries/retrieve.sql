SELECT recordings.id,
       recordings.url,
       recordings.created_at,
       recordings.updated_at,
       recordings.unlisted,
       recordings.parent_id,
       recordings.name,
       recordings.location,
       recordings.occupation,
       categories.label AS category,
       ages.label AS age,
       genders.label AS gender
FROM recordings
LEFT JOIN categories ON categories.id = recordings.category_id
LEFT JOIN ages ON ages.id = recordings.age_id
LEFT JOIN genders ON genders.id = recordings.gender_id
WHERE recordings.id = $1
LIMIT 1;
