-- Migration 1 (étape 1 du plan, 18/11/2026) : toutes les tables du plan, vides. Les dates sont des textes ISO en
-- heure locale (« 2026-11-18T14:05:00 »), les vrai ou faux des entiers 0 ou 1, les listes et réglages libres du JSON.
-- Les clés (Claude, Adzuna, Jooble, Gmail) ne sont jamais ici : elles vont dans un fichier chiffré à part (DPAPI).

-- Les sources d'offres (Adzuna, Jooble, page lue, alertes mail)
CREATE TABLE sources (
    id          INTEGER PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE,
    nom         TEXT NOT NULL,
    type        TEXT NOT NULL CHECK (type IN ('api', 'page', 'mail')),
    active      INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    reglages    TEXT,
    cree_le     TEXT NOT NULL
);

-- Les recherches gardées, rafraîchies au clic ou à intervalle régulier
CREATE TABLE recherches (
    id                  INTEGER PRIMARY KEY,
    nom                 TEXT NOT NULL,
    mots_cles           TEXT NOT NULL DEFAULT '',
    lieu                TEXT NOT NULL DEFAULT '',
    rayon_km            INTEGER NOT NULL DEFAULT 0,
    filtres             TEXT,
    rafraichir_minutes  INTEGER,
    derniere_fois       TEXT,
    cree_le             TEXT NOT NULL
);

-- Les offres, archivées en entier sur le PC (jamais effacées : une offre hors sujet est rangée à part)
CREATE TABLE offres (
    id                  INTEGER PRIMARY KEY,
    source_id           INTEGER REFERENCES sources (id),
    id_source           TEXT,
    url                 TEXT,
    titre               TEXT NOT NULL,
    entreprise          TEXT NOT NULL DEFAULT '',
    lieu                TEXT NOT NULL DEFAULT '',
    departement         TEXT,
    region              TEXT,
    latitude            REAL,
    longitude           REAL,
    contrat             TEXT,
    teletravail         TEXT,
    salaire             TEXT,
    diplome             TEXT,
    diplome_lu_par      TEXT CHECK (diplome_lu_par IN ('site', 'regles', 'ia')),
    experience          TEXT,
    experience_lue_par  TEXT CHECK (experience_lue_par IN ('site', 'regles', 'ia')),
    description         TEXT NOT NULL DEFAULT '',
    mail_contact        TEXT,
    url_candidature     TEXT,
    publiee_le          TEXT,
    ajoutee_le          TEXT NOT NULL,
    empreinte           TEXT,
    doublon_de          INTEGER REFERENCES offres (id),
    pertinence          INTEGER CHECK (pertinence BETWEEN 0 AND 100),
    raison              TEXT,
    hors_sujet          INTEGER NOT NULL DEFAULT 0 CHECK (hors_sujet IN (0, 1)),
    favori              INTEGER NOT NULL DEFAULT 0 CHECK (favori IN (0, 1)),
    plus_tard           INTEGER NOT NULL DEFAULT 0 CHECK (plus_tard IN (0, 1)),
    brut                TEXT,
    UNIQUE (source_id, id_source)
);
CREATE INDEX offres_empreinte ON offres (empreinte);
CREATE INDEX offres_ajoutee ON offres (ajoutee_le);

-- La recherche dans tout le texte des offres archivées (sans accents ni majuscules), tenue à jour par les déclencheurs
CREATE VIRTUAL TABLE offres_fts USING fts5 (
    titre, entreprise, lieu, description,
    content = 'offres', content_rowid = 'id', tokenize = 'unicode61 remove_diacritics 2'
);
CREATE TRIGGER offres_fts_ajout AFTER INSERT ON offres BEGIN
    INSERT INTO offres_fts (rowid, titre, entreprise, lieu, description)
    VALUES (new.id, new.titre, new.entreprise, new.lieu, new.description);
END;
CREATE TRIGGER offres_fts_retrait AFTER DELETE ON offres BEGIN
    INSERT INTO offres_fts (offres_fts, rowid, titre, entreprise, lieu, description)
    VALUES ('delete', old.id, old.titre, old.entreprise, old.lieu, old.description);
END;
CREATE TRIGGER offres_fts_changement AFTER UPDATE OF titre, entreprise, lieu, description ON offres BEGIN
    INSERT INTO offres_fts (offres_fts, rowid, titre, entreprise, lieu, description)
    VALUES ('delete', old.id, old.titre, old.entreprise, old.lieu, old.description);
    INSERT INTO offres_fts (rowid, titre, entreprise, lieu, description)
    VALUES (new.id, new.titre, new.entreprise, new.lieu, new.description);
END;

-- Ce que la page d'une offre montrait quand Jordan l'a ouverte (nombre de candidats sur LinkedIn, avec sa date)
CREATE TABLE offres_vues (
    id                  INTEGER PRIMARY KEY,
    offre_id            INTEGER NOT NULL REFERENCES offres (id),
    vue_le              TEXT NOT NULL,
    site                TEXT,
    nombre_candidats    INTEGER,
    texte_candidats     TEXT
);
CREATE INDEX offres_vues_offre ON offres_vues (offre_id);

