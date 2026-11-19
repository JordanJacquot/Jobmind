-- Migration 12 (étape 12 du plan, 19/11/2026) : le tableau de bord, la sauvegarde et le rafraîchissement automatique
-- des recherches. Chaque rafraîchissement automatique est noté (quand, quelle recherche, combien de nouvelles offres,
-- ce qui n'a pas marché) ; une bulle peut ouvrir un autre onglet que Candidatures (« 5 nouvelles offres » : Offres).
-- Les réglages (le rythme, la dernière fois, les nouvelles offres pas encore vues) vont dans parametres.

CREATE TABLE rafraichissements (
    id              INTEGER PRIMARY KEY,
    fait_le         TEXT NOT NULL,
    recherche_id    INTEGER REFERENCES recherches (id) ON DELETE SET NULL,
    trouvees        INTEGER NOT NULL DEFAULT 0,
    nouvelles       INTEGER NOT NULL DEFAULT 0,
    etat            TEXT NOT NULL CHECK (etat IN ('ok', 'erreur', 'plafond')),
    message         TEXT NOT NULL DEFAULT ''
);
CREATE INDEX rafraichissements_fait ON rafraichissements (fait_le);

ALTER TABLE bulles ADD COLUMN onglet TEXT CHECK (onglet IN ('candidatures', 'offres'));