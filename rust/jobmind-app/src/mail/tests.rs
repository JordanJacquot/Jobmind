//! La boîte Gmail de bout en bout, sur une base en mémoire et la fausse boîte locale (127.0.0.1, jamais Gmail) : le
//! branchement vérifié et chiffré, l'essai, la relève (alertes, réponses rangées, à ranger, publicité laissée), les
//! bulles une seule fois, la confirmation du statut, ranger et « Rien à voir », l'état de la page sans mot de passe ni
//! HTML, et aucune commande qui écrit dans la boîte.

use super::exemples::{BOITE_ADRESSE, BOITE_MDP, boite_exemples, mail_brut};
use super::*;
use jobmind_core::candidature::{Fiche, Moyen};
use jobmind_mail::faux::{ConfigImap, FauxImap, ModeImap};
use jobmind_storage::cles::Coffre;

fn m(j: u32, h: u32) -> Moment {
    Moment::nouveau(2026, 11, j, h, 0, 0).unwrap()
}

struct Faux;
impl Coffre for Faux {
    fn fermer(&self, c: &[u8]) -> Result<Vec<u8>, String> {
        Ok(c.iter().rev().map(|b| b ^ 0x5c).collect())
    }
    fn ouvrir(&self, c: &[u8]) -> Result<Vec<u8>, String> {
        Ok(c.iter().rev().map(|b| b ^ 0x5c).collect())
    }
}

