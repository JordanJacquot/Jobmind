-- Migration 11 (étape 11 du plan, 19/11/2026) : l'envoi sur le site de l'offre. Chaque envoi sur un site tenté est
-- noté AVANT le clic de Jobmind sur le bouton d'envoi du site (« en_cours »), avec le contenu exact de chaque champ du
-- formulaire relu sur la page (JSON : le libellé du site, ce qui y est, d'où ça vient), puis « envoye » (la page de
-- confirmation reconnue, sa phrase gardée), « refuse » (le site a refusé : rien n'est parti), « incertain » (Jobmind ne
-- sait pas : Jordan regarde et dit), « pas_parti », « confirme » (Jordan dit qu'elle est partie, LinkedIn compris).
-- Un envoi resté « en_cours » quand Jobmind s'est arrêtée devient « incertain » au démarrage.
CREATE TABLE envois_site (
    id              INTEGER PRIMARY KEY,
    offre_id        INTEGER REFERENCES offres (id) ON DELETE SET NULL,
    preparation_id  INTEGER REFERENCES preparations (id) ON DELETE SET NULL,
    cv_id           INTEGER REFERENCES cv (id) ON DELETE SET NULL,
    site            TEXT NOT NULL,
    adresse         TEXT NOT NULL,
    champs          TEXT NOT NULL,
    lettre          TEXT NOT NULL DEFAULT '',
    piece_jointe    TEXT,
    tente_le        TEXT NOT NULL,
    fini_le         TEXT,
    etat            TEXT NOT NULL DEFAULT 'en_cours'
                        CHECK (etat IN ('en_cours', 'envoye', 'refuse', 'incertain', 'pas_parti', 'confirme')),
    preuve          TEXT,
    erreur          TEXT,
    envoi_id        INTEGER REFERENCES envois (id) ON DELETE SET NULL
);
CREATE INDEX envois_site_offre ON envois_site (offre_id);
CREATE INDEX envois_site_quand ON envois_site (tente_le);

-- Un envoi sur le site parti, dans la table envois (moyen « site ») : le site, la page, la phrase de confirmation (ou
-- « dit par Jordan »), et qui l'a dit parti (la page du site, ou Jordan). Le contenu exact de chaque champ va dans
-- `envois.recapitulatif` (JSON, colonne de la migration 1).
ALTER TABLE envois ADD COLUMN site TEXT;
ALTER TABLE envois ADD COLUMN adresse_site TEXT;
ALTER TABLE envois ADD COLUMN preuve TEXT;
ALTER TABLE envois ADD COLUMN confirme_site TEXT CHECK (confirme_site IN ('page', 'jordan'));

-- Les coordonnées que Jordan tape une fois pour les formulaires des sites (nom, mail et téléphone existent depuis la
-- migration 1) : son prénom.
ALTER TABLE profil ADD COLUMN prenom TEXT NOT NULL DEFAULT '';
