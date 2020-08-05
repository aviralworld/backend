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

INSERT INTO ages (id, label) VALUES (1, 'Age 1');
INSERT INTO ages (id, label) VALUES (2, 'Age B');
INSERT INTO ages (id, label) VALUES (3, 'Age three');
INSERT INTO ages (id, label) VALUES (4, 'Fooled ya! This is Age 2');

INSERT INTO categories (id, label) VALUES (1, 'This is a category');
INSERT INTO categories (id, label) VALUES (2, 'Some other category');
INSERT INTO categories (id, label) VALUES (3, 'This category has
  some newlines
and spaces in it');
INSERT INTO categories (id, label) VALUES (4, 'यह हिन्दी है ।');
INSERT INTO categories (id, label) VALUES (5, 'Ceci n’est pas une catégorie');
INSERT INTO categories (id, label) VALUES (6, 'یہ بھی ہے');

INSERT INTO genders (id, label) VALUES (1, 'One of the genders');
INSERT INTO genders (id, label) VALUES (2, 'Some other genders');
INSERT INTO genders (id, label) VALUES (3, 'No gender specified');
INSERT INTO genders (id, label) VALUES (4, 'None of the above');

--
-- TOC entry 2980 (class 0 OID 0)
-- Dependencies: 205
-- Name: ages_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('ages_id_seq', 4, true);


--
-- TOC entry 2981 (class 0 OID 0)
-- Dependencies: 209
-- Name: categories_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('categories_id_seq', 6, true);


--
-- TOC entry 2982 (class 0 OID 0)
-- Dependencies: 207
-- Name: genders_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('genders_id_seq', 4, true);


--
-- TOC entry 2983 (class 0 OID 0)
-- Dependencies: 203
-- Name: movine_migrations_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('movine_migrations_id_seq', 2, true);