fn dossier(nom: &str) -> std::path::PathBuf {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let d = std::env::temp_dir().join(format!("JobmindEssai-{}-mail-{nom}-{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Les candidatures d'exemple, dans l'ordre de la construction d'essai (1 Transports Moselle Fret, 2 Logistique des
/// Trois Frontières, 3 Résidence Les Tilleuls, 4 Netéa Services, 5 Cartonnerie, 6 Brico Lorraine).
fn suivi(b: &Base) {
    for (e, p) in [
        ("Transports Moselle Fret", "Agent de quai"),
        ("Logistique des Trois Frontières", "Préparateur de commandes"),
        ("Résidence Les Tilleuls", "Agent de service"),
        ("Netéa Services", "Agent d'entretien"),
        ("Cartonnerie de la Seille", "Opérateur de production"),
        ("Brico Lorraine", "Vendeur rayon outillage"),
    ] {
        let f = Fiche {
            entreprise: e.into(),
            poste: p.into(),
            ville: "Metz".into(),
            envoyee_le: Jour::nouveau(2026, 11, 3).unwrap(),
            moyen: Moyen::Mail,
            source: String::new(),
            lien: String::new(),
            envoye: String::new(),
            cv: String::new(),
            notes: String::new(),
        };
        candidatures::ajouter(b.connexion(), &f, m(3, 9)).unwrap();
    }
}

/// Fait tourner une tâche du fil de la boîte ici même (le programme le fait sur un fil à part).
fn tourner(faux: &FauxImap, cles: &Cles, t: Tache) -> Retour {
    let (adresse, mdp) = identifiants(cles).unwrap();
    let acces = faux.acces(&adresse, &mdp);
    match t {
        Tache::Essayer { jeton } => {
            Retour { jeton, resultat: Resultat::Essai(jobmind_mail::essayer(&acces)), envoi: None }
        }
        Tache::Relever { jeton, reperes } => {
            Retour { jeton, resultat: Resultat::Releve(jobmind_mail::relever(&acces, &reperes)), envoi: None }
        }
    }
}

fn verifier_lecture_seule(faux: &FauxImap) {
    assert!(faux.refusees().is_empty(), "{:?}", faux.refusees());
    for c in faux.commandes() {
        let u = c.to_ascii_uppercase();
        for interdit in ["STORE", "COPY", "MOVE", "EXPUNGE", "SELECT", "APPEND", "DELETE", "CREATE", "RENAME"] {
            assert!(!u.split_whitespace().any(|w| w == interdit), "commande interdite : {c}");
        }
        assert!(!c.contains(BOITE_MDP), "mot de passe noté : {c}");
    }
}

#[test]
fn branchement_releve_et_rangement() {
    let d = dossier("tout");
    let (b, _) = Base::en_memoire(m(19, 6)).unwrap();
    suivi(&b);
    let (mut cles, _) = Cles::ouvrir(&d, Box::new(Faux));
    let faux = FauxImap::demarrer(ConfigImap::new(BOITE_ADRESSE, BOITE_MDP, boite_exemples(m(19, 12)))).unwrap();
    let mut boite = Boite::default();

    // Le branchement : vérifié (une adresse fausse, un mot de passe de compte : refusés, rien gardé)
    let f =
        boite.geste(Some(&b), Some(&mut cles), BRANCHER, &json!({ "adresse": "jordan", "mdp": BOITE_MDP }), m(19, 12));
    assert!(f.mail.is_none() && f.message.unwrap().contains("adresse"));
    let f = boite.geste(
        Some(&b),
        Some(&mut cles),
        BRANCHER,
        &json!({ "adresse": BOITE_ADRESSE, "mdp": "MonMotDePasse!" }),
        m(19, 12),
    );
    assert!(f.mail.is_none() && !f.message.unwrap().contains("MonMotDePasse"));
    assert!(!branchee(&cles));
    // Juste : gardé chiffré, l'essai part
    let groupes = "essa iess aies saii".to_uppercase();
    let f = boite.geste(
        Some(&b),
        Some(&mut cles),
        BRANCHER,
        &json!({ "adresse": BOITE_ADRESSE, "mdp": groupes }),
        m(19, 12),
    );
    assert!(branchee(&cles));
    let fichier = std::fs::read_to_string(Cles::chemin_dans(&d)).unwrap();
    assert!(!fichier.contains(BOITE_MDP) && !fichier.contains(BOITE_ADRESSE));
    let t = f.mail.clone().unwrap();
    assert!(matches!(t, Tache::Essayer { .. }));
    // (pendant l'essai, pas de seconde demande)
    assert!(boite.geste(Some(&b), Some(&mut cles), RELEVER, &Value::Null, m(19, 12)).mail.is_none());
    let f = boite.fini(Some(&b), tourner(&faux, &cles, t), BOITE_ADRESSE, false, m(19, 12));
    assert!(f.bravo, "{f:?}");
    // L'essai réussi : la relève tout de suite
    let t = f.mail.unwrap();
    let f = boite.fini(Some(&b), tourner(&faux, &cles, t), BOITE_ADRESSE, false, m(19, 12));
    let j = f.journal.clone().unwrap();
    assert!(j.starts_with("Mail : relève finie"), "{j}");
    let c = b.connexion();
    // Les alertes : leurs offres dans l'archive, venues d'une alerte mail
    let offres = jobmind_storage::offres::lister(c).unwrap();
    assert!(offres.len() >= 10, "{}", offres.len());
    let o = jobmind_storage::offres::lire(c, offres[0].id).unwrap().unwrap();
    assert_eq!(o.venue_par_mail, Jour::nouveau(2026, 11, 19));
    // Les réponses sûres : rangées, avec leur statut proposé ; la publicité n'est pas gardée
    let refus = base_mails::de_candidature(c, 1).unwrap();
    assert_eq!(refus.len(), 1);
    assert_eq!(refus[0].proposition.as_ref().map(|p| p.0), Some(Proposition::Refus));
    assert!(refus[0].attend);
    let entretien = base_mails::de_candidature(c, 4).unwrap();
    assert_eq!(entretien[0].proposition.as_ref().map(|p| p.0), Some(Proposition::Entretien));
    assert!(!entretien[0].texte.contains("alert(") && !entretien[0].texte.contains('<'), "{}", entretien[0].texte);
    let infos = base_mails::de_candidature(c, 3).unwrap();
    assert_eq!(infos[0].proposition.as_ref().map(|p| p.0), Some(Proposition::Infos));
    let a_ranger = base_mails::a_ranger(c).unwrap();
    assert_eq!(a_ranger.len(), 1, "{a_ranger:?}");
    assert_eq!(a_ranger[0].suggestion, Some(6));
    let pub_: i64 = c.query_row("SELECT COUNT(*) FROM mails WHERE message_id LIKE 'pub-%'", [], |r| r.get(0)).unwrap();
    assert_eq!(pub_, 0);
    // Les bulles : une par réponse rangée, une seule fois
    let n: i64 = c.query_row("SELECT COUNT(*) FROM bulles WHERE etat = 'attente'", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 3);
    // Une seconde relève : rien de neuf, aucune bulle de plus
    let t = boite.releve_due(Some(&b), &cles, Jour::nouveau(2026, 11, 19).unwrap()).unwrap();
    let f = boite.fini(Some(&b), tourner(&faux, &cles, t), BOITE_ADRESSE, false, m(19, 13));
    assert!(f.message.is_none(), "{f:?}");
    let n: i64 = c.query_row("SELECT COUNT(*) FROM bulles", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 3);
    // Un mail nouveau dans la boîte : seul lui est lu à la relève suivante
    faux.ajouter(mail_brut(
        "nouveau@cartonnerie-seille.fr",
        "Cartonnerie de la Seille <rh@cartonnerie-seille.fr>",
        "Votre candidature",
        "19 Nov 2026 12:30:00 +0100",
        "Bonjour, le poste a été pourvu. Nous gardons votre CV.",
        None,
    ));
    let t = boite.releve_due(Some(&b), &cles, Jour::nouveau(2026, 11, 19).unwrap()).unwrap();
    boite.fini(Some(&b), tourner(&faux, &cles, t), BOITE_ADRESSE, false, m(19, 14));
    assert_eq!(base_mails::de_candidature(c, 5).unwrap().len(), 1);

    // Jordan confirme le refus : le statut change (jamais avant)
    assert_eq!(candidatures::lire(c, 1).unwrap().unwrap().statut, Statut::Envoyee);
    let f = boite.geste(Some(&b), None, OUI, &json!({ "id": refus[0].id }), m(19, 15));
    assert_eq!(f.message.as_deref(), Some("Statut changé : Réponse négative."));
    assert_eq!(candidatures::lire(c, 1).unwrap().unwrap().statut, Statut::Negative);
    // L'entretien : le petit formulaire du dossier s'ouvre (la date se note), le statut ne change pas seul
    let f = boite.geste(Some(&b), None, OUI, &json!({ "id": entretien[0].id }), m(19, 15));
    assert_eq!(f.ordre, Some(json!({ "kit": "accueil", "statut": "entretien" })));
    assert_eq!(f.candidature, Some(4));
    assert_eq!(candidatures::lire(c, 4).unwrap().unwrap().statut, Statut::Envoyee);
    // La demande d'informations : « C'est noté »
    boite.geste(Some(&b), None, NON, &json!({ "id": infos[0].id }), m(19, 15));
    assert!(!base_mails::lire(c, infos[0].id).unwrap().unwrap().attend);
    // Le mail à ranger : rangé dans le dossier de Brico Lorraine
    let f = boite.geste(Some(&b), None, RANGER, &json!({ "id": a_ranger[0].id, "candidature": 6 }), m(19, 15));
    assert_eq!(f.candidature, Some(6));
    assert_eq!(base_mails::de_candidature(c, 6).unwrap().len(), 1);
    // « Rien à voir » sur un mail rangé : il part
    let f = boite.geste(Some(&b), None, RIEN, &json!({ "id": a_ranger[0].id }), m(19, 15));
    assert!(f.message.unwrap().contains("n'est plus gardé"));
    assert!(base_mails::de_candidature(c, 6).unwrap().is_empty());

    // L'état de la page : ni le mot de passe, ni le HTML d'un mail
    let (vm, _) = boite.vue(Some(&b), Some(&cles));
    assert!(vm.branchee && vm.adresse == BOITE_ADRESSE && vm.apercu == "gardé");
    // (0.2.1 : la fin du mot de passe n'est plus montrée)
    assert!(!vm.apercu.contains("saii"));
    let mut s = crate::suivi::Suivi { ouverte: Some(4) };
    let (e, _) = s.etat_tout(
        "0.7.0",
        Some(&b),
        Jour::nouveau(2026, 11, 19).unwrap(),
        &crate::vue_profil::VIDE,
        &crate::offres::VIDE,
        &vm,
    );
    let t = e.to_string();
    assert!(!t.contains(BOITE_MDP) && !t.contains("<script") && !t.contains("alert(1)"));
    let fiche = &e["accueil"]["perso"]["onglets"][0]["suivi"]["fiche"];
    assert_eq!(fiche["mails"].as_array().unwrap().len(), 1);
    assert!(fiche["frise"].as_array().unwrap().iter().any(|x| x["texte"] == "Réponse reçue"));
    // (l'entretien confirmé : plus de proposition, le formulaire de l'entretien prend le relais)
    assert!(fiche["proposition"].is_null(), "{}", fiche["proposition"]);
    let rub = &e["accueil"]["perso"]["onglets"][2]["rubriques"][4];
    assert_eq!((rub["id"].as_str(), rub["dossier"]["branchee"].as_bool()), (Some("mail"), Some(true)));

    // Débrancher : les clés effacées, la boîte n'est plus lue
    let f = boite.geste(Some(&b), Some(&mut cles), DEBRANCHER, &Value::Null, m(19, 16));
    assert!(f.message.unwrap().contains("débranchée"));
    assert!(!branchee(&cles) && boite.releve_due(Some(&b), &cles, Jour::nouveau(2026, 11, 19).unwrap()).is_none());
    verifier_lecture_seule(&faux);
    let _ = std::fs::remove_dir_all(&d);
}

/// Les erreurs de Gmail : un mot de passe refusé, l'accès IMAP fermé, la boîte qui ne répond plus. Une phrase simple,
/// jamais le mot de passe ; la relève suivante réessaie.
#[test]
fn erreurs_de_la_boite() {
    let d = dossier("erreurs");
    let (b, _) = Base::en_memoire(m(19, 6)).unwrap();
    let (mut cles, _) = Cles::ouvrir(&d, Box::new(Faux));
    let faux = FauxImap::demarrer(ConfigImap::new(BOITE_ADRESSE, BOITE_MDP, Vec::new())).unwrap();
    let mut boite = Boite::default();
    let f = boite.geste(
        Some(&b),
        Some(&mut cles),
        BRANCHER,
        &json!({ "adresse": BOITE_ADRESSE, "mdp": "abcdabcdabcdabcd" }),
        m(19, 12),
    );
    let f = boite.fini(Some(&b), tourner(&faux, &cles, f.mail.unwrap()), BOITE_ADRESSE, false, m(19, 12));
    let t = f.message.unwrap();
    assert!(t.contains("mot de passe") && !t.contains("abcdabcd"), "{t}");
    assert!(f.mail.is_none());
    for mode in [ModeImap::ImapFerme, ModeImap::Coupe] {
        *faux.mode().lock().unwrap() = mode;
        let f = boite.geste(Some(&b), Some(&mut cles), ESSAYER, &Value::Null, m(19, 12));
        let f = boite.fini(Some(&b), tourner(&faux, &cles, f.mail.unwrap()), BOITE_ADRESSE, false, m(19, 12));
        assert!(f.message.is_some() && boite.annonce.as_ref().unwrap().genre == "erreur");
    }
    // Un résultat arrivé après l'abandon : laissé
    let f = boite.geste(Some(&b), Some(&mut cles), RELEVER, &Value::Null, m(19, 12));
    let t = f.mail.unwrap();
    boite.trop_long();
    let f = boite.fini(Some(&b), tourner(&faux, &cles, t), BOITE_ADRESSE, false, m(19, 12));
    assert!(f.journal.unwrap().contains("abandonnée"));
    // Des gestes mal formés : rien ne casse
    for (id, v) in [
        (RANGER, json!({ "id": "1" })),
        (RANGER, json!({ "id": 99, "candidature": 99 })),
        (RIEN, json!(null)),
        (OUI, json!({ "id": -3 })),
        (NON, json!([1])),
        (BRANCHER, json!("x")),
    ] {
        let f = boite.geste(Some(&b), Some(&mut cles), id, &v, m(19, 12));
        assert!(!f.bravo, "{id}");
    }
    verifier_lecture_seule(&faux);
    let _ = std::fs::remove_dir_all(&d);
}

/// Audit du 27/11/2026 : une alerte n'est reconnue que si Gmail a vu la signature du site (l'en-tête
/// Authentication-Results). Le même mail sans cet en-tête, avec une signature ratée ou d'un autre domaine : aucune
/// offre ajoutée.
#[test]
fn alerte_sans_signature_du_site_ignoree() {
    let (b, _) = Base::en_memoire(m(19, 6)).unwrap();
    let a = &jobmind_sources::alertes::exemples::ALERTES[1];
    let brut = |id: &str| {
        let o = mail_brut(id, a.de, a.objet, "19 Nov 2026 06:15:00 +0100", a.texte, Some(a.html));
        String::from_utf8(o).unwrap()
    };
    let vrai = brut("vrai@essai.jobmind");
    assert!(vrai.starts_with("Authentication-Results: mx.google.com; dkim=pass header.i=@indeed.com"), "{vrai}");
    let sans = brut("sans@essai.jobmind").split_once("\r\n").unwrap().1.to_string();
    let rate = brut("rate@essai.jobmind").replacen("dkim=pass", "dkim=fail", 1);
    let autre = brut("autre@essai.jobmind").replacen("header.i=@indeed.com", "header.i=@arnaque.ru", 1);
    // (l'imitateur écrit lui-même un en-tête « dkim=pass » ; Gmail ajoute le sien AU-DESSUS : seul celui-là compte)
    let ajoute = format!("Authentication-Results: mx.google.com; dkim=none\r\n{}", brut("ajoute@essai.jobmind"));
    let releve = |n: u32, octets: &[u8]| jobmind_mail::Releve {
        uidvalidity: 1,
        dernier_uid: n,
        mails: vec![jobmind_mail::mime::lire(n, octets)],
        restants: 0,
        illisibles: 0,
    };
    for (n, faux) in [sans, rate, autre, ajoute].iter().enumerate() {
        let bilan = traiter(&b, &releve(n as u32 + 1, faux.as_bytes()), BOITE_ADRESSE, m(19, 7), 60).unwrap();
        assert_eq!((bilan.alertes, bilan.offres), (0, 0), "mail {n}");
    }
    assert!(jobmind_storage::offres::lister(b.connexion()).unwrap().is_empty());
    // Le vrai : ses offres ajoutées
    let bilan = traiter(&b, &releve(10, vrai.as_bytes()), BOITE_ADRESSE, m(19, 8), 60).unwrap();
    assert_eq!(bilan.alertes, 1);
    assert!(bilan.offres > 0);
}

#[test]
fn bilans_et_textes() {
    assert_eq!(Bilan::default().phrase(), "Rien de neuf pour tes candidatures.");
    let b = Bilan { alertes: 2, offres: 5, rangees: vec![(1, 2), (3, 4)], a_ranger: 1 };
    assert_eq!(
        b.phrase(),
        "2 réponses rangées dans leurs dossiers, 1 mail à ranger et 5 offres ajoutées par les alertes."
    );
    let r = rubrique(&VueMails::default(), Jour::nouveau(2026, 11, 19).unwrap());
    let t = r.to_string();
    assert!(!t.contains(['\u{2013}', '\u{2014}']) && !t.contains('<'));
    assert_eq!(r["dossier"]["etapes"].as_array().unwrap().len(), 4);
    assert_eq!(
        quand(Moment::nouveau(2026, 11, 19, 8, 5, 0), Jour::nouveau(2026, 11, 19).unwrap()),
        "le 19 nov. à 8 h 05"
    );
}

/// L'heure d'un mail reçu est ramenée à l'heure locale du PC (avant : l'heure écrite dans le mail, quel que soit son
/// fuseau) ; une date impossible n'est pas convertie (l'appelant garde l'ancien comportement).
#[test]
fn heure_des_mails_en_heure_locale() {
    let d = |annee, mois, jour, heure, minute, decalage_minutes| jobmind_mail::DateMail {
        annee,
        mois,
        jour,
        heure,
        minute,
        seconde: 7,
        decalage_minutes,
    };
    // 08:30 en heure universelle, lu à Paris en hiver (+60) : 09:30
    assert_eq!(heure_locale(&d(2026, 11, 19, 8, 30, 0), 60).unwrap().iso(), "2026-11-19T09:30:07");
    // 23:30 à New York (-300) le 18 : le 19 à 05:30 à Paris
    assert_eq!(heure_locale(&d(2026, 11, 18, 23, 30, -300), 60).unwrap().iso(), "2026-11-19T05:30:07");
    // 09:00 à Paris (+60), lu à Paris : inchangé ; en été (+120) : 10:00
    assert_eq!(heure_locale(&d(2026, 11, 19, 9, 0, 60), 60).unwrap().iso(), "2026-11-19T09:00:07");
    assert_eq!(heure_locale(&d(2026, 7, 1, 9, 0, 60), 120).unwrap().iso(), "2026-07-01T10:00:07");
    // Le 31 décembre tard à Tokyo (+540) : le même instant à Paris
    assert_eq!(heure_locale(&d(2027, 1, 1, 1, 0, 540), 60).unwrap().iso(), "2026-12-31T17:00:07");
    // Des dates impossibles : rien
    for x in [d(2026, 13, 1, 9, 0, 0), d(2026, 4, 31, 9, 0, 0), d(2026, 11, 19, 25, 0, 0), d(2026, 2, 29, 9, 0, 0)] {
        assert_eq!(heure_locale(&x, 60), None, "{x:?}");
    }
}
