-- Migration 9 (étape 9 du plan, 19/11/2026) : le style de Jordan. Les tables regles_style et parametres viennent de la
-- migration 1, qui ne change jamais : regles_style garde ce que Jordan change aux règles (fixe = 1 : une règle de
-- départ coupée ou remise, motif = son code ; fixe = 0 : une expression à lui, avec son remplacement), parametres la
-- longueur de phrase permise (style_phrase_max) et l'intensité par défaut (style_intensite).
CREATE UNIQUE INDEX regles_style_fixes ON regles_style (motif) WHERE fixe = 1;

-- La version dans le style de Jordan (lettre_style, style_score, style_intensite, style_le : prévus par la migration 8)
-- gagne ses phrases à vérifier, ce que Jordan doit savoir de la réécriture (une phrase remise comme avant), la réponse
-- de Claude telle quelle et son coût. lettre_choisie : la lettre que Jordan garde (brute : celle de Claude ; style :
-- dans son style ; moi : la sienne, modifiée à la main ; rien : la sienne, comme à l'étape 8).
ALTER TABLE preparations ADD COLUMN style_a_verifier TEXT NOT NULL DEFAULT '[]';
ALTER TABLE preparations ADD COLUMN style_notes TEXT NOT NULL DEFAULT '[]';
ALTER TABLE preparations ADD COLUMN style_reponse TEXT;
ALTER TABLE preparations ADD COLUMN style_cout_millioniemes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preparations ADD COLUMN lettre_choisie TEXT CHECK (lettre_choisie IN ('brute', 'style', 'moi'));
