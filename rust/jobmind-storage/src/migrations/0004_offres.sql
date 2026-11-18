-- Migration 4 (étape 4 du plan, 18/11/2026) : les offres d'Adzuna et de Jooble. Les tables offres, offres_fts (la
-- recherche dans tout le texte), sources et recherches viennent de la migration 1, qui ne change jamais : il manquait
-- les deux sources, ce qu'une recherche gardée sait de son lieu et de sa dernière fois, « Pas pour moi » et « déjà
-- ouverte » sur une offre, le jour où elle a été retrouvée, les recherches qui l'ont trouvée, et le compte des
-- demandes du jour à chaque source (pour rester très en dessous de leurs limites).

INSERT INTO sources (code, nom, type, cree_le) VALUES ('adzuna', 'Adzuna', 'api', '2026-11-18T00:00:00');
INSERT INTO sources (code, nom, type, cree_le) VALUES ('jooble', 'Jooble', 'api', '2026-11-18T00:00:00');

-- Une recherche gardée : son genre de lieu (jobmind-core, GenreLieu::code ; la ville est dans « lieu »), son
-- département ou sa région, et ce qu'a donné sa dernière fois (le nombre d'offres trouvées, la phrase montrée)
ALTER TABLE recherches ADD COLUMN genre_lieu TEXT NOT NULL DEFAULT 'ville'
    CHECK (genre_lieu IN ('ville', 'departement', 'region', 'partout'));
ALTER TABLE recherches ADD COLUMN departement TEXT NOT NULL DEFAULT '';
ALTER TABLE recherches ADD COLUMN region TEXT NOT NULL DEFAULT '';
ALTER TABLE recherches ADD COLUMN dernier_nombre INTEGER;
ALTER TABLE recherches ADD COLUMN dernier_message TEXT;

-- Une offre : « Pas pour moi » (cachée, jamais effacée), déjà ouverte par Jordan, le dernier jour où une source l'a
-- encore donnée. Les doublons gardent leur ligne (l'archive de chaque source) et pointent la première (doublon_de).
ALTER TABLE offres ADD COLUMN pas_pour_moi INTEGER NOT NULL DEFAULT 0 CHECK (pas_pour_moi IN (0, 1));
ALTER TABLE offres ADD COLUMN vue INTEGER NOT NULL DEFAULT 0 CHECK (vue IN (0, 1));
ALTER TABLE offres ADD COLUMN retrouvee_le TEXT;
CREATE INDEX offres_doublon ON offres (doublon_de);
CREATE INDEX offres_url ON offres (url);

-- Les recherches gardées qui ont trouvé chaque offre (le filtre « autour d'une ville » s'en sert pour les offres sans
-- position)
CREATE TABLE offres_recherches (
    offre_id        INTEGER NOT NULL REFERENCES offres (id) ON DELETE CASCADE,
    recherche_id    INTEGER NOT NULL REFERENCES recherches (id) ON DELETE CASCADE,
    trouvee_le      TEXT NOT NULL,
    PRIMARY KEY (offre_id, recherche_id)
);
CREATE INDEX offres_recherches_recherche ON offres_recherches (recherche_id);

-- Les demandes envoyées à chaque source, par jour (jamais plus que la limite que Jobmind se donne)
CREATE TABLE appels (
    jour    TEXT NOT NULL,
    source  TEXT NOT NULL,
    nombre  INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (jour, source)
);
