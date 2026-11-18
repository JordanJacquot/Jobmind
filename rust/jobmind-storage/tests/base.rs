//! La base de Jobmind sur un dossier d'essai sous %TEMP% (jamais les vraies données) : création avec toutes les tables
//! du plan, migrations rejouées sans erreur ni changement, version enregistrée, recherche dans tout le texte, base
//! d'une version future jamais touchée, et les données réunies (Donnees) qui l'ouvrent.

use std::path::PathBuf;

use jobmind_core::Statut;
use jobmind_storage::base::{DOSSIER, FICHIER, version_connue};
use jobmind_storage::{Base, Donnees, ErreurBase, Moment, horloge_figee};

/// Un dossier d'essai neuf sous %TEMP%, effacé à la fin.
struct Essai(PathBuf);

impl Essai {
    fn nouveau(nom: &str) -> Essai {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-{nom}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        Essai(d)
    }
}

impl Drop for Essai {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn moment() -> Moment {
    Moment::nouveau(2026, 11, 18, 14, 5, 0).unwrap()
}

/// Les tables du plan (phase 2, « Base de données »).
const TABLES: [&str; 15] = [
    "offres",
    "offres_fts",
    "offres_vues",
    "sources",
    "recherches",
    "candidatures",
    "evenements",
    "contenus_generes",
    "envois",
    "mails",
    "profil",
    "cv",
    "profil_style",
    "regles_style",
    "parametres",
];

fn tables(b: &Base) -> Vec<String> {
    let mut r = b.connexion().prepare("SELECT name FROM sqlite_master WHERE type IN ('table', 'view')").unwrap();
    r.query_map([], |l| l.get::<_, String>(0)).unwrap().map(Result::unwrap).collect()
}

#[test]
fn creation_avec_toutes_les_tables() {
    let e = Essai::nouveau("creation");
    let (b, appliquees) = Base::ouvrir(&e.0, moment()).unwrap();
    assert_eq!(appliquees, vec![1]);
    assert_eq!(b.version().unwrap(), 1);
    assert_eq!(version_connue(), 1);
    // data\jobmind.db dans le dossier de données
    assert_eq!(b.chemin().unwrap(), e.0.join(DOSSIER).join(FICHIER));
    assert!(e.0.join("data").join("jobmind.db").is_file());
    let t = tables(&b);
    for nom in TABLES {
        assert!(t.iter().any(|x| x == nom), "table {nom} absente : {t:?}");
    }
    // Toutes vides
    for nom in TABLES {
        let n: i64 = b.connexion().query_row(&format!("SELECT COUNT(*) FROM {nom}"), [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "{nom}");
    }
    // La version enregistrée : la table des migrations (avec sa date) et PRAGMA user_version
    let (v, nom, quand): (u32, String, String) = b
        .connexion()
        .query_row("SELECT version, nom, appliquee_le FROM migrations", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap();
    assert_eq!((v, nom.as_str(), quand.as_str()), (1, "les tables du plan", "2026-11-18T14:05:00"));
    let u: u32 = b.connexion().query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(u, 1);
    // Les liens entre tables sont vérifiés
    let fk: i64 = b.connexion().query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
    assert_eq!(fk, 1);
    assert_eq!(b.ligne_journal(&appliquees), r"Base : data\jobmind.db prête (version 1, migration 1 appliquée)");
}

#[test]
fn migrations_rejouees_sans_erreur() {
    let e = Essai::nouveau("rejouees");
    {
        let (b, _) = Base::ouvrir(&e.0, moment()).unwrap();
        b.connexion().execute("INSERT INTO parametres (cle, valeur) VALUES ('alerte_jours', '14')", []).unwrap();
    }
    // Rouverte trois fois : rien d'appliqué de nouveau, rien de perdu, une seule ligne de migration
    for _ in 0..3 {
        let (b, appliquees) = Base::ouvrir(&e.0, moment()).unwrap();
        assert!(appliquees.is_empty());
        assert_eq!(b.version().unwrap(), 1);
        let n: i64 = b.connexion().query_row("SELECT COUNT(*) FROM migrations", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
        let v: String = b
            .connexion()
            .query_row("SELECT valeur FROM parametres WHERE cle = 'alerte_jours'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, "14");
        assert_eq!(b.ligne_journal(&appliquees), r"Base : data\jobmind.db prête (version 1)");
    }
}

#[test]
fn base_d_une_version_future_jamais_touchee() {
    let e = Essai::nouveau("future");
    {
        let (b, _) = Base::ouvrir(&e.0, moment()).unwrap();
        b.connexion()
            .execute("INSERT INTO migrations (version, nom, appliquee_le) VALUES (2, 'plus tard', 'x')", [])
            .unwrap();
    }
    let avant = std::fs::read(e.0.join("data").join("jobmind.db")).unwrap();
    match Base::ouvrir(&e.0, moment()) {
        Err(ErreurBase::Future { trouvee: 2, connue: 1 }) => {}
        Err(autre) => panic!("erreur inattendue : {autre}"),
        Ok(_) => panic!("une base de version 2 a été ouverte"),
    }
    assert_eq!(std::fs::read(e.0.join("data").join("jobmind.db")).unwrap(), avant);
}

#[test]
fn recherche_dans_tout_le_texte_des_offres() {
    let (b, _) = Base::en_memoire(moment()).unwrap();
    let c = b.connexion();
    c.execute(
        "INSERT INTO offres (titre, entreprise, lieu, description, ajoutee_le) VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            "Préparateur de commandes",
            "Logistique des Trois Frontières",
            "Thionville",
            "Débutant accepté, CACES 1 apprécié.",
            "2026-11-18T14:05:00",
        ],
    )
    .unwrap();
    c.execute(
        "INSERT INTO offres (titre, entreprise, lieu, description, ajoutee_le) VALUES (?1, ?2, ?3, ?4, ?5)",
        ["Comptable", "Cabinet Fiducia Est", "Thionville", "BTS comptabilité.", "2026-11-18T14:05:00"],
    )
    .unwrap();
    let trouve = |q: &str| -> Vec<String> {
        let mut r = c
            .prepare("SELECT o.titre FROM offres_fts f JOIN offres o ON o.id = f.rowid WHERE offres_fts MATCH ?1 ORDER BY o.id")
            .unwrap();
        r.query_map([q], |l| l.get::<_, String>(0)).unwrap().map(Result::unwrap).collect()
    };
    // Sans accents ni majuscules
    assert_eq!(trouve("debutant"), ["Préparateur de commandes"]);
    assert_eq!(trouve("FRONTIERES"), ["Préparateur de commandes"]);
    assert_eq!(trouve("thionville"), ["Préparateur de commandes", "Comptable"]);
    // Tenue à jour : une offre changée ou retirée l'est aussi dans la recherche
    c.execute("UPDATE offres SET description = 'Expérience exigée.' WHERE titre = 'Comptable'", []).unwrap();
    assert!(trouve("bts").is_empty());
    assert_eq!(trouve("experience"), ["Comptable"]);
    c.execute("DELETE FROM offres WHERE titre = 'Comptable'", []).unwrap();
    assert!(trouve("experience").is_empty());
}

/// La base accepte chaque statut de jobmind-core, et rien d'autre.
#[test]
fn les_statuts_de_la_base_sont_ceux_du_coeur() {
    let (b, _) = Base::en_memoire(moment()).unwrap();
    let ajouter = |statut: &str| {
        b.connexion().execute(
            "INSERT INTO candidatures (entreprise, poste, statut, cree_le, change_le) VALUES ('A', 'B', ?1, 'x', 'x')",
            [statut],
        )
    };
    for s in Statut::TOUS {
        ajouter(s.code()).unwrap();
    }
    assert!(ajouter("Envoyee").is_err());
    assert!(ajouter("perdue").is_err());
    // Une candidature liée à une offre qui n'existe pas : refusée (liens vérifiés)
    let r = b.connexion().execute(
        "INSERT INTO candidatures (offre_id, entreprise, poste, cree_le, change_le) VALUES (999, 'A', 'B', 'x', 'x')",
        [],
    );
    assert!(r.is_err());
}

#[test]
fn les_donnees_ouvrent_la_base_et_le_notent() {
    let e = Essai::nouveau("donnees");
    let h = horloge_figee(moment());
    let d = Donnees::ouvrir(&e.0, h.clone(), false);
    assert_eq!(d.base.as_ref().unwrap().version().unwrap(), 1);
    drop(d);
    let d = Donnees::ouvrir(&e.0, h, false);
    assert!(d.base.is_some());
    let j = std::fs::read_to_string(e.0.join("journal.txt")).unwrap();
    assert_eq!(
        j,
        "2026-11-18T14:05:00 Base : data\\jobmind.db prête (version 1, migration 1 appliquée)\r\n\
         2026-11-18T14:05:00 Base : data\\jobmind.db prête (version 1)\r\n"
    );
}

#[test]
fn une_base_illisible_n_empeche_pas_de_s_ouvrir() {
    let e = Essai::nouveau("illisible");
    std::fs::create_dir_all(e.0.join("data")).unwrap();
    std::fs::write(e.0.join("data").join("jobmind.db"), b"ceci n'est pas une base SQLite, vraiment pas du tout")
        .unwrap();
    let d = Donnees::ouvrir(&e.0, horloge_figee(moment()), false);
    assert!(d.base.is_none());
    let j = std::fs::read_to_string(e.0.join("journal.txt")).unwrap();
    assert!(j.starts_with("2026-11-18T14:05:00 Base : non ouverte (SQLite : "), "{j}");
}
