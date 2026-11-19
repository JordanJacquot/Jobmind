//! L'envoi par mail de bout en bout, sur une base en mémoire et le faux serveur d'envoi local (127.0.0.1 : il garde ce
//! qu'il reçoit, aucun mail ne part) : le récapitulatif, l'envoi après le clic, un seul envoi pour un double clic, le
//! rythme, les refus avant de partir (adresse, CV absent, boîte), les pannes (mot de passe, destinataire, coupure,
//! envoi incertain puis vu par Jordan), la candidature entrée dans le suivi avec le contenu exact, et la réponse
//! (« Re: ») reconnue à coup sûr par la relève de l'étape 7 grâce au Message-ID.

use super::*;
use crate::mail::exemples::{BOITE_ADRESSE, BOITE_MDP};
use jobmind_core::cle::Cle;
use jobmind_core::{Statut, candidature::Moyen};
use jobmind_mail::faux_smtp::{FauxSmtp, ModeSmtp, base64_lu};
use jobmind_mail::{Mail, Releve};
use jobmind_sources::OffreTrouvee;
use jobmind_storage::candidatures;
use jobmind_storage::cles::Coffre;
use jobmind_storage::preparations::NouvellePreparation;
use jobmind_storage::profil::{GenreCv, NouveauCv};

fn m(h: u32, mi: u32, s: u32) -> Moment {
    Moment::nouveau(2026, 11, 19, h, mi, s).unwrap()
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
    let d = std::env::temp_dir().join(format!("JobmindEssai-{}-envoi-{nom}-{n}", std::process::id()));
    std::fs::create_dir_all(d.join("cv")).unwrap();
    d
}

const LETTRE: &str = "Madame, Monsieur,\n\nJe vous propose ma candidature au poste d'agent de quai. J'ai chargé et \
                      déchargé des camions pendant deux ans à Woippy.\n\nJe vous prie d'agréer, Madame, Monsieur, mes \
                      salutations distinguées.\n\nJordan Exemple";

fn pdf() -> Vec<u8> {
    let mut o = b"%PDF-1.4\n".to_vec();
    o.extend((0..3000u32).map(|i| (i % 251) as u8));
    o.extend_from_slice(b"\n%%EOF\n");
    o
}

/// Une base : une offre qui donne une adresse, sa candidature préparée, un CV importé (le fichier dans `cv`).
fn base(d: &Path, avec_fichier: bool) -> (Base, i64, i64) {
    let (b, _) = Base::en_memoire(m(9, 0, 0)).unwrap();
    let c = b.connexion();
    let o = OffreTrouvee {
        id_source: "1".into(),
        titre: "Agent de quai H/F".into(),
        entreprise: "Transports Moselle Fret".into(),
        lieu: "Woippy (57)".into(),
        zones: Vec::new(),
        url: String::new(),
        description: "Chargement des camions. CACES 3 exigé.\nPour postuler, envoyez CV et lettre à \
                      Recrutement@TMF-exemple.fr avant le 30 novembre."
            .into(),
        contrat: String::new(),
        salaire: String::new(),
        publiee_le: String::new(),
        latitude: None,
        longitude: None,
        brut: "{}".into(),
    };
    base_offres::ajouter_trouvees(c, "adzuna", &[o], None, m(9, 0, 0)).unwrap();
    let offre = base_offres::lister(c).unwrap()[0].id;
    let n = NouvellePreparation {
        offre_id: offre,
        modele: "claude-opus-5-5".into(),
        reponse_brute: "{}".into(),
        lettre_brute: LETTRE.into(),
        objet: "Candidature au poste d'agent de quai, réf. 57-112".into(),
        questions: json!([]),
        conseils: json!([]),
        ..NouvellePreparation::default()
    };
    let p = base_prep::ajouter(c, &n, m(9, 30, 0)).unwrap();
    if avec_fichier {
        std::fs::write(d.join("cv").join("cv-20261118-140500.pdf"), pdf()).unwrap();
        let n = NouveauCv {
            nom: "CV Jordan Exemple.pdf",
            fichier: Some("cv\\cv-20261118-140500.pdf"),
            genre: GenreCv::Pdf,
            texte: "Agent de quai, deux ans à Woippy. CACES 3.",
            taille: Some(pdf().len() as i64),
        };
        profil::ajouter_cv(c, &n, m(9, 0, 0)).unwrap();
    }
    (b, offre, p)
}

