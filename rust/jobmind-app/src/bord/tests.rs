//! Les tests de l'étape 12 côté appli : les gestes du tableau de bord, des exports et de la sauvegarde, la remise d'une
//! sauvegarde de bout en bout dans un dossier d'essai (jamais les vraies données), le réglage du rafraîchissement.

use super::*;
use jobmind_core::candidature::{Saisie, verifier};
use jobmind_storage::horloge_figee;

fn m(h: u32, mi: u32) -> Moment {
    Moment::nouveau(2026, 11, 19, h, mi, 0).unwrap()
}

/// Un dossier d'essai neuf (sous %TEMP%), effacé à la fin du test.
struct Essai(std::path::PathBuf);
impl Essai {
    fn nouveau(nom: &str) -> Essai {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-bord-{nom}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        Essai(d)
    }
}
impl Drop for Essai {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ajouter(b: &Base, entreprise: &str, poste: &str, date: &str) {
    let s = Saisie {
        entreprise: entreprise.into(),
        poste: poste.into(),
        ville: "Sarreguemines".into(),
        date: date.into(),
        moyen: "mail".into(),
        source: "Indeed".into(),
        ..Saisie::default()
    };
    candidatures::ajouter(b.connexion(), &verifier(&s, m(9, 0).jour()).unwrap(), m(9, 0)).unwrap();
}

fn nombre(b: &Base) -> usize {
    candidatures::lister(b.connexion()).unwrap().0.len()
}

/// Aucun tiret long, aucun HTML, aucun texte vide là où l'écran écrit.
fn textes_simples(v: &Value) {
    match v {
        Value::String(s) => assert!(!s.contains(['\u{2013}', '\u{2014}', '<', '>']), "{s}"),
        Value::Array(a) => a.iter().for_each(textes_simples),
        Value::Object(o) => o.values().for_each(textes_simples),
        _ => {}
    }
}

#[test]
fn les_exports_preparent_la_boite() {
    let e = Essai::nouveau("exports");
    let (b, _) = Base::ouvrir(&e.0, m(9, 0)).unwrap();
    let mut bord = Bord::default();
    let ctx = Contexte { base: Some(&b), dossier: &e.0 };
    // Rien à exporter
    let f = bord.geste(&ctx, CSV, &Value::Null, m(9, 0));
    assert_eq!(f.message.as_deref(), Some("Aucune candidature à exporter pour l'instant."));
    assert!(f.fichier.is_none() && !bord.boite);
    ajouter(&b, "Brasserie du Lac de Madine", "Serveur", "12/11/2026");
    ajouter(&b, "Ateliers Vosgiens du Bois", "Menuisier poseur", "03/11/2026");
    let f = bord.geste(&ctx, CSV, &Value::Null, m(9, 0));
    let Some(Fichier::Enregistrer { genre: Genre::Csv, nom, octets }) = f.fichier else { panic!("{f:?}") };
    assert_eq!(nom, "Mes candidatures 2026-11-19.csv");
    assert!(octets.starts_with(b"\xEF\xBB\xBF"));
    assert!(String::from_utf8(octets).unwrap().contains("Brasserie du Lac de Madine;Serveur;Sarreguemines"));
    assert!(bord.boite);
    // Une seconde boîte pendant la première : non
    let f = bord.geste(&ctx, PDF, &Value::Null, m(9, 0));
    assert_eq!(f.message.as_deref(), Some("Une fenêtre pour choisir un fichier est déjà ouverte."));
    // Fermée sans choix
    let f = bord.retour(RetourFichier::Annule);
    assert!(f.message.is_none() && !bord.boite);
    let f = bord.geste(&ctx, PDF, &Value::Null, m(9, 0));
    let Some(Fichier::Enregistrer { genre: Genre::Pdf, nom, octets }) = f.fichier else { panic!("{f:?}") };
    assert_eq!(nom, "Mes candidatures 2026-11-19.pdf");
    assert!(octets.starts_with(b"%PDF-"));
    let f = bord.retour(RetourFichier::Enregistre { genre: Genre::Pdf, nom });
    assert_eq!(
        f.message.as_deref(),
        Some("Le PDF de tes candidatures est enregistré : « Mes candidatures 2026-11-19.pdf ».")
    );
    // Un dossier où l'on ne peut pas écrire
    bord.geste(&ctx, CSV, &Value::Null, m(9, 0));
    let f = bord.retour(RetourFichier::Erreur("accès refusé".into()));
    assert!(f.message.unwrap().contains("n'a pas pu être enregistré"));
    assert!(!bord.boite);
}

/// La sauvegarde, puis sa remise de bout en bout dans un dossier d'essai : la copie d'avant gardée, la base remplacée
/// puis rouverte, les clés jamais touchées ; un fichier abîmé ou d'une autre appli refusé ; rien pendant une recherche.
#[test]
fn sauvegarder_puis_remettre() {
    let e = Essai::nouveau("remise");
    let mut d = Donnees::ouvrir(&e.0, horloge_figee(m(9, 0)), false);
    std::fs::write(e.0.join("data").join("cles.json"), b"{\"adzuna_id\":\"chiffree\"}").unwrap();
    let mut bord = Bord::default();
    {
        let b = d.base.as_ref().unwrap();
        ajouter(b, "Fromagerie des Hautes-Vosges", "Vendeur", "10/11/2026");
        ajouter(b, "Garage Meusien", "Mécanicien", "05/11/2026");
    }
    let f = bord.geste(&Contexte { base: d.base.as_ref(), dossier: &e.0 }, SAUVER, &Value::Null, m(10, 0));
    let Some(Fichier::Enregistrer { genre: Genre::Zip, nom, octets }) = f.fichier else { panic!("{f:?}") };
    assert_eq!(nom, "Jobmind sauvegarde 2026-11-19.zip");
    let noms = sauvegarde::noms_des_parties(&octets).unwrap();
    assert!(noms.iter().all(|n| !n.contains("cles")), "{noms:?}");
    let zip = e.0.join("ma sauvegarde.zip");
    sauvegarde::ecrire(&zip, &octets).unwrap();
    let f = bord.retour(RetourFichier::Enregistre { genre: Genre::Zip, nom });
    assert_eq!(f.message.as_deref(), Some("Ta sauvegarde est enregistrée : « Jobmind sauvegarde 2026-11-19.zip »."));
    // Après la sauvegarde, une candidature de plus
    ajouter(d.base.as_ref().unwrap(), "Pressing Spinalien", "Agent de repassage", "18/11/2026");
    assert_eq!(nombre(d.base.as_ref().unwrap()), 3);
    // Un fichier qui n'est pas une sauvegarde : refusé, rien ne change
    let f = bord.geste(&Contexte { base: d.base.as_ref(), dossier: &e.0 }, RESTAURER, &Value::Null, m(11, 0));
    assert_eq!(f.fichier, Some(Fichier::Ouvrir));
    let lu = sauvegarde::lire(b"bonjour, je ne suis pas un zip", &e.0).map(Box::new);
    let f = bord.retour(RetourFichier::Lu { nom: "faux.zip".into(), contenu: lu });
    assert_eq!(f.message.as_deref(), Some("Ce fichier n'est pas une sauvegarde de Jobmind."));
    assert!(bord.attente.is_none());
    // La vraie : lue et vérifiée, puis la question
    bord.geste(&Contexte { base: d.base.as_ref(), dossier: &e.0 }, RESTAURER, &Value::Null, m(11, 0));
    let lu = sauvegarde::lire_fichier(&zip, &e.0).map(Box::new);
    let f = bord.retour(RetourFichier::Lu { nom: "ma sauvegarde.zip".into(), contenu: lu });
    assert_eq!(f.ordre, Some(json!({ "kit": "accueil", "sauvegarde": "question" })));
    let v = vue_reglages(&bord, d.base.as_ref(), &e.0, m(11, 0));
    textes_simples(&v);
    assert_eq!(v["sauvegarde"]["attente"]["fichier"], "ma sauvegarde.zip");
    assert_eq!(
        v["sauvegarde"]["attente"]["detail"],
        "Faite le jeudi 19 novembre 2026 à 10 h : 2 candidatures, 0 offre et 0 CV."
    );
    // Pendant une recherche : rien n'est fait, la question reste
    let f = bord.restaurer(&mut d, Some("la recherche d'offres"), m(11, 1));
    assert_eq!(f.message.as_deref(), Some("Attends la fin de la recherche d'offres, puis réessaie."));
    assert!(bord.attente.is_some());
    assert_eq!(nombre(d.base.as_ref().unwrap()), 3);
    // Oui : remise (la phrase dans la carte de la sauvegarde seulement, pas en double en haut de l'écran)
    let f = bord.restaurer(&mut d, None, m(11, 2));
    assert!(f.message.is_none() && f.bravo && f.change);
    let (t, genre) = bord.annonce.clone().unwrap();
    assert!(t.starts_with("Ta sauvegarde est remise (faite le jeudi 19 novembre 2026"), "{t}");
    assert_eq!(genre, "ok");
    assert!(f.journal.unwrap().contains("avant-restauration"));
    assert_eq!(nombre(d.base.as_ref().unwrap()), 2);
    assert!(bord.attente.is_none());
    assert!(!sauvegarde::en_attente(&e.0));
    // La copie d'avant est là, et elle contient bien les 3 candidatures
    let avant: Vec<_> = std::fs::read_dir(e.0.join(sauvegarde::DOSSIER))
        .unwrap()
        .filter_map(|x| x.ok())
        .filter(|x| x.file_name().to_string_lossy().starts_with("avant-restauration-"))
        .collect();
    assert_eq!(avant.len(), 1);
    let copie = sauvegarde::lire_fichier(&avant[0].path(), &e.0).unwrap();
    assert_eq!(copie.resume.candidatures, 3);
    // Les clés de ce PC n'ont pas bougé
    assert_eq!(std::fs::read(e.0.join("data").join("cles.json")).unwrap(), b"{\"adzuna_id\":\"chiffree\"}");
    // « Non » à une question : rien ne change
    bord.geste(&Contexte { base: d.base.as_ref(), dossier: &e.0 }, RESTAURER, &Value::Null, m(12, 0));
    let lu = sauvegarde::lire_fichier(&avant[0].path(), &e.0).map(Box::new);
    bord.retour(RetourFichier::Lu { nom: "copie.zip".into(), contenu: lu });
    let f = bord.geste(&Contexte { base: d.base.as_ref(), dossier: &e.0 }, NON, &Value::Null, m(12, 1));
    assert_eq!(f.message.as_deref(), Some("Rien n'a été changé."));
    assert_eq!(nombre(d.base.as_ref().unwrap()), 2);
    assert!(bord.restaurer(&mut d, None, m(12, 2)).message.is_none());
}

/// Une sauvegarde faite dans un autre dossier d'essai, avec ces candidatures : le fichier .zip.
fn une_sauvegarde(e: &Essai, entreprises: &[&str]) -> std::path::PathBuf {
    let autre = e.0.join("autre");
    let (b, _) = Base::ouvrir(&autre, m(8, 0)).unwrap();
    for x in entreprises {
        ajouter(&b, x, "Vendeur", "10/11/2026");
    }
    let (octets, _) = sauvegarde::faire(&b, &autre, m(8, 0)).unwrap();
    drop(b);
    let zip = e.0.join("sauvegarde.zip");
    sauvegarde::ecrire(&zip, &octets).unwrap();
    zip
}

/// Le bug grave de l'audit : quand la base est abîmée (elle ne s'ouvre pas), remettre une sauvegarde doit marcher,
/// c'est justement là qu'il le faut. La base abîmée est gardée telle quelle à côté, la sauvegarde remise, la base
/// rouverte ; rien n'attend plus au démarrage.
#[test]
fn remettre_quand_la_base_est_abimee() {
    let e = Essai::nouveau("abimee");
    let zip = une_sauvegarde(&e, &["Scierie de la Vologne", "Boucherie du Saulnois"]);
    // Une base abîmée : des octets qui ne sont pas une base SQLite
    std::fs::create_dir_all(e.0.join("data")).unwrap();
    std::fs::write(e.0.join("data").join("jobmind.db"), vec![0x42u8; 8192]).unwrap();
    let mut d = Donnees::ouvrir(&e.0, horloge_figee(m(9, 0)), false);
    assert!(d.base.is_none(), "la base abîmée ne s'ouvre pas");
    let mut bord = Bord::default();
    // « Remettre une sauvegarde » reste possible sans base ; les autres gestes disent pourquoi non
    let f = bord.geste(&Contexte { base: None, dossier: &e.0 }, SAUVER, &Value::Null, m(9, 1));
    assert_eq!(f.message.as_deref(), Some(SANS_BASE));
    let f = bord.geste(&Contexte { base: None, dossier: &e.0 }, RESTAURER, &Value::Null, m(9, 1));
    assert_eq!(f.fichier, Some(Fichier::Ouvrir));
    assert!(bord.boite);
    let lu = sauvegarde::lire_fichier(&zip, &e.0).map(Box::new);
    let f = bord.retour(RetourFichier::Lu { nom: "sauvegarde.zip".into(), contenu: lu });
    assert_eq!(f.ordre, Some(json!({ "kit": "accueil", "sauvegarde": "question" })));
    // La question est montrée même sans base
    let v = vue_reglages(&bord, None, &e.0, m(9, 2));
    assert_eq!(v["sauvegarde"]["attente"]["fichier"], "sauvegarde.zip");
    // Oui : remise
    let f = bord.restaurer(&mut d, None, m(9, 3));
    assert!(f.bravo, "{f:?}");
    let j = f.journal.unwrap();
    assert!(j.contains("base pas ouverte") && j.contains("base-abimee-"), "{j}");
    assert_eq!(nombre(d.base.as_ref().expect("base rouverte")), 2);
    assert!(!sauvegarde::en_attente(&e.0));
    // La base abîmée est gardée telle quelle dans le dossier des sauvegardes
    let gardees: Vec<_> = std::fs::read_dir(e.0.join(sauvegarde::DOSSIER))
        .unwrap()
        .filter_map(|x| x.ok())
        .filter(|x| x.file_name().to_string_lossy().starts_with("base-abimee-"))
        .collect();
    assert_eq!(gardees.len(), 1);
    assert_eq!(std::fs::read(gardees[0].path()).unwrap(), vec![0x42u8; 8192]);
    // Rien ne reste à défaire au démarrage suivant, ni de reste à effacer
    assert!(sauvegarde::nettoyer_restes(&e.0).is_empty());
    drop(d);
    let d = Donnees::ouvrir(&e.0, horloge_figee(m(9, 5)), false);
    assert_eq!(nombre(d.base.as_ref().unwrap()), 2);
}

/// Aucune base du tout (premier démarrage raté, dossier vide) : la remise marche aussi.
#[test]
fn remettre_sans_aucune_base() {
    let e = Essai::nouveau("sans-base");
    let zip = une_sauvegarde(&e, &["Moulin de Kalhausen"]);
    let mut d = Donnees::ouvrir(&e.0, horloge_figee(m(9, 0)), false);
    // (la base est fermée, puis son fichier retiré : comme si elle n'avait jamais pu s'ouvrir)
    d.base = None;
    for f in ["jobmind.db", "jobmind.db-wal", "jobmind.db-shm"] {
        let _ = std::fs::remove_file(e.0.join("data").join(f));
    }
    let mut bord = Bord::default();
    bord.geste(&Contexte { base: None, dossier: &e.0 }, RESTAURER, &Value::Null, m(9, 1));
    let lu = sauvegarde::lire_fichier(&zip, &e.0).map(Box::new);
    bord.retour(RetourFichier::Lu { nom: "sauvegarde.zip".into(), contenu: lu });
    let f = bord.restaurer(&mut d, None, m(9, 2));
    assert!(f.bravo, "{f:?}");
    assert!(f.journal.unwrap().contains("il n'y avait pas de base avant"));
    assert_eq!(nombre(d.base.as_ref().unwrap()), 1);
    assert!(!sauvegarde::en_attente(&e.0));
}

/// La copie d'avant ne peut pas être faite (le dossier des sauvegardes est un fichier) : rien n'est changé, la base
/// est rouverte telle quelle, et la carte le dit.
#[test]
fn remise_sans_copie_possible_rien_ne_change() {
    let e = Essai::nouveau("sans-copie");
    let zip = une_sauvegarde(&e, &["Verrerie de Meisenthal"]);
    let mut d = Donnees::ouvrir(&e.0, horloge_figee(m(9, 0)), false);
    ajouter(d.base.as_ref().unwrap(), "Faïencerie de Sarreguemines", "Emballeur", "12/11/2026");
    ajouter(d.base.as_ref().unwrap(), "Cristallerie de Saint-Louis", "Emballeur", "13/11/2026");
    let mut bord = Bord::default();
    bord.geste(&Contexte { base: d.base.as_ref(), dossier: &e.0 }, RESTAURER, &Value::Null, m(9, 1));
    let lu = sauvegarde::lire_fichier(&zip, &e.0).map(Box::new);
    bord.retour(RetourFichier::Lu { nom: "sauvegarde.zip".into(), contenu: lu });
    let _ = std::fs::remove_dir_all(e.0.join(sauvegarde::DOSSIER));
    std::fs::write(e.0.join(sauvegarde::DOSSIER), b"pas un dossier").unwrap();
    let f = bord.restaurer(&mut d, None, m(9, 2));
    assert!(!f.bravo && f.message.is_none());
    assert_eq!(bord.annonce.clone().unwrap().1, "erreur");
    assert!(bord.annonce.clone().unwrap().0.contains("Rien n'a été changé"));
    assert_eq!(nombre(d.base.as_ref().expect("base rouverte")), 2);
    assert!(!sauvegarde::en_attente(&e.0));
}

#[test]
fn le_rythme_du_rafraichissement() {
    let e = Essai::nouveau("rythme");
    let (b, _) = Base::ouvrir(&e.0, m(9, 0)).unwrap();
    let mut bord = Bord::default();
    let ctx = Contexte { base: Some(&b), dossier: &e.0 };
    let v = vue_reglages(&bord, Some(&b), &e.0, m(9, 0));
    assert_eq!(v["rafraichir"]["valeur"], 6);
    assert!(v["rafraichir"]["dernier"].is_null());
    let f = bord.geste(&ctx, RYTHME, &json!({ "heures": 12 }), m(9, 0));
    assert_eq!(f.message.as_deref(), Some("Jobmind cherchera de nouvelles offres toute seule toutes les 12 heures."));
    let f = bord.geste(&ctx, RYTHME, &json!({ "heures": 0 }), m(9, 0));
    assert_eq!(f.message.as_deref(), Some("Jobmind ne cherchera plus de nouvelles offres toute seule."));
    // Un rythme qui n'est pas proposé : refusé
    for faux in [json!({ "heures": 5 }), json!({ "heures": -6 }), json!({ "heures": "6" }), json!(null)] {
        let f = bord.geste(&ctx, RYTHME, &faux, m(9, 0));
        assert!(f.message.is_none() && !f.change, "{faux}");
    }
    assert_eq!(rafraichir::rythme(b.connexion()).unwrap(), 0);
    rafraichir::commencer(b.connexion(), m(8, 5)).unwrap();
    rafraichir::noter(b.connexion(), m(8, 5), None, 30, 4, rafraichir::EtatTour::Ok, "").unwrap();
    let v = vue_reglages(&bord, Some(&b), &e.0, m(9, 0));
    textes_simples(&v);
    assert_eq!(v["rafraichir"]["dernier"], "Dernière fois : aujourd'hui à 8 h 05, 4 nouvelles offres.");
    assert_eq!(v["rafraichir"]["choix"].as_array().unwrap().len(), 5);
}

#[test]
fn le_tableau_pour_l_ecran() {
    let e = Essai::nouveau("tableau");
    let (b, _) = Base::ouvrir(&e.0, m(9, 0)).unwrap();
    let v = vue_tableau(&tableau_de(&b, m(9, 0).jour()).unwrap());
    textes_simples(&v);
    assert_eq!(v["sous"], "Aucune candidature pour l'instant.");
    assert_eq!(v["cartes"][3]["nombre"], "0 %");
    ajouter(&b, "Boulangerie du Pont", "Vendeuse", "17/11/2026");
    ajouter(&b, "Transports de la Nied", "Chauffeur livreur", "02/11/2026");
    let v = vue_tableau(&tableau_de(&b, m(9, 0).jour()).unwrap());
    textes_simples(&v);
    assert_eq!(v["sous"], "2 candidatures en tout, depuis le début.");
    let cartes: Vec<(String, String)> = v["cartes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["nombre"].as_str().unwrap().to_string(), c["texte"].as_str().unwrap().to_string()))
        .collect();
    assert_eq!(
        cartes,
        [
            ("1".to_string(), "envoi cette semaine".to_string()),
            ("2".into(), "envois ce mois-ci".into()),
            ("0".into(), "réponses reçues".into()),
            ("0 %".into(), "de réponses".into()),
            ("0".into(), "entretiens".into()),
            ("1".into(), "relance à faire".into()),
        ]
    );
    assert_eq!(v["cartes"][5]["ton"], "attention");
    assert_eq!(v["barres"].as_array().unwrap().len(), 8);
    assert_eq!(v["statuts"][0]["part"], 1000);
}
