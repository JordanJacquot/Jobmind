-- Migration 17 (26/11/2026) : ce qu'un audit a trouvé. Rien n'est retiré, aucune ligne ne change.
--
-- 1. Les candidatures d'une offre se cherchaient sans index (`candidatures.offre_id`) : toute la table était relue à
--    chaque offre montrée.
CREATE INDEX candidatures_offre ON candidatures (offre_id);

-- 2. Ce que Claude a trouvé dans une offre est maintenant gardé avec sa lecture, même quand les règles avaient déjà
--    une valeur à cet endroit (elle passe devant celle de Claude). Si une version suivante des règles ne trouve plus
--    rien, la réponse de Claude gardée ici est remise, sans le lui redemander (jobmind-storage, lecture::relire).
--    `comprise` : 1 quand ces colonnes disent ce que la réponse contenait. Une lecture d'avant cette migration reste à
--    0 : sa réponse (`reponse`) est comprise de nouveau par l'appli, une seule fois (lecture::a_recomprendre).
ALTER TABLE lectures_ia ADD COLUMN diplome TEXT;
ALTER TABLE lectures_ia ADD COLUMN diplome_preuve TEXT;
ALTER TABLE lectures_ia ADD COLUMN experience TEXT;
ALTER TABLE lectures_ia ADD COLUMN experience_preuve TEXT;
ALTER TABLE lectures_ia ADD COLUMN comprise INTEGER NOT NULL DEFAULT 0 CHECK (comprise IN (0, 1));