fn cles(d: &Path) -> Cles {
    let (mut c, _) = Cles::ouvrir(d, Box::new(Faux));
    c.garder_plusieurs(&[(Cle::GmailAdresse, BOITE_ADRESSE), (Cle::GmailMdp, BOITE_MDP)]).unwrap();
    c
}

fn ctx<'a>(b: &'a Base, c: &'a Cles, d: &'a Path) -> Contexte<'a> {
    Contexte { base: Some(b), cles: Some(c), dossier: d, decalage_minutes: 60 }
}

fn saisie(offre: i64, p: i64) -> Value {
    json!({ "id": offre, "version": p, "a": "recrutement@tmf-exemple.fr", "objet": "Candidature au poste d'agent de quai, réf. 57-112" })
}

/// Fait tourner la tâche contre le faux serveur, comme le fil du programme.
fn tourner(s: &FauxSmtp, t: &Tache) -> Retour {
    Retour { jeton: t.jeton, resultat: smtp::envoyer(&s.acces(BOITE_ADRESSE, BOITE_MDP), &t.brouillon) }
}

#[test]
fn recapitulatif_puis_envoi_puis_suivi_puis_reponse_reconnue() {
    let d = dossier("normal");
    let (b, offre, p) = base(&d, true);
    let k = cles(&d);
    let s = FauxSmtp::demarrer(BOITE_ADRESSE, BOITE_MDP).unwrap();
    let mut e = Envoi::default();
    // Le récapitulatif : l'adresse lue dans l'offre, avec son extrait ; la lettre gardée ; le CV joint
    let f = e.geste(&ctx(&b, &k, &d), OUVRIR, &json!({ "id": offre }), m(10, 0, 0));
    assert!(f.change && f.envoi.is_none(), "ouvrir n'envoie rien");
    let (v, _) = e.vue(Some(&b), Some(&k), &d, Some(offre), m(10, 0, 0));
    assert_eq!(v["ouvert"], true);
    assert_eq!(v["de"], BOITE_ADRESSE);
    assert_eq!(v["a"], "recrutement@tmf-exemple.fr");
    assert_eq!(
        v["a_lue"]["extrait"],
        "Pour postuler, envoyez CV et lettre à Recrutement@TMF-exemple.fr avant le 30 novembre."
    );
    assert_eq!(v["objet"], "Candidature au poste d'agent de quai, réf. 57-112");
    assert_eq!(v["lettre"], LETTRE);
    assert_eq!(v["cv"]["nom"], "CV Jordan Exemple.pdf");
    assert_eq!(v["cv"]["present"], true);
    assert!(v["attente"].is_null() && v["incertain"].is_null() && v["envoyee"].is_null());
    assert!(s.recus().is_empty() && s.sessions() == 0, "rien ne part sans le clic");
    // Le clic sur « Envoyer » ; un double clic : une seule tâche
    let f = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 5));
    let t = f.envoi.clone().expect("la tâche");
    let f2 = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 5));
    assert!(f2.envoi.is_none(), "double clic : un seul envoi");
    assert!(f2.message.unwrap().contains("en train de partir"));
    let (v, _) = e.vue(Some(&b), Some(&k), &d, Some(offre), m(10, 0, 6));
    assert_eq!(v["en_cours"], true);
    // (aucun mot de passe dans la tâche, l'état de la page ni le journal)
    for x in [format!("{t:?}"), v.to_string(), f.journal.clone().unwrap_or_default()] {
        assert!(!x.contains(BOITE_MDP), "mot de passe visible : {x}");
    }
    assert_eq!(t.brouillon.a, "recrutement@tmf-exemple.fr");
    assert_eq!(t.brouillon.de, BOITE_ADRESSE);
    let r = tourner(&s, &t);
    let f = e.fini(Some(&b), r, m(10, 0, 7));
    assert!(f.bravo && f.message.unwrap().contains("partie"));
    assert_eq!(s.recus().len(), 1);
    assert_eq!(s.recus()[0].a, vec!["recrutement@tmf-exemple.fr".to_string()]);
    // La candidature dans le suivi : Envoyée, par mail, le contenu exact
    let c = b.connexion();
    let (l, _) = candidatures::lister(c).unwrap();
    assert_eq!(l.len(), 1);
    let x = &l[0];
    assert_eq!((x.statut, x.moyen), (Statut::Envoyee, Moyen::Mail));
    assert_eq!(
        (x.entreprise.as_str(), x.poste.as_str(), x.ville.as_str()),
        ("Transports Moselle Fret", "Agent de quai H/F", "Woippy")
    );
    assert_eq!(x.envoye, LETTRE);
    assert_eq!(x.cv, "CV Jordan Exemple.pdf");
    let dossier = pour_dossier(Some(&b), x.id, m(10, 0, 7).jour());
    let e0 = &dossier[0];
    assert_eq!(e0["texte"], LETTRE);
    assert_eq!(e0["lignes"][1], json!(["À", "recrutement@tmf-exemple.fr"]));
    assert_eq!(e0["lignes"][2], json!(["Objet", "Candidature au poste d'agent de quai, réf. 57-112"]));
    assert_eq!(e0["lignes"][3], json!(["Envoyé", "aujourd'hui à 10 h 00"]));
    assert_eq!(e0["lignes"][4], json!(["CV joint", "CV Jordan Exemple.pdf (3 Ko)"]));
    let mid = t.brouillon.message_id.clone();
    assert_eq!(e0["message_id"], format!("Numéro du mail : <{mid}>"));
    // Le mail reçu par le serveur : la lettre et le CV, octet par octet
    let brut = String::from_utf8(s.recus()[0].octets.clone()).unwrap();
    let lu = jobmind_mail::mime::lire(1, brut.as_bytes());
    assert_eq!(lu.message_id, mid);
    assert_eq!(lu.objet, "Candidature au poste d'agent de quai, réf. 57-112");
    let pj = brut.split("Content-Disposition: attachment;").nth(1).unwrap().split("\r\n\r\n").nth(1).unwrap();
    assert_eq!(base64_lu(pj.split("--").next().unwrap()).unwrap(), pdf());
    // La vue : envoyée
    let (v, _) = e.vue(Some(&b), Some(&k), &d, Some(offre), m(10, 0, 8));
    assert_eq!(v["issue"]["genre"], "parti");
    assert!(v["envoyee"]["texte"].as_str().unwrap().starts_with("Envoyée par mail aujourd'hui"));
    // La réponse du recruteur (« Re: », In-Reply-To : le Message-ID) reconnue à coup sûr par la relève
    let reponse = format!(
        "From: Service RH <rh@autre-domaine-exemple.fr>\r\nTo: {BOITE_ADRESSE}\r\nSubject: Re: Candidature\r\n\
         Date: Thu, 20 Nov 2026 09:00:00 +0100\r\nMessage-ID: <rep1@autre-domaine-exemple.fr>\r\n\
         In-Reply-To: <{mid}>\r\nReferences: <{mid}>\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n\
         Bonjour, merci pour votre candidature. Nous vous proposons un entretien.\r\n"
    );
    let mail: Mail = jobmind_mail::mime::lire(5, reponse.as_bytes());
    let rel = Releve { uidvalidity: 1, dernier_uid: 5, mails: vec![mail], restants: 0, illisibles: 0 };
    let bilan = crate::mail::traiter(&b, &rel, BOITE_ADRESSE, m(10, 30, 0)).unwrap();
    assert_eq!(bilan.rangees.len(), 1, "rangée à coup sûr");
    assert_eq!(bilan.rangees[0].1, x.id);
    let raison: String = c
        .query_row("SELECT raison FROM mails WHERE message_id = 'rep1@autre-domaine-exemple.fr'", [], |r| r.get(0))
        .unwrap();
    assert!(raison.contains("répond au mail envoyé par Jobmind"), "{raison}");
}

