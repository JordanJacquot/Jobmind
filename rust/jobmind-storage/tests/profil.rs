//! Le profil dans la base (étape 3) : les CV et le CV principal, les textes pour le style, « Ce que je cherche » gardé
//! puis relu tel quel, la migration 3 sur une base de l'étape 2, et la copie des fichiers de CV (sur un dossier d'essai
//! sous %TEMP%, jamais les vraies données).

use std::path::PathBuf;

use jobmind_core::recherche::{SaisieRecherche, verifier_recherche};
use jobmind_storage::profil::{self, GenreCv, NouveauCv};
use jobmind_storage::{Base, Moment};

fn m(h: u32) -> Moment {
    Moment::nouveau(2026, 11, 18, h, 5, 0).unwrap()
}

fn cv<'a>(nom: &'a str, texte: &'a str) -> NouveauCv<'a> {
    NouveauCv { nom, fichier: None, genre: GenreCv::Texte, texte, taille: None }
}

#[test]
fn les_cv_et_le_principal() {
    let (b, _) = Base::en_memoire(m(9)).unwrap();
    let c = b.connexion();
    assert!(profil::lister_cv(c).unwrap().is_empty());
    let a = profil::ajouter_cv(c, &cv("CV 2025", "Jordan Exemple, cariste"), m(9)).unwrap();
    let f = NouveauCv {
        nom: "CV 2026",
        fichier: Some(r"cv\cv-20261118-100500.pdf"),
        genre: GenreCv::Pdf,
        texte: "Jordan",
        taille: Some(182_000),
    };
    let b2 = profil::ajouter_cv(c, &f, m(10)).unwrap();
    // Le premier est le principal ; il reste en tête
    let l = profil::lister_cv(c).unwrap();
    assert_eq!(l.iter().map(|x| (x.id, x.principal)).collect::<Vec<_>>(), [(a, true), (b2, false)]);
    assert_eq!(l[1].fichier.as_deref(), Some(r"cv\cv-20261118-100500.pdf"));
    assert_eq!((l[1].genre, l[1].taille), (GenreCv::Pdf, Some(182_000)));
    // Un autre devient principal : un seul à la fois
    assert!(profil::rendre_principal(c, b2).unwrap());
    assert!(!profil::rendre_principal(c, 999).unwrap());
    let l = profil::lister_cv(c).unwrap();
    assert_eq!(l.iter().map(|x| (x.id, x.principal)).collect::<Vec<_>>(), [(b2, true), (a, false)]);
    let deux: Result<usize, _> = c.execute("UPDATE cv SET principal = 1", []);
    assert!(deux.is_err(), "deux CV principaux acceptés");
    // Modifié
    assert!(profil::modifier_cv(c, a, "CV ancien", "Texte corrigé", m(11)).unwrap());
    assert!(!profil::modifier_cv(c, 999, "x", "y", m(11)).unwrap());
    let x = profil::cv(c, a).unwrap().unwrap();
    assert_eq!((x.nom.as_str(), x.texte.as_str()), ("CV ancien", "Texte corrigé"));
    // Le principal supprimé : l'autre le devient, et le fichier à effacer est rendu
    let parti = profil::supprimer_cv(c, b2).unwrap().unwrap();
    assert_eq!(parti.fichier.as_deref(), Some(r"cv\cv-20261118-100500.pdf"));
    assert!(profil::cv(c, a).unwrap().unwrap().principal);
    assert!(profil::supprimer_cv(c, b2).unwrap().is_none());
    assert_eq!(profil::nombre_cv(c).unwrap(), 1);
}

#[test]
fn les_textes_pour_le_style() {
    let (b, _) = Base::en_memoire(m(9)).unwrap();
    let c = b.connexion();
    let a = profil::ajouter_style(c, "Lettre au magasin", "Bonjour, je vous écris pour la place.", m(9)).unwrap();
    let b2 = profil::ajouter_style(c, "Mail à un ami", "Salut, je te réponds vite.", m(10)).unwrap();
    let l = profil::lister_style(c).unwrap();
    assert_eq!(l.iter().map(|x| x.id).collect::<Vec<_>>(), [b2, a]);
    assert_eq!(l[1].ajoute_le.iso(), "2026-11-18");
    assert!(profil::supprimer_style(c, a).unwrap());
    assert!(!profil::supprimer_style(c, a).unwrap());
    assert_eq!(profil::nombre_style(c).unwrap(), 1);
}

