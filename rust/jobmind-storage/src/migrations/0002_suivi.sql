-- Migration 2 (étape 2 du plan, 18/11/2026) : le suivi des candidatures à la main. La migration 1 ne change jamais :
-- les colonnes qui manquaient pour une candidature tapée à la main sont ajoutées ici, avec les bulles près de
-- l'horloge (une seule par événement, pour toujours).

-- Par où la candidature est partie (jobmind-core, Moyen::code), où l'offre a été vue, son lien, ce qui a été envoyé,
-- le CV utilisé, le jour du dernier changement de statut, et l'entretien (son moment, son lieu)
ALTER TABLE candidatures ADD COLUMN moyen TEXT NOT NULL DEFAULT 'autre'
    CHECK (moyen IN ('mail', 'site', 'main_propre', 'autre'));
ALTER TABLE candidatures ADD COLUMN source TEXT NOT NULL DEFAULT '';
ALTER TABLE candidatures ADD COLUMN lien TEXT NOT NULL DEFAULT '';
ALTER TABLE candidatures ADD COLUMN envoye TEXT NOT NULL DEFAULT '';
ALTER TABLE candidatures ADD COLUMN cv TEXT NOT NULL DEFAULT '';
ALTER TABLE candidatures ADD COLUMN statut_le TEXT;
ALTER TABLE candidatures ADD COLUMN entretien_le TEXT;
ALTER TABLE candidatures ADD COLUMN entretien_lieu TEXT NOT NULL DEFAULT '';
CREATE INDEX evenements_quand ON evenements (candidature_id, quand);

-- Les bulles demandées à Mindset (notification.demande) : en attente, puis envoyées. Un clic sur une bulle envoyée
-- ouvre ce qu'elle annonçait (une candidature, ou la liste si elle en regroupe plusieurs).
CREATE TABLE bulles (
    id              TEXT PRIMARY KEY CHECK (length(id) = 12),
    titre           TEXT NOT NULL,
    texte           TEXT NOT NULL,
    candidature_id  INTEGER,
    etat            TEXT NOT NULL CHECK (etat IN ('attente', 'envoyee')),
    cree_le         TEXT NOT NULL,
    envoyee_le      TEXT
);

-- Chaque événement déjà annoncé (ou déjà vu par Jordan) : jamais deux bulles pour la même clé
-- (« relance:<candidature>:<jour d'envoi> », « entretien:<candidature>:<moment> »)
CREATE TABLE bulles_cles (
    cle         TEXT PRIMARY KEY,
    bulle_id    TEXT,
    vue_le      TEXT NOT NULL
);