/// Un « Re: » sans In-Reply-To ni References, d'un autre domaine, sans le nom de l'entreprise : pas sûr (à ranger) ;
/// avec References seulement : sûr.
#[test]
fn reponse_par_references_seulement() {
    let d = dossier("references");
    let (b, offre, p) = base(&d, true);
    let k = cles(&d);
    let s = FauxSmtp::demarrer(BOITE_ADRESSE, BOITE_MDP).unwrap();
    let mut e = Envoi::default();
    let t = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 0)).envoi.unwrap();
    e.fini(Some(&b), tourner(&s, &t), m(10, 0, 1));
    let mid = t.brouillon.message_id;
    let mail = |n: u32, refs: &str| {
        let brut = format!(
            "From: Inconnu <x{n}@ailleurs-exemple.org>\r\nTo: {BOITE_ADRESSE}\r\nSubject: Re: votre message\r\n\
             Date: Thu, 20 Nov 2026 09:00:00 +0100\r\nMessage-ID: <r{n}@ailleurs-exemple.org>\r\n{refs}\
             Content-Type: text/plain; charset=utf-8\r\n\r\nBonjour, bien reçu.\r\n"
        );
        jobmind_mail::mime::lire(n, brut.as_bytes())
    };
    let rel = Releve {
        uidvalidity: 1,
        dernier_uid: 2,
        mails: vec![mail(1, ""), mail(2, &format!("References: <autre@x.fr> <{mid}>\r\n"))],
        restants: 0,
        illisibles: 0,
    };
    let bilan = crate::mail::traiter(&b, &rel, BOITE_ADRESSE, m(10, 30, 0)).unwrap();
    assert_eq!(bilan.rangees.len(), 1);
    let rangee: String =
        b.connexion().query_row("SELECT message_id FROM mails WHERE etat = 'rangee'", [], |r| r.get(0)).unwrap();
    assert_eq!(rangee, "r2@ailleurs-exemple.org");
}

