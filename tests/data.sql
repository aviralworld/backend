SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;
SET SCHEMA 'public';

-- clear out existing data
TRUNCATE recordings CASCADE;
TRUNCATE ages CASCADE;
TRUNCATE categories CASCADE;
TRUNCATE genders CASCADE;
TRUNCATE mime_types CASCADE;

INSERT INTO ages (id, label, enabled) VALUES (1, 'Age 1', TRUE);
INSERT INTO ages (id, label, enabled) VALUES (2, 'Age B', TRUE);
INSERT INTO ages (id, label, enabled) VALUES (3, 'Age three', TRUE);
INSERT INTO ages (id, label, enabled) VALUES (4, 'Fooled ya! This is Age 2', TRUE);
INSERT INTO ages (id, label, enabled) VALUES (20, 'This age doesn''t exist', FALSE);

INSERT INTO categories (id, label, enabled) VALUES (6, 'This is a category', TRUE);
INSERT INTO categories (id, label, enabled) VALUES (2, 'Some other category', TRUE);
INSERT INTO categories (id, label, enabled) VALUES (5, 'This one is disabled', FALSE);
INSERT INTO categories (id, label, enabled) VALUES (7, 'This category has
  some newlines
and spaces in it', TRUE);
INSERT INTO categories (id, label, enabled) VALUES (3, 'यह हिन्दी है ।', TRUE);
INSERT INTO categories (id, label, enabled) VALUES (4, 'Ceci n’est pas une catégorie', TRUE);
INSERT INTO categories (id, label, enabled) VALUES (1, 'یہ بھی ہے', TRUE);

INSERT INTO genders (id, label, enabled) VALUES (1, 'One of the genders', TRUE);
INSERT INTO genders (id, label, enabled) VALUES (2, 'Some other genders', TRUE);
INSERT INTO genders (id, label, enabled) VALUES (3, 'No gender specified', TRUE);
INSERT INTO genders (id, label, enabled) VALUES (50, 'None of the above', TRUE);
INSERT INTO genders (id, label, enabled) VALUES (5, 'This is a bogus gender', FALSE);

INSERT INTO mime_types (essence, container, codec, extension) VALUES ('audio/ogg; codec=opus', 'ogg', 'opus', 'ogg');
INSERT INTO mime_types (essence, container, codec, extension) VALUES ('audio/ogg', 'ogg', 'vorbis', 'ogg');

--
-- TOC entry 2980 (class 0 OID 0)
-- Dependencies: 205
-- Name: ages_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('ages_id_seq', 4, TRUE);


--
-- TOC entry 2981 (class 0 OID 0)
-- Dependencies: 209
-- Name: categories_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('categories_id_seq', 6, TRUE);


--
-- TOC entry 2982 (class 0 OID 0)
-- Dependencies: 207
-- Name: genders_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('genders_id_seq', 4, TRUE);


--
-- TOC entry 2983 (class 0 OID 0)
-- Dependencies: 203
-- Name: movine_migrations_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('movine_migrations_id_seq', 2, TRUE);
