-- Migration 13 (20/11/2026) : les offres des sites retirées de la liste (la croix « Retirer de ma liste » dans
-- l'onglet Offres, « Où chercher »). Une offre retirée ne revient plus dans les recherches suivantes : elle est
-- reconnue par son site et son identifiant chez lui (sinon son adresse sans les paramètres de suivi). Son adresse, son
-- titre et la date du retrait servent à « Revoir les offres retirées ».

CREATE TABLE offres_retirees (
    id              INTEGER PRIMARY KEY,
    site            TEXT NOT NULL,
    id_site         TEXT NOT NULL,
    url             TEXT NOT NULL DEFAULT '',
    titre           TEXT NOT NULL DEFAULT '',
    retiree_le      TEXT NOT NULL,
    UNIQUE (site, id_site)
);