#[test]
fn le_rythme_une_minute_entre_deux_envois() {
    let d = dossier("rythme");
    let (b, offre, p) = base(&d, true);
    let k = cles(&d);
    let s = FauxSmtp::demarrer(BOITE_ADRESSE, BOITE_MDP).unwrap();
    let mut e = Envoi::default();
    let t = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 0)).envoi.unwrap();
    e.fini(Some(&b), tourner(&s, &t), m(10, 0, 2));
    // Déjà envoyée : il faut le dire (« Envoyer encore une fois »)
    let f = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 2, 0));
    assert!(f.envoi.is_none() && f.message.unwrap().contains("déjà partie"));
    let mut encore = saisie(offre, p);
    encore["encore"] = json!(true);
    // 30 secondes après : trop tôt
    let f = e.geste(&ctx(&b, &k, &d), ENVOYER, &encore, m(10, 0, 30));
    assert!(f.envoi.is_none());
    assert!(f.message.unwrap().contains("Attends 30 secondes"));
    let (v, _) = e.vue(Some(&b), Some(&k), &d, Some(offre), m(10, 0, 30));
    assert_eq!(v["attente"]["secondes"], 30);
    // Une minute après : possible
    let t2 = e.geste(&ctx(&b, &k, &d), ENVOYER, &encore, m(10, 1, 0)).envoi.expect("la suite");
    e.fini(Some(&b), tourner(&s, &t2), m(10, 1, 1));
    assert_eq!(s.recus().len(), 2);
    assert_ne!(t.brouillon.message_id, t2.brouillon.message_id);
    // (la même candidature, deux mails dans « Ce que j'ai envoyé »)
    let (l, _) = candidatures::lister(b.connexion()).unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!(pour_dossier(Some(&b), l[0].id, m(10, 1, 1).jour()).as_array().unwrap().len(), 2);
    // Le plafond du jour
    for i in 0..18 {
        let x = base_envois::commencer(
            b.connexion(),
            &base_envois::Tentative {
                offre_id: offre,
                preparation_id: None,
                cv_id: None,
                expediteur: BOITE_ADRESSE.into(),
                destinataire: "a@b-exemple.fr".into(),
                objet: "x".into(),
                contenu: "x".into(),
                piece_jointe: None,
                piece_taille: None,
                message_id: format!("p{i}@x.fr"),
            },
            m(8, 0, i),
        )
        .unwrap();
        base_envois::echec(b.connexion(), x, "x", m(8, 0, i)).unwrap();
        b.connexion().execute("UPDATE envois_tentes SET etat = 'envoye' WHERE id = ?1", [x]).unwrap();
    }
    let f = e.geste(&ctx(&b, &k, &d), ENVOYER, &encore, m(12, 0, 0));
    assert!(f.envoi.is_none() && f.message.unwrap().contains("reprends demain"));
}

