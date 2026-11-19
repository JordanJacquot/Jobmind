-- Migration 7 (étape 7 du plan, 19/11/2026) : la boîte Gmail. La table mails (migration 1) garde chaque mail lu qui
-- compte pour Jobmind : une alerte d'offres (seulement de quoi ne pas la relire : son objet, le nombre d'offres), une
-- réponse rangée dans le dossier d'une candidature, un mail « à ranger » que Jordan choisit, ou « rien à voir » (son
-- texte effacé, seule sa trace reste pour ne pas le relire). Le texte gardé est le texte brut nettoyé du mail (jamais
-- son HTML, jamais ses pièces jointes). Le statut proposé garde la phrase du mail qui le dit, qui l'a lu (les règles ou
-- Claude), et ce que Jordan en a fait. Où en est la relève (UIDVALIDITY, dernier UID) va dans parametres.

ALTER TABLE mails ADD COLUMN uid INTEGER;
ALTER TABLE mails ADD COLUMN de_nom TEXT NOT NULL DEFAULT '';
ALTER TABLE mails ADD COLUMN texte TEXT;
ALTER TABLE mails ADD COLUMN etat TEXT NOT NULL DEFAULT 'rangee'
    CHECK (etat IN ('alerte', 'rangee', 'a_ranger', 'rien'));
ALTER TABLE mails ADD COLUMN confiance TEXT CHECK (confiance IN ('sure', 'douteuse', 'jordan'));
ALTER TABLE mails ADD COLUMN suggestion_id INTEGER REFERENCES candidatures (id) ON DELETE SET NULL;
ALTER TABLE mails ADD COLUMN raison TEXT NOT NULL DEFAULT '';
ALTER TABLE mails ADD COLUMN statut_citation TEXT;
ALTER TABLE mails ADD COLUMN statut_par TEXT CHECK (statut_par IN ('regles', 'ia'));
ALTER TABLE mails ADD COLUMN proposition TEXT CHECK (proposition IN ('attente', 'acceptee', 'refusee'));
ALTER TABLE mails ADD COLUMN flou INTEGER NOT NULL DEFAULT 0 CHECK (flou IN (0, 1));
ALTER TABLE mails ADD COLUMN lu_ia INTEGER NOT NULL DEFAULT 0 CHECK (lu_ia IN (0, 1));
ALTER TABLE mails ADD COLUMN offres INTEGER;
ALTER TABLE mails ADD COLUMN pieces_jointes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE mails ADD COLUMN garde_le TEXT;
CREATE INDEX mails_candidature ON mails (candidature_id);
CREATE INDEX mails_etat ON mails (etat);

-- Une offre venue d'une alerte mail : le jour où l'alerte l'a apportée
ALTER TABLE offres ADD COLUMN alerte_mail_le TEXT;

-- Étape 10 (l'envoi par mail) : le Message-ID du mail envoyé, pour reconnaître la réponse (« Re: »)
ALTER TABLE envois ADD COLUMN message_id TEXT;
