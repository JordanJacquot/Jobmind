-- Migration 6 (étape 6 du plan, 19/11/2026) : la lecture et le tri. Le diplôme et l'expérience lus dans le texte de
-- chaque offre (colonnes diplome, diplome_lu_par, experience, experience_lue_par de la migration 1) gardent leur preuve
-- (le bout de phrase de l'offre, tel quel) et leur confiance ; le permis et le CACES demandés ; la version des règles
-- qui l'a lue (une version plus récente la relit) ; la note de pertinence (pertinence, raison, hors_sujet de la
-- migration 1) garde l'empreinte de ce qui l'a faite (« Ce que je cherche » et le CV : une autre empreinte, et elle est
-- refaite) ; « Pas hors sujet », dit par Jordan ; et chaque lecture par Claude, une seule par offre, avec son coût.

ALTER TABLE offres ADD COLUMN diplome_preuve TEXT;
ALTER TABLE offres ADD COLUMN diplome_confiance TEXT CHECK (diplome_confiance IN ('forte', 'moyenne'));
ALTER TABLE offres ADD COLUMN experience_preuve TEXT;
ALTER TABLE offres ADD COLUMN experience_confiance TEXT CHECK (experience_confiance IN ('forte', 'moyenne'));
ALTER TABLE offres ADD COLUMN autres_demandes TEXT;
ALTER TABLE offres ADD COLUMN lecture_version INTEGER;
ALTER TABLE offres ADD COLUMN lue_par_ia INTEGER NOT NULL DEFAULT 0 CHECK (lue_par_ia IN (0, 1));
ALTER TABLE offres ADD COLUMN pertinence_signature TEXT;
ALTER TABLE offres ADD COLUMN pas_hors_sujet INTEGER NOT NULL DEFAULT 0 CHECK (pas_hors_sujet IN (0, 1));
CREATE INDEX offres_lecture ON offres (lecture_version);

-- Chaque lecture d'une offre par Claude : la réponse gardée (pas de deuxième appel pour la même offre), ce qu'elle a
-- donné (« ok », « citation_inventee » : une citation absente de l'offre, rejetée ; « illisible »), les jetons et le
-- coût en millionièmes de dollar (le plafond du jour se compte ici)
CREATE TABLE lectures_ia (
    id                  INTEGER PRIMARY KEY,
    offre_id            INTEGER NOT NULL REFERENCES offres (id) ON DELETE CASCADE,
    lue_le              TEXT NOT NULL,
    modele              TEXT NOT NULL,
    etat                TEXT NOT NULL CHECK (etat IN ('ok', 'citation_inventee', 'illisible')),
    jetons_entree       INTEGER NOT NULL DEFAULT 0,
    jetons_sortie       INTEGER NOT NULL DEFAULT 0,
    cout_millioniemes   INTEGER NOT NULL DEFAULT 0,
    reponse             TEXT
);
CREATE INDEX lectures_ia_jour ON lectures_ia (lue_le);
CREATE INDEX lectures_ia_offre ON lectures_ia (offre_id);