#[test]
fn refus_avant_de_partir() {
    let d = dossier("refus");
    let (b, offre, p) = base(&d, false);
    let k = cles(&d);
    let mut e = Envoi::default();
    // Sans CV en fichier
    let f = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 0));
    assert!(f.envoi.is_none() && f.message.unwrap().contains("Choisir mon CV"));
    let (v, _) = e.vue(Some(&b), Some(&k), &d, Some(offre), m(10, 0, 0));
    assert!(v["cv"].is_null());
    assert_eq!(v["issue"]["genre"], "echec");
    // Un CV importé, mais son fichier effacé du PC
    let n = NouveauCv {
        nom: "CV.pdf",
        fichier: Some("cv\\cv-efface.pdf"),
        genre: GenreCv::Pdf,
        texte: "x",
        taille: Some(10),
    };
    profil::ajouter_cv(b.connexion(), &n, m(9, 0, 0)).unwrap();
    let f = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 0));
    assert!(f.envoi.is_none() && f.message.unwrap().contains("n'est plus sur ce PC"));
    let (v, _) = e.vue(Some(&b), Some(&k), &d, Some(offre), m(10, 0, 0));
    assert_eq!(v["cv"]["present"], false);
    std::fs::write(d.join("cv").join("cv-efface.pdf"), pdf()).unwrap();
    // Des adresses et des objets impossibles
    for (a, mot) in [
        ("", "Écris l'adresse"),
        ("rh@", "ne semble pas juste"),
        ("a@x-exemple.fr, b@x-exemple.fr", "Une seule adresse"),
        ("noreply@tmf-exemple.fr", "ne reçoit pas"),
        ("rh@tmf-exemple.fr\r\nBcc: espion@x.fr", "Une seule adresse"),
    ] {
        let mut v = saisie(offre, p);
        v["a"] = json!(a);
        let f = e.geste(&ctx(&b, &k, &d), ENVOYER, &v, m(10, 0, 0));
        assert!(f.envoi.is_none(), "{a}");
        assert!(f.message.as_deref().unwrap_or_default().contains(mot), "{a} : {:?}", f.message);
    }
    let mut v = saisie(offre, p);
    v["objet"] = json!("   ");
    assert!(e.geste(&ctx(&b, &k, &d), ENVOYER, &v, m(10, 0, 0)).envoi.is_none());
    // Une version d'une autre offre, ou aucune : rien
    let mut v = saisie(offre, p);
    v["version"] = json!(999);
    assert!(e.geste(&ctx(&b, &k, &d), ENVOYER, &v, m(10, 0, 0)).message.unwrap().contains("Prépare d'abord"));
    // Sans boîte branchée
    let (sans, _) = Cles::ouvrir(&dossier("sans-boite"), Box::new(Faux));
    let f = e.geste(&ctx(&b, &sans, &d), ENVOYER, &saisie(offre, p), m(10, 0, 0));
    assert!(f.envoi.is_none() && f.message.unwrap().contains("branche d'abord ta boîte"));
    // Rien n'a été noté comme tenté, rien dans le suivi
    assert_eq!(base_envois::rythme(b.connexion(), m(10, 0, 0).jour()).unwrap(), (None, 0));
    assert!(candidatures::lister(b.connexion()).unwrap().0.is_empty());
    // Et maintenant, tout est bon : la tâche part
    assert!(e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 0)).envoi.is_some());
}

