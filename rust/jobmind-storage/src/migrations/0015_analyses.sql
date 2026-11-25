-- Migration 15 (25/11/2026) : les offres des sites déjà analysées par Jobmind, gardées sur le disque. Une offre lue
-- en coulisse (lecture automatique ou clic) est écrite ici : la fois d'après, sa carte arrive tout de suite et elle
-- s'ouvre sans recharger la page du site. Elle est reconnue par son site et son identifiant chez lui, comme une offre
-- retirée (migration 13). `format` est le numéro du format de `contenu` (la page lue, en JSON) : une ligne d'un autre
-- format est ignorée, l'offre est relue (0 : une ligne sans contenu, posée seulement pour retenir `vue_le`).
-- `vue_le` : le moment où la personne a ouvert l'offre elle-même (« Déjà vue ») ; vide tant qu'elle ne l'a pas
-- ouverte. Les lignes de plus de 30 jours sont effacées par l'appli.

CREATE TABLE offres_analysees (
    id              INTEGER PRIMARY KEY,
    site            TEXT NOT NULL,
    id_site         TEXT NOT NULL,
    url             TEXT NOT NULL DEFAULT '',
    format          INTEGER NOT NULL DEFAULT 0,
    contenu         TEXT NOT NULL DEFAULT '',
    analysee_le     TEXT NOT NULL,
    vue_le          TEXT NOT NULL DEFAULT '',
    UNIQUE (site, id_site)
);