-- Les CV de Jordan (importés en PDF ou DOCX, ou collés)
CREATE TABLE cv (
    id              INTEGER PRIMARY KEY,
    nom             TEXT NOT NULL,
    fichier         TEXT,
    type_fichier    TEXT CHECK (type_fichier IN ('pdf', 'docx', 'texte')),
    texte           TEXT NOT NULL DEFAULT '',
    importe_le      TEXT NOT NULL,
    principal       INTEGER NOT NULL DEFAULT 0 CHECK (principal IN (0, 1))
);

-- Les candidatures : le suivi (statuts de jobmind-core, Statut::code)
CREATE TABLE candidatures (
    id              INTEGER PRIMARY KEY,
    offre_id        INTEGER REFERENCES offres (id),
    entreprise      TEXT NOT NULL,
    poste           TEXT NOT NULL,
    lieu            TEXT NOT NULL DEFAULT '',
    statut          TEXT NOT NULL DEFAULT 'envoyee' CHECK (statut IN
                        ('envoyee', 'relancee', 'entretien', 'positive', 'negative', 'sans_reponse', 'abandonnee')),
    envoyee_le      TEXT,
    relance_le      TEXT,
    notes           TEXT NOT NULL DEFAULT '',
    cree_le         TEXT NOT NULL,
    change_le       TEXT NOT NULL
);
CREATE INDEX candidatures_statut ON candidatures (statut);

-- La frise d'une candidature : chaque statut et chaque geste daté
CREATE TABLE evenements (
    id              INTEGER PRIMARY KEY,
    candidature_id  INTEGER NOT NULL REFERENCES candidatures (id) ON DELETE CASCADE,
    quand           TEXT NOT NULL,
    type            TEXT NOT NULL,
    statut          TEXT,
    texte           TEXT
);
CREATE INDEX evenements_candidature ON evenements (candidature_id);

-- Ce que l'IA a préparé : le texte brut et le texte final gardés tous les deux, version par version
CREATE TABLE contenus_generes (
    id              INTEGER PRIMARY KEY,
    offre_id        INTEGER REFERENCES offres (id),
    candidature_id  INTEGER REFERENCES candidatures (id),
    type            TEXT NOT NULL CHECK (type IN ('lettre', 'message', 'reponses', 'conseil_cv')),
    version         INTEGER NOT NULL DEFAULT 1,
    texte_brut      TEXT NOT NULL,
    texte_final     TEXT,
    intensite       TEXT CHECK (intensite IN ('legere', 'moyenne', 'forte', 'aucune')),
    score_naturel   INTEGER CHECK (score_naturel BETWEEN 0 AND 100),
    modele          TEXT,
    cree_le         TEXT NOT NULL
);

-- Les envois : le contenu exact parti, par mail ou sur le site, et le CV joint
CREATE TABLE envois (
    id              INTEGER PRIMARY KEY,
    candidature_id  INTEGER NOT NULL REFERENCES candidatures (id),
    moyen           TEXT NOT NULL CHECK (moyen IN ('mail', 'site')),
    destinataire    TEXT,
    objet           TEXT,
    contenu         TEXT NOT NULL,
    cv_id           INTEGER REFERENCES cv (id),
    recapitulatif   TEXT,
    envoye_le       TEXT NOT NULL
);

-- Les mails lus : alertes d'offres et réponses rattachées à une candidature (les autres mails ne sont pas gardés)
CREATE TABLE mails (
    id              INTEGER PRIMARY KEY,
    message_id      TEXT NOT NULL UNIQUE,
    type            TEXT NOT NULL CHECK (type IN ('alerte', 'reponse')),
    candidature_id  INTEGER REFERENCES candidatures (id),
    expediteur      TEXT,
    objet           TEXT,
    recu_le         TEXT,
    extrait         TEXT,
    statut_propose  TEXT,
    traite          INTEGER NOT NULL DEFAULT 0 CHECK (traite IN (0, 1))
);

-- Le profil de Jordan (une seule ligne) : ce qu'il cherche, ses coordonnées pour les candidatures
CREATE TABLE profil (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    nom             TEXT NOT NULL DEFAULT '',
    mail            TEXT NOT NULL DEFAULT '',
    telephone       TEXT NOT NULL DEFAULT '',
    ville           TEXT NOT NULL DEFAULT '',
    metiers         TEXT NOT NULL DEFAULT '',
    rayon_km        INTEGER NOT NULL DEFAULT 30,
    contrats        TEXT,
    disponibilite   TEXT NOT NULL DEFAULT '',
    presentation    TEXT NOT NULL DEFAULT '',
    change_le       TEXT
);

-- Les textes de Jordan qui donnent son style (page « Mon style »)
CREATE TABLE profil_style (
    id          INTEGER PRIMARY KEY,
    titre       TEXT NOT NULL DEFAULT '',
    texte       TEXT NOT NULL,
    ajoute_le   TEXT NOT NULL
);

-- Les règles de réécriture : fixes (livrées avec Jobmind) ou ajoutées, désactivables
CREATE TABLE regles_style (
    id              INTEGER PRIMARY KEY,
    type            TEXT NOT NULL CHECK (type IN ('retirer', 'remplacer', 'plancher')),
    motif           TEXT NOT NULL,
    remplacement    TEXT,
    active          INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    fixe            INTEGER NOT NULL DEFAULT 0 CHECK (fixe IN (0, 1))
);

-- Les réglages de Jobmind (alerte après X jours, intensité du style, sources choisies...)
CREATE TABLE parametres (
    cle     TEXT PRIMARY KEY,
    valeur  TEXT NOT NULL
);
