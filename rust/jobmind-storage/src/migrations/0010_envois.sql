-- Migration 10 (étape 10 du plan, 19/11/2026) : l'envoi par mail. La table envois (migration 1, Message-ID ajouté par
-- la migration 7) garde chaque mail PARTI, avec le contenu exact : l'expéditeur, le nom et la taille de la pièce
-- jointe, l'offre et la version de la candidature préparée d'où il vient, et qui l'a dit parti (Gmail, ou Jordan
-- après un envoi incertain qu'il a vu dans Gmail, Messages envoyés).
ALTER TABLE envois ADD COLUMN expediteur TEXT;
ALTER TABLE envois ADD COLUMN piece_jointe TEXT;
ALTER TABLE envois ADD COLUMN piece_taille INTEGER;
ALTER TABLE envois ADD COLUMN offre_id INTEGER REFERENCES offres (id) ON DELETE SET NULL;
ALTER TABLE envois ADD COLUMN preparation_id INTEGER REFERENCES preparations (id) ON DELETE SET NULL;
ALTER TABLE envois ADD COLUMN confirme_par TEXT CHECK (confirme_par IN ('gmail', 'jordan'));
CREATE INDEX envois_candidature ON envois (candidature_id);
CREATE INDEX envois_message ON envois (message_id);

-- Chaque envoi tenté, noté AVANT de partir (« en_cours »), puis « envoye », « echec » (rien n'est parti),
-- « incertain » (la connexion s'est coupée juste après la fin du message), « pas_parti » ou « confirme » (ce que
-- Jordan a vu dans Gmail après un envoi incertain). Le rythme des envois (une minute entre deux, 20 par jour) se
-- compte ici ; un envoi resté « en_cours » quand Jobmind s'est arrêtée devient « incertain » au démarrage.
CREATE TABLE envois_tentes (
    id              INTEGER PRIMARY KEY,
    offre_id        INTEGER REFERENCES offres (id) ON DELETE SET NULL,
    preparation_id  INTEGER REFERENCES preparations (id) ON DELETE SET NULL,
    cv_id           INTEGER REFERENCES cv (id) ON DELETE SET NULL,
    expediteur      TEXT NOT NULL,
    destinataire    TEXT NOT NULL,
    objet           TEXT NOT NULL,
    contenu         TEXT NOT NULL,
    piece_jointe    TEXT,
    piece_taille    INTEGER,
    message_id      TEXT NOT NULL,
    tente_le        TEXT NOT NULL,
    fini_le         TEXT,
    etat            TEXT NOT NULL DEFAULT 'en_cours'
                        CHECK (etat IN ('en_cours', 'envoye', 'echec', 'incertain', 'pas_parti', 'confirme')),
    erreur          TEXT,
    envoi_id        INTEGER REFERENCES envois (id) ON DELETE SET NULL
);
CREATE INDEX envois_tentes_quand ON envois_tentes (tente_le);
CREATE INDEX envois_tentes_offre ON envois_tentes (offre_id);
