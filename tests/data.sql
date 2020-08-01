--
-- PostgreSQL database dump
--

-- Dumped from database version 12.3 (Debian 12.3-1.pgdg100+1)
-- Dumped by pg_dump version 12.3

-- Started on 2020-08-01 19:13:37

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

--
-- TOC entry 2969 (class 0 OID 16411)
-- Dependencies: 206
-- Data for Name: ages; Type: TABLE DATA; Schema: public; Owner: postgres
--

TRUNCATE ages;

COPY ages (id, label) FROM stdin;
1	Age 1
2	Age B
3	Age three
4	Fooled ya! Age 2
\.


--
-- TOC entry 2973 (class 0 OID 16431)
-- Dependencies: 210
-- Data for Name: categories; Type: TABLE DATA; Schema: public; Owner: postgres
--

TRUNCATE categories;

COPY categories (id, label) FROM stdin;
1	This is a category
2	Some other category
3	This category has a\nnewline in it
4	यह हिन्दी में है ।
5	Ceci n’est pas une catégorie
6	یہ بھی تے
\.


--
-- TOC entry 2971 (class 0 OID 16421)
-- Dependencies: 208
-- Data for Name: genders; Type: TABLE DATA; Schema: public; Owner: postgres
--

TRUNCATE genders;

COPY genders (id, label) FROM stdin;
1	One of the genders
2	Some other gender
3	No gender specified
4	None of the above
\.


--
-- TOC entry 2974 (class 0 OID 16442)
-- Dependencies: 211
-- Data for Name: recordings; Type: TABLE DATA; Schema: public; Owner: postgres
--

TRUNCATE recordings;


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


-- Completed on 2020-08-01 19:13:37

--
-- PostgreSQL database dump complete
--