/// Les pannes : rien n'est marqué envoyé, Jordan peut réessayer ; l'envoi incertain attend ce que Jordan voit dans
/// Gmail.
#[test]
fn pannes_et_envoi_incertain() {
    let d = dossier("pannes");
    let (b, offre, p) = base(&d, true);
    let k = cles(&d);
    let s = FauxSmtp::demarrer(BOITE_ADRESSE, BOITE_MDP).unwrap();
    let mut e = Envoi::default();
    let minute = std::cell::Cell::new(0);
    let essai = |e: &mut Envoi, mode: ModeSmtp| -> Fait {
        s.regler(mode);
        minute.set(minute.get() + 2);
        let minute = minute.get();
        let t = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, minute, 0)).envoi.expect("tâche");
        e.fini(Some(&b), tourner(&s, &t), m(10, minute, 1))
    };
    for (mode, mot) in [
        (ModeSmtp::MauvaisMotDePasse, "mot de passe d'application"),
        (ModeSmtp::DestinataireRefuse, "adresse de destination"),
        (ModeSmtp::MessageRefuse, "a refusé ce mail"),
        (ModeSmtp::CoupeAvant, "joindre Gmail"),
    ] {
        let f = essai(&mut e, mode);
        let t = f.message.unwrap();
        assert!(t.contains(mot) && t.contains("Rien n'a été envoyé"), "{mode:?} : {t}");
        assert!(!f.bravo);
        assert!(candidatures::lister(b.connexion()).unwrap().0.is_empty(), "{mode:?} : rien dans le suivi");
        let (v, _) = e.vue(Some(&b), Some(&k), &d, Some(offre), m(10, minute.get(), 2));
        assert_eq!(v["issue"]["genre"], "echec");
        assert!(v["incertain"].is_null());
    }
    // Coupure juste après la fin : incertain
    let f = essai(&mut e, ModeSmtp::CoupeApresFin);
    assert!(f.message.unwrap().contains("Messages envoyés"));
    assert!(candidatures::lister(b.connexion()).unwrap().0.is_empty());
    let (v, _) = e.vue(Some(&b), Some(&k), &d, Some(offre), m(10, 20, 0));
    let tente = v["incertain"]["tente"].as_i64().expect("incertain");
    assert!(v["incertain"]["texte"].as_str().unwrap().contains("Messages envoyés"));
    // Tant que Jordan n'a rien dit : pas de nouvel envoi
    s.regler(ModeSmtp::Normal);
    let f = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 30, 0));
    assert!(f.envoi.is_none() && f.message.unwrap().contains("incertain"));
    // « Il est bien parti » : dans le suivi, avec ce Message-ID
    let f = e.geste(&ctx(&b, &k, &d), PARTI, &json!({ "tente": tente }), m(10, 31, 0));
    assert!(f.bravo);
    let (l, _) = candidatures::lister(b.connexion()).unwrap();
    assert_eq!(l.len(), 1);
    let dd = pour_dossier(Some(&b), l[0].id, m(10, 31, 0).jour());
    assert!(dd[0]["titre"].as_str().unwrap().contains("Messages envoyés"));
    assert_eq!(jobmind_storage::mails::envoyes(b.connexion()).unwrap().len(), 1);
    // (un second clic ne fait rien de plus)
    let f = e.geste(&ctx(&b, &k, &d), PARTI, &json!({ "tente": tente }), m(10, 31, 1));
    assert!(!f.bravo);
    assert_eq!(base_envois::du_dossier(b.connexion(), l[0].id).unwrap().len(), 1);
}

