-- Migration 8 (étape 8 du plan, 19/11/2026) : la candidature préparée par Claude. Chaque préparation est une version
-- (1, 2, 3... pour une offre), rattachée à l'offre (la première de son groupe de doublons) et, si elle existe, à la
-- candidature du suivi. Elle garde la réponse de Claude telle quelle (reponse_brute, le JSON rendu) et la lettre qui en
-- est tirée (lettre_brute, jamais changée : l'étape 9 la réécrira dans le style de Jordan), à côté de la lettre que
-- Jordan modifie à la main (lettre). Les champs de l'étape 9 (lettre_style, style_score, style_intensite, style_le)
-- restent vides ici. La table contenus_generes de la migration 1 n'est pas utilisée (une préparation est faite de
-- phrases vérifiées, de questions et de conseils, gardés en JSON).

CREATE TABLE preparations (
    id                  INTEGER PRIMARY KEY,
    offre_id            INTEGER NOT NULL REFERENCES offres (id) ON DELETE CASCADE,
    candidature_id      INTEGER REFERENCES candidatures (id) ON DELETE SET NULL,
    numero              INTEGER NOT NULL CHECK (numero >= 1),
    cree_le             TEXT NOT NULL,
    modele              TEXT NOT NULL,
    -- La consigne de Jordan pour cette version (« plus court »), et la version d'où elle part
    consigne            TEXT,
    depuis_id           INTEGER REFERENCES preparations (id) ON DELETE SET NULL,
    -- Le CV donné à Claude
    cv_id               INTEGER REFERENCES cv (id) ON DELETE SET NULL,
    -- Ce que Claude a rendu, tel quel (le JSON), et la lettre qui en est tirée, jamais changée
    reponse_brute       TEXT NOT NULL,
    lettre_brute        TEXT NOT NULL,
    -- La lettre de Jordan (au départ la lettre brute), et quand il l'a changée
    lettre              TEXT NOT NULL,
    lettre_modifiee_le  TEXT,
    objet               TEXT NOT NULL DEFAULT '',
    -- Les phrases à vérifier (JSON : [{ "texte", "raison", "code", "vue" }]), les questions et leurs réponses, les
    -- conseils pour le CV (JSON)
    a_verifier          TEXT NOT NULL DEFAULT '[]',
    questions           TEXT NOT NULL DEFAULT '[]',
    conseils            TEXT NOT NULL DEFAULT '[]',
    jetons_entree       INTEGER NOT NULL DEFAULT 0,
    jetons_sortie       INTEGER NOT NULL DEFAULT 0,
    cout_millioniemes   INTEGER NOT NULL DEFAULT 0,
    -- Étape 9 (le style de Jordan), pas encore remplis
    lettre_style        TEXT,
    style_score         INTEGER CHECK (style_score BETWEEN 0 AND 100),
    style_intensite     TEXT CHECK (style_intensite IN ('legere', 'moyenne', 'forte', 'aucune')),
    style_le            TEXT,
    UNIQUE (offre_id, numero)
);
CREATE INDEX preparations_offre ON preparations (offre_id);
CREATE INDEX preparations_candidature ON preparations (candidature_id);

-- Chaque demande de candidature faite à Claude, réussie ou non (le plafond du jour se compte ici) : son état
-- (« en_cours » le temps de la demande, puis « ok » ou le code de l'erreur), les jetons, le coût, et pour une réponse
-- illisible la réponse telle quelle (rien n'est perdu, le journal dit quoi)
CREATE TABLE appels_preparation (
    id                  INTEGER PRIMARY KEY,
    offre_id            INTEGER REFERENCES offres (id) ON DELETE SET NULL,
    demande_le          TEXT NOT NULL,
    etat                TEXT NOT NULL,
    jetons_entree       INTEGER NOT NULL DEFAULT 0,
    jetons_sortie       INTEGER NOT NULL DEFAULT 0,
    cout_millioniemes   INTEGER NOT NULL DEFAULT 0,
    reponse             TEXT
);
CREATE INDEX appels_preparation_jour ON appels_preparation (demande_le);
