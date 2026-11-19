-- Migration 5 (étape 5 du plan, 18/11/2026) : « Ajouter à Jobmind », les offres lues dans le panneau des sites. Elles
-- entrent dans la même table que celles d'Adzuna et de Jooble (une source par site), passent par la même fusion des
-- doublons, et chaque clic note ce que la page montrait (le nombre de candidats et sa date) dans offres_vues
-- (migration 1).

INSERT INTO sources (code, nom, type, cree_le) VALUES ('linkedin', 'LinkedIn', 'page', '2026-11-18T00:00:00');
INSERT INTO sources (code, nom, type, cree_le) VALUES ('indeed', 'Indeed', 'page', '2026-11-18T00:00:00');
INSERT INTO sources (code, nom, type, cree_le) VALUES ('wttj', 'Welcome to the Jungle', 'page', '2026-11-18T00:00:00');
INSERT INTO sources (code, nom, type, cree_le) VALUES ('francetravail', 'France Travail', 'page', '2026-11-18T00:00:00');
INSERT INTO sources (code, nom, type, cree_le) VALUES ('site', 'Un autre site', 'page', '2026-11-18T00:00:00');

-- La dernière vue d'une offre, lue vite
CREATE INDEX offres_vues_offre_date ON offres_vues (offre_id, vue_le);