#[test]
fn incertain_puis_pas_parti_on_peut_renvoyer() {
    let d = dossier("pas-parti");
    let (b, offre, p) = base(&d, true);
    let k = cles(&d);
    let s = FauxSmtp::demarrer(BOITE_ADRESSE, BOITE_MDP).unwrap();
    s.regler(ModeSmtp::CoupeApresFin);
    let mut e = Envoi::default();
    let t = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 0)).envoi.unwrap();
    e.fini(Some(&b), tourner(&s, &t), m(10, 0, 1));
    let tente = base_envois::incertain(b.connexion(), offre).unwrap().unwrap().id;
    let f = e.geste(&ctx(&b, &k, &d), PAS_PARTI, &json!({ "tente": tente }), m(10, 5, 0));
    assert!(f.message.unwrap().contains("de nouveau"));
    assert!(candidatures::lister(b.connexion()).unwrap().0.is_empty());
    s.regler(ModeSmtp::Normal);
    let t = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 6, 0)).envoi.expect("renvoi permis");
    assert!(e.fini(Some(&b), tourner(&s, &t), m(10, 6, 1)).bravo);
}

/// Un résultat qui arrive pour un autre envoi, ou deux fois : laissé.
#[test]
fn resultat_inconnu_laisse() {
    let d = dossier("inconnu");
    let (b, offre, p) = base(&d, true);
    let k = cles(&d);
    let mut e = Envoi::default();
    let f = e.fini(Some(&b), Retour { jeton: 42, resultat: Ok("x@y".into()) }, m(10, 0, 0));
    assert!(f.message.is_none());
    let t = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 0)).envoi.unwrap();
    let ok = Retour { jeton: t.jeton, resultat: Ok(t.brouillon.message_id.clone()) };
    assert!(e.fini(Some(&b), ok.clone(), m(10, 0, 1)).bravo);
    assert!(!e.fini(Some(&b), ok, m(10, 0, 2)).bravo);
    assert_eq!(candidatures::lister(b.connexion()).unwrap().0.len(), 1);
}

#[test]
fn textes_sans_tiret_long() {
    let t = textes().to_string();
    assert!(!t.contains('\u{2014}') && !t.contains('\u{2013}'));
    let src = include_str!("../envoi.rs");
    assert!(!src.contains('\u{2014}') && !src.contains('\u{2013}'));
}

/// Rien d'inventé : une phrase « à vérifier » encore dans la lettre gardée bloque l'envoi ; « C'est juste » le permet.
#[test]
fn phrase_a_verifier_bloque_l_envoi() {
    use jobmind_storage::preparations::PhraseAVerifier;
    let d = dossier("a-verifier");
    let (b, offre, p) = base(&d, true);
    let k = cles(&d);
    let phrase = "J'ai chargé et déchargé des camions pendant deux ans à Woippy.";
    let mut l =
        vec![PhraseAVerifier { texte: phrase.into(), raison: "pas dans ton CV".into(), code: "x".into(), vue: false }];
    base_prep::changer_a_verifier(b.connexion(), p, &l).unwrap();
    let mut e = Envoi::default();
    let f = e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 0));
    assert!(f.envoi.is_none() && f.message.unwrap().contains("à vérifier"));
    let (v, _) = e.vue(Some(&b), Some(&k), &d, Some(offre), m(10, 0, 0));
    assert_eq!(v["a_verifier"], json!([phrase]));
    l[0].vue = true;
    base_prep::changer_a_verifier(b.connexion(), p, &l).unwrap();
    assert!(e.geste(&ctx(&b, &k, &d), ENVOYER, &saisie(offre, p), m(10, 0, 0)).envoi.is_some());
}
