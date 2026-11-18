-- Migration 3 (étape 3 du plan, 18/11/2026) : Mon profil. Les tables profil, cv et profil_style viennent de la
-- migration 1, qui ne change jamais : il manquait seulement ce que « Ce que je cherche » garde en plus du métier, de
-- la ville et du rayon, et un seul CV principal à la fois.

-- « Ce que je cherche » : le genre de lieu (jobmind-core, GenreLieu::code), le département ou la région, le
-- télétravail (Teletravail::code), « aucun diplôme requis », « débutant accepté », le salaire minimum (en euros bruts
-- par mois) et le moment où tout cela a été enregistré. Les postes vont dans « metiers » (un par ligne), les contrats
-- dans « contrats » (leurs codes, séparés par des virgules).
ALTER TABLE profil ADD COLUMN lieu TEXT NOT NULL DEFAULT 'ville'
    CHECK (lieu IN ('ville', 'departement', 'region', 'partout'));
ALTER TABLE profil ADD COLUMN departement TEXT NOT NULL DEFAULT '';
ALTER TABLE profil ADD COLUMN region TEXT NOT NULL DEFAULT '';
ALTER TABLE profil ADD COLUMN teletravail TEXT NOT NULL DEFAULT 'peu_importe'
    CHECK (teletravail IN ('peu_importe', 'possible', 'complet'));
ALTER TABLE profil ADD COLUMN sans_diplome INTEGER NOT NULL DEFAULT 0 CHECK (sans_diplome IN (0, 1));
ALTER TABLE profil ADD COLUMN debutant INTEGER NOT NULL DEFAULT 0 CHECK (debutant IN (0, 1));
ALTER TABLE profil ADD COLUMN salaire_min INTEGER CHECK (salaire_min IS NULL OR salaire_min BETWEEN 300 AND 20000);
ALTER TABLE profil ADD COLUMN recherche_le TEXT;

-- Les CV : un seul principal à la fois, le jour du dernier changement de son texte, la taille du fichier d'origine
-- (copié dans le dossier « cv » des données : il sera la pièce jointe des candidatures)
CREATE UNIQUE INDEX cv_un_principal ON cv (principal) WHERE principal = 1;
ALTER TABLE cv ADD COLUMN change_le TEXT;
ALTER TABLE cv ADD COLUMN taille INTEGER;