#[test]
fn ce_que_je_cherche_garde_et_relu() {
    let (b, _) = Base::en_memoire(m(9)).unwrap();
    let c = b.connexion();
    assert_eq!(profil::lire_recherche(c).unwrap(), None);
    let mut s = SaisieRecherche {
        postes: "Agent de quai, cariste".into(),
        lieu: "ville".into(),
        ville: "Metz".into(),
        rayon: 30,
        contrats: vec!["interim".into(), "cdi".into()],
        teletravail: "possible".into(),
        sans_diplome: true,
        debutant: true,
        salaire: "1 800".into(),
        ..SaisieRecherche::default()
    };
    for (lieu, dep, region) in
        [("ville", "", ""), ("departement", "57", ""), ("region", "", "grand_est"), ("partout", "", "")]
    {
        s.lieu = lieu.into();
        s.departement = dep.into();
        s.region = region.into();
        let r = verifier_recherche(&s).unwrap();
        profil::enregistrer_recherche(c, &r, m(10)).unwrap();
        assert_eq!(profil::lire_recherche(c).unwrap().as_ref(), Some(&r), "{lieu}");
    }
    // Sans salaire ni contrat
    s.salaire = String::new();
    s.contrats.clear();
    let r = verifier_recherche(&s).unwrap();
    profil::enregistrer_recherche(c, &r, m(11)).unwrap();
    assert_eq!(profil::lire_recherche(c).unwrap(), Some(r));
    // Sans poste, autour d'une ville : tous les métiers du coin (gardé et relu tel quel)
    s.postes = String::new();
    s.lieu = "ville".into();
    s.ville = "Sarrebourg".into();
    let r = verifier_recherche(&s).unwrap();
    assert!(r.postes.is_empty());
    profil::enregistrer_recherche(c, &r, m(11)).unwrap();
    assert_eq!(profil::lire_recherche(c).unwrap(), Some(r));
    let n: i64 = c.query_row("SELECT COUNT(*) FROM profil", [], |x| x.get(0)).unwrap();
    assert_eq!(n, 1);
    // La base refuse un lieu ou un salaire hors des valeurs du cœur
    assert!(c.execute("UPDATE profil SET lieu = 'lune'", []).is_err());
    assert!(c.execute("UPDATE profil SET salaire_min = 99", []).is_err());
}

/// Une base de l'étape 2 (version 2) avec une candidature : la migration 3 l'ajoute sans rien perdre.
#[test]
fn une_base_de_l_etape_2_est_migree() {
    let mut b = Base::en_memoire_jusqua(2, m(9)).unwrap();
    b.connexion()
        .execute(
            "INSERT INTO candidatures (entreprise, poste, statut, envoyee_le, cree_le, change_le) \
             VALUES ('Brico Lorraine', 'Vendeur', 'envoyee', '2026-11-02', '2026-11-02T10:00:00', '2026-11-02T10:00:00')",
            [],
        )
        .unwrap();
    b.connexion()
        .execute("INSERT INTO cv (nom, type_fichier, texte, importe_le, principal) VALUES ('CV', 'texte', 'x', '2026-11-01T10:00:00', 1)", [])
        .unwrap();
    assert_eq!(b.migrer_tout(m(9)).unwrap(), vec![3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]);
    assert_eq!(jobmind_storage::candidatures::lister(b.connexion()).unwrap().0.len(), 1);
    let l = profil::lister_cv(b.connexion()).unwrap();
    assert_eq!((l.len(), l[0].principal, l[0].change_le, l[0].taille), (1, true, None, None));
    assert_eq!(profil::lire_recherche(b.connexion()).unwrap(), None);
}

fn essai(nom: &str) -> PathBuf {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let d = std::env::temp_dir().join(format!("JobmindEssai-{}-{nom}-{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn les_fichiers_de_cv_copies() {
    let d = essai("fichiers-cv");
    let a = profil::copier_fichier(&d, b"%PDF-1.4 a", "PDF", m(14)).unwrap();
    let b = profil::copier_fichier(&d, b"%PDF-1.4 b", "pdf", m(14)).unwrap();
    assert_eq!(a, r"cv\cv-20261118-140500.pdf");
    assert_eq!(b, r"cv\cv-20261118-140500-2.pdf");
    let ca = profil::chemin_fichier(&d, &a).unwrap();
    assert_eq!(std::fs::read(&ca).unwrap(), b"%PDF-1.4 a");
    assert_eq!(std::fs::read(profil::chemin_fichier(&d, &b).unwrap()).unwrap(), b"%PDF-1.4 b");
    assert_eq!(profil::copier_fichier(&d, b"x", "d.o/cx", m(15)).unwrap(), r"cv\cv-20261118-150500.docx");
    // Un chemin venu de la base n'est jamais suivi hors du dossier cv
    for faux in [
        r"cv\..\data\jobmind.db",
        r"..\cv\x.pdf",
        r"C:\Windows\x.pdf",
        r"data\jobmind.db",
        r"cv\",
        r"cv\.x",
        r"cv\a\b.pdf",
        "cv",
    ] {
        assert!(profil::chemin_fichier(&d, faux).is_none(), "{faux}");
    }
    let _ = std::fs::remove_dir_all(&d);
}
