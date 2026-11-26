-- Migration 16 (25/11/2026) : les corrections de la personne à « Ce que Jobmind a compris de ton CV » (Mon profil).
-- La fiche elle-même n'est pas gardée : elle est refaite d'après le texte du CV principal (jobmind_core::parcours).
-- Ici, seulement ce que la personne a changé : une ligne retirée (`geste` = 'retrait') ou ajoutée ('ajout'), dans une
-- des quatre listes (`liste` : metiers, savoirs, diplomes, permis). `cle` est la ligne remise au propre, pour la
-- reconnaître d'une lecture à l'autre ; `texte` est ce que la personne a tapé (vide pour un retrait). Les corrections
-- suivent la personne : elles s'appliquent à la fiche du CV principal, quel qu'il soit.

CREATE TABLE parcours_corrections (
    id              INTEGER PRIMARY KEY,
    liste           TEXT NOT NULL,
    cle             TEXT NOT NULL,
    geste           TEXT NOT NULL,
    texte           TEXT NOT NULL DEFAULT '',
    change_le       TEXT NOT NULL,
    UNIQUE (liste, cle)
);
