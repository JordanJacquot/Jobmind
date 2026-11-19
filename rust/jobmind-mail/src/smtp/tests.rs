//! Les tests de l'envoi, contre le faux serveur SMTP local (aucun mail ne part).

use super::*;
use crate::faux_smtp::{FauxSmtp, ModeSmtp, base64_lu};
use std::time::Duration;

const DE: &str = "jordan.essai@gmail.com";
const MDP: &str = "zqvxkmtrpwlahfeo";
const A: &str = "recrutement@logipole-exemple.fr";

fn date() -> DateMail {
    DateMail { annee: 2026, mois: 11, jour: 19, heure: 14, minute: 5, seconde: 9, decalage_minutes: 60 }
}

/// Un faux PDF : des octets de toutes les valeurs (0 à 255), dont des fins de ligne et des points.
fn pdf() -> Vec<u8> {
    let mut o = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    for i in 0..5000u32 {
        o.push((i * 7 % 256) as u8);
    }
    o.extend_from_slice(b"\n.\n..\r\n%%EOF\n");
    o
}

fn brouillon(texte: &str, objet: &str, piece: Option<PieceJointe>) -> Brouillon {
    Brouillon {
        de: DE.into(),
        de_nom: "Jordan Essai".into(),
        a: A.into(),
        objet: objet.into(),
        texte: texte.into(),
        piece,
        message_id: nouveau_message_id(DE),
        date: date(),
    }
}

fn piece() -> Option<PieceJointe> {
    Some(PieceJointe { nom: "CV Jérôme 2026.pdf".into(), type_mime: type_mime("x.pdf").into(), octets: pdf() })
}

const LETTRE: &str = "Madame, Monsieur,\n\nJe vous propose ma candidature au poste de préparateur de commandes. \
                      J'ai travaillé deux ans en entrepôt à Woippy.\n.Une ligne qui commence par un point.\n\nJe vous \
                      prie d'agréer, Madame, Monsieur, mes salutations distinguées.\n\nJordan Essai";
const OBJET: &str = "Candidature au poste de préparateur de commandes (réf. 2026-114, équipe été)";

fn serveur() -> FauxSmtp {
    FauxSmtp::demarrer(DE, MDP).expect("faux serveur")
}

/// Le texte d'un en-tête du message reçu (déplié).
fn entete(m: &str, nom: &str) -> Option<String> {
    let tete = m.split("\r\n\r\n").next().unwrap_or_default();
    let deplie = tete.replace("\r\n ", " ").replace("\r\n\t", " ");
    deplie.lines().find_map(|l| l.strip_prefix(&format!("{nom}: ")).map(str::to_string))
}

fn verifier_sans_mdp(f: &FauxSmtp, erreurs: &[ErreurEnvoi]) {
    for c in f.commandes() {
        assert!(!c.contains(MDP), "mot de passe noté : {c}");
        assert!(!c.contains(&base64(format!("\0{DE}\0{MDP}").as_bytes())), "AUTH en clair : {c}");
    }
    for e in erreurs {
        for t in [e.to_string(), format!("{e:?}"), e.message()] {
            assert!(!t.contains(MDP), "mot de passe dans une erreur : {t}");
        }
    }
}

/// Le test clé : le mail reçu par le serveur, relu octet par octet (en-têtes, encodages, lettre, pièce jointe), puis
/// relu par la lecture MIME de l'étape 7 (celle qui lira les réponses).
#[test]
fn envoi_normal_relu_octet_par_octet() {
    let f = serveur();
    let b = brouillon(LETTRE, OBJET, piece());
    let id = envoyer(&f.acces(DE, MDP), &b).expect("envoyé");
    assert_eq!(id, b.message_id);
    let recus = f.recus();
    assert_eq!(recus.len(), 1);
    let r = &recus[0];
    assert_eq!((r.de.as_str(), r.a.clone()), (DE, vec![A.to_string()]));
    let m = String::from_utf8(r.octets.clone()).expect("le mail est en ASCII");
    assert!(m.is_ascii(), "tout est encodé : rien que de l'ASCII");
    // Chaque ligne finie par CRLF, aucune de plus de 78 caractères (RFC 5322)
    for l in m.split("\r\n") {
        assert!(l.len() <= 78, "ligne trop longue ({}) : {l}", l.len());
        assert!(!l.contains('\r') && !l.contains('\n'));
    }
    assert!(!m.replace("\r\n", "").contains('\n'), "un LF seul");
    assert_eq!(entete(&m, "Date").as_deref(), Some("Thu, 19 Nov 2026 14:05:09 +0100"));
    assert_eq!(entete(&m, "From").as_deref(), Some(format!("\"Jordan Essai\" <{DE}>").as_str()));
    assert_eq!(entete(&m, "To").as_deref(), Some(format!("<{A}>").as_str()));
    assert_eq!(entete(&m, "Message-ID").as_deref(), Some(format!("<{}>", b.message_id).as_str()));
    assert_eq!(entete(&m, "MIME-Version").as_deref(), Some("1.0"));
    assert!(entete(&m, "Bcc").is_none() && entete(&m, "Cc").is_none());
    // L'objet : des mots encodés en UTF-8, qui redonnent l'objet exact
    let sujet = entete(&m, "Subject").expect("objet");
    let mut lu = Vec::new();
    for mot in sujet.split(' ') {
        let x = mot.strip_prefix("=?UTF-8?B?").and_then(|x| x.strip_suffix("?=")).expect("mot encodé");
        assert!(mot.len() <= 75);
        lu.extend(base64_lu(x).expect("base64"));
    }
    assert_eq!(String::from_utf8(lu).expect("UTF-8"), OBJET);
    // Les deux parties : la lettre en UTF-8, puis la pièce jointe
    let ct = entete(&m, "Content-Type").expect("type");
    let frontiere = ct.split("boundary=\"").nth(1).and_then(|x| x.split('"').next()).expect("frontière");
    let parties: Vec<&str> = m.split(&format!("--{frontiere}")).collect();
    assert_eq!(parties.len(), 4, "préambule, lettre, pièce jointe, fin");
    assert_eq!(parties[3], "--\r\n");
    let (tete1, corps1) = parties[1].split_once("\r\n\r\n").expect("partie 1");
    assert!(tete1.contains("Content-Type: text/plain; charset=UTF-8"));
    assert!(tete1.contains("Content-Transfer-Encoding: base64"));
    let lettre = String::from_utf8(base64_lu(corps1).expect("base64")).expect("UTF-8");
    assert_eq!(lettre, LETTRE.replace('\n', "\r\n"));
    let (tete2, corps2) = parties[2].split_once("\r\n\r\n").expect("partie 2");
    assert!(tete2.contains("Content-Type: application/pdf;\r\n name=\"CV Jerome 2026.pdf\""), "{tete2}");
    assert!(tete2.contains("Content-Disposition: attachment;\r\n filename=\"CV Jerome 2026.pdf\";"), "{tete2}");
    assert!(tete2.contains("filename*=UTF-8''CV%20J%C3%A9r%C3%B4me%202026.pdf"), "{tete2}");
    assert_eq!(base64_lu(corps2).expect("base64"), pdf(), "la pièce jointe, octet par octet");
    // La lecture MIME de l'étape 7 relit tout pareil
    let lu = crate::mime::lire(1, &r.octets);
    assert_eq!(lu.objet, OBJET);
    assert_eq!(lu.message_id, b.message_id);
    assert_eq!(lu.de_adresse, DE);
    assert_eq!(lu.pieces_jointes, 1);
    assert!(lu.texte.contains("préparateur de commandes"), "{}", lu.texte);
    assert!(lu.texte.contains(".Une ligne qui commence par un point."));
    // Les commandes : une session, un seul RCPT, AUTH masqué
    let c = f.commandes();
    assert_eq!(
        c,
        vec![
            "EHLO [127.0.0.1]".to_string(),
            "AUTH PLAIN *****".into(),
            format!("MAIL FROM:<{DE}>"),
            format!("RCPT TO:<{A}>"),
            "DATA".into(),
            "QUIT".into()
        ]
    );
    assert_eq!(f.sessions(), 1);
    verifier_sans_mdp(&f, &[]);
}

#[test]
fn sans_piece_jointe_un_seul_texte() {
    let f = serveur();
    envoyer(&f.acces(DE, MDP), &brouillon("Bonjour,\n\nMa candidature.\n\nJordan", "Candidature", None)).expect("ok");
    let m = String::from_utf8(f.recus()[0].octets.clone()).expect("ascii");
    assert_eq!(entete(&m, "Content-Type").as_deref(), Some("text/plain; charset=UTF-8"));
    assert_eq!(entete(&m, "Subject").as_deref(), Some("Candidature"));
    let corps = m.split_once("\r\n\r\n").expect("corps").1;
    assert_eq!(base64_lu(corps).expect("b64"), "Bonjour,\r\n\r\nMa candidature.\r\n\r\nJordan".as_bytes());
    assert_eq!(crate::mime::lire(1, &f.recus()[0].octets).pieces_jointes, 0);
}

#[test]
fn tres_longue_lettre_avec_accents() {
    let f = serveur();
    let mut t = String::from("Madame, Monsieur,\n\n");
    while t.chars().count() < 15_000 {
        t.push_str("J'ai préparé des commandes, déchargé des camions, contrôlé les réceptions à l'entrepôt ; ");
        t.push_str("les équipes étaient à l'heure, même l'été. Œuvre, cœur, « guillemets », 12 € et 1 850 €.\n");
    }
    t.push_str("\nJordan");
    let objet = "Candidature spontanée : préparateur, cariste, magasinier, réceptionnaire, agent de quai \
                 polyvalent à Thionville ou à Metz, équipe du matin ou de l'après-midi";
    let b = brouillon(&t, objet, piece());
    envoyer(&f.acces(DE, MDP), &b).expect("ok");
    let r = &f.recus()[0].octets;
    let lu = crate::mime::lire(1, r);
    assert_eq!(lu.objet, objet);
    // (la lecture de l'étape 7 garde 20 000 caractères et remet les espaces au propre : le début suffit)
    assert!(lu.texte.starts_with("Madame, Monsieur,"), "{}", &lu.texte[..80]);
    assert!(lu.texte.contains("Œuvre, cœur, « guillemets », 12 € et 1 850 €."));
    // La lettre entière, octet par octet
    let m = String::from_utf8(r.clone()).expect("ascii");
    let ct = entete(&m, "Content-Type").expect("type");
    let frontiere = ct.split("boundary=\"").nth(1).and_then(|x| x.split('"').next()).expect("frontière");
    let partie = m.split(&format!("--{frontiere}")).nth(1).expect("partie");
    let corps = partie.split_once("\r\n\r\n").expect("corps").1;
    assert_eq!(String::from_utf8(base64_lu(corps).expect("b64")).expect("utf8"), t.replace('\n', "\r\n"));
    for l in m.split("\r\n") {
        assert!(l.len() <= 78, "{l}");
    }
}

#[test]
fn refus_du_mot_de_passe_rien_ne_part() {
    let f = serveur();
    let e = envoyer(&f.acces(DE, "pasbonpasbonpasb"), &brouillon(LETTRE, OBJET, piece())).expect_err("refus");
    assert_eq!(e, ErreurEnvoi::Identifiants);
    assert!(e.rien_parti());
    f.regler(ModeSmtp::MauvaisMotDePasse);
    let e2 = envoyer(&f.acces(DE, MDP), &brouillon(LETTRE, OBJET, piece())).expect_err("refus");
    assert_eq!(e2, ErreurEnvoi::Identifiants);
    assert!(f.recus().is_empty());
    assert!(!f.commandes().iter().any(|c| c.starts_with("MAIL") || c == "DATA"));
    verifier_sans_mdp(&f, &[e, e2]);
}

#[test]
fn coupure_apres_la_fin_envoi_incertain() {
    let f = serveur();
    f.regler(ModeSmtp::CoupeApresFin);
    let e = envoyer(&f.acces(DE, MDP), &brouillon(LETTRE, OBJET, piece())).expect_err("coupé");
    assert!(matches!(e, ErreurEnvoi::Incertain(_)), "{e:?}");
    assert!(!e.rien_parti());
    assert!(e.message().contains("Messages envoyés"), "{}", e.message());
    // (le serveur avait bien tout reçu : c'est pour cela que Jobmind ne dit pas « rien n'est parti »)
    assert_eq!(f.recus().len(), 1);
    verifier_sans_mdp(&f, &[e]);
}

#[test]
fn coupure_avant_et_serveur_muet_rien_ne_part() {
    let f = serveur();
    f.regler(ModeSmtp::CoupeAvant);
    let e = envoyer(&f.acces(DE, MDP), &brouillon(LETTRE, OBJET, None)).expect_err("coupé");
    assert!(matches!(e, ErreurEnvoi::Connexion(ErreurMail::Reseau(_))), "{e:?}");
    assert!(e.rien_parti() && e.message().contains("Rien n'a été envoyé"));
    f.regler(ModeSmtp::Lent);
    let mut a = f.acces(DE, MDP);
    a.delai = Duration::from_millis(300);
    let debut = std::time::Instant::now();
    let e2 = envoyer(&a, &brouillon(LETTRE, OBJET, None)).expect_err("lent");
    assert_eq!(e2, ErreurEnvoi::Connexion(ErreurMail::Delai));
    assert!(debut.elapsed() < Duration::from_secs(5));
    assert!(f.recus().is_empty());
    // Personne n'écoute : réseau
    let port = std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).map(|a| a.port()).expect("port");
    let mut a = f.acces(DE, MDP);
    a.port = port;
    let e3 = envoyer(&a, &brouillon(LETTRE, OBJET, None)).expect_err("personne");
    assert!(matches!(e3, ErreurEnvoi::Connexion(ErreurMail::Reseau(_))), "{e3:?}");
    verifier_sans_mdp(&f, &[e, e2, e3]);
}

#[test]
fn destinataire_refuse_et_message_refuse() {
    let f = serveur();
    f.regler(ModeSmtp::DestinataireRefuse);
    let e = envoyer(&f.acces(DE, MDP), &brouillon(LETTRE, OBJET, None)).expect_err("refusé");
    assert!(matches!(e, ErreurEnvoi::DestinataireRefuse(ref t) if t.starts_with("550")), "{e:?}");
    assert!(e.rien_parti());
    assert!(!f.commandes().iter().any(|c| c == "DATA"), "pas de DATA sans destinataire");
    f.regler(ModeSmtp::MessageRefuse);
    let e2 = envoyer(&f.acces(DE, MDP), &brouillon(LETTRE, OBJET, piece())).expect_err("refusé");
    assert!(matches!(e2, ErreurEnvoi::MessageRefuse(ref t) if t.starts_with("552")), "{e2:?}");
    assert!(e2.rien_parti());
    assert!(f.recus().is_empty());
    verifier_sans_mdp(&f, &[e, e2]);
}

/// Une adresse, un objet ou une pièce jointe impossibles : rien n'est tenté (aucune connexion).
#[test]
fn brouillons_impossibles_aucune_connexion() {
    let f = serveur();
    let a = f.acces(DE, MDP);
    let mut cas: Vec<Brouillon> = Vec::new();
    for adr in [
        "",
        "pas une adresse",
        "a@b",
        "deux@@exemple.fr",
        "x@exemple.fr, autre@exemple.fr",
        "x@exemple.fr\r\nBcc: autre@exemple.fr",
        "<x@exemple.fr>",
        "jérôme@exemple.fr",
        "x@-exemple.fr",
        ".x@exemple.fr",
        "x..y@exemple.fr",
        "x@exemple..fr",
    ] {
        cas.push(Brouillon { a: adr.into(), ..brouillon(LETTRE, OBJET, None) });
    }
    cas.push(brouillon(LETTRE, "Candidature\r\nBcc: espion@exemple.fr", None));
    cas.push(brouillon(LETTRE, "   ", None));
    cas.push(brouillon(LETTRE, &"x".repeat(OBJET_MAX + 1), None));
    cas.push(brouillon("  \n ", OBJET, None));
    cas.push(brouillon(&"é".repeat(TEXTE_MAX + 1), OBJET, None));
    cas.push(brouillon(
        LETTRE,
        OBJET,
        Some(PieceJointe { nom: "CV.pdf".into(), type_mime: "application/pdf".into(), octets: Vec::new() }),
    ));
    cas.push(brouillon(
        LETTRE,
        OBJET,
        Some(PieceJointe { nom: "CV.pdf".into(), type_mime: "application/pdf".into(), octets: vec![1; PIECE_MAX + 1] }),
    ));
    cas.push(Brouillon { de_nom: "Jordan\r\nBcc: x@exemple.fr".into(), ..brouillon(LETTRE, OBJET, None) });
    cas.push(Brouillon { message_id: "pas un id".into(), ..brouillon(LETTRE, OBJET, None) });
    // (l'expéditeur doit être la boîte branchée)
    cas.push(Brouillon { de: "autre@gmail.com".into(), ..brouillon(LETTRE, OBJET, None) });
    for b in &cas {
        let e = envoyer(&a, b).expect_err("refusé");
        assert!(matches!(e, ErreurEnvoi::Brouillon(_)), "{:?} {:?} : {e:?}", b.a, b.objet);
        assert!(e.rien_parti());
    }
    assert_eq!(f.sessions(), 0, "aucune connexion pour un mail impossible");
}

#[test]
fn essayer_sans_rien_envoyer() {
    let f = serveur();
    essayer_envoi(&f.acces(DE, MDP)).expect("ok");
    assert_eq!(f.commandes(), vec!["EHLO [127.0.0.1]".to_string(), "AUTH PLAIN *****".into(), "QUIT".into()]);
    assert!(f.recus().is_empty());
    assert_eq!(essayer_envoi(&f.acces(DE, "faux")), Err(ErreurEnvoi::Identifiants));
    assert!(!f.commandes().iter().any(|c| c.starts_with("MAIL") || c.starts_with("RCPT") || c == "DATA"));
    verifier_sans_mdp(&f, &[]);
}

#[test]
fn sans_tls_seulement_vers_127_0_0_1() {
    for hote in ["localhost", "192.0.2.1", "smtp.exemple.invalid", "127.0.0.2"] {
        let a = Acces {
            hote: hote.into(),
            port: 1,
            tls: false,
            adresse: DE.into(),
            mdp: MDP.into(),
            delai: Duration::from_millis(100),
        };
        let e = envoyer(&a, &brouillon(LETTRE, OBJET, None)).expect_err("refusé");
        assert_eq!(e, ErreurEnvoi::Connexion(ErreurMail::TlsObligatoire));
        assert_eq!(essayer_envoi(&a), Err(ErreurEnvoi::Connexion(ErreurMail::TlsObligatoire)));
    }
    let g = Acces::gmail_envoi(DE, MDP);
    assert_eq!((g.hote.as_str(), g.port, g.tls), ("smtp.gmail.com", 465, true));
    assert!(!format!("{g:?}").contains(MDP));
}

#[test]
fn le_point_en_debut_de_ligne_est_double_puis_retire() {
    assert_eq!(pour_data(b"a\r\n.b\r\n..c\r\n"), b"a\r\n..b\r\n...c\r\n.\r\n");
    assert_eq!(pour_data(b".x"), b"..x\r\n.\r\n");
}

#[test]
fn dates_et_jours_de_la_semaine() {
    let d =
        |a, m, j, dec| DateMail { annee: a, mois: m, jour: j, heure: 0, minute: 0, seconde: 0, decalage_minutes: dec };
    assert_eq!(date_rfc5322(&d(2026, 11, 18, 60)), "Wed, 18 Nov 2026 00:00:00 +0100");
    assert_eq!(date_rfc5322(&d(2026, 7, 14, 120)), "Tue, 14 Jul 2026 00:00:00 +0200");
    assert_eq!(date_rfc5322(&d(2000, 2, 29, -300)), "Tue, 29 Feb 2000 00:00:00 -0500");
    assert_eq!(date_rfc5322(&d(1970, 1, 1, 0)), "Thu, 1 Jan 1970 00:00:00 +0000");
    assert_eq!(date_rfc5322(&d(2027, 1, 1, 330)), "Fri, 1 Jan 2027 00:00:00 +0530");
}

#[test]
fn message_id_unique_et_sur() {
    let mut vus = std::collections::HashSet::new();
    for _ in 0..2000 {
        let id = nouveau_message_id(DE);
        assert!(id.ends_with(".jobmind@gmail.com"), "{id}");
        assert_eq!(id.split('.').next().map(str::len), Some(32));
        assert!(vus.insert(id));
    }
    assert!(nouveau_message_id("bizarre").ends_with("@jobmind"));
}

#[test]
fn encodages() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64("é".as_bytes()), "w6k=");
    // (un mot encodé ne coupe jamais un caractère : chaque mot se relit seul en UTF-8)
    let t = "€".repeat(40);
    for mot in mot_encode(&t).split("\r\n ") {
        let x = mot.trim_start_matches("=?UTF-8?B?").trim_end_matches("?=");
        assert!(String::from_utf8(base64_lu(x).expect("b64")).is_ok());
        assert!(mot.len() <= 75);
    }
    assert_eq!(nom_piece_sur("C:\\Users\\x\\CV \"final\".pdf"), "CV final.pdf");
    assert_eq!(nom_piece_sur("../../.."), "CV.pdf");
    assert_eq!(type_mime("Mon CV.DOCX"), "application/vnd.openxmlformats-officedocument.wordprocessingml.document");
}

#[test]
fn messages_simples_sans_tiret_long() {
    let toutes = [
        ErreurEnvoi::Brouillon("x".into()),
        ErreurEnvoi::Connexion(ErreurMail::Reseau("x".into())),
        ErreurEnvoi::Identifiants,
        ErreurEnvoi::ExpediteurRefuse("x".into()),
        ErreurEnvoi::DestinataireRefuse("x".into()),
        ErreurEnvoi::Refus("x".into()),
        ErreurEnvoi::MessageRefuse("x".into()),
        ErreurEnvoi::Incertain("x".into()),
        ErreurEnvoi::Illisible("x".into()),
    ];
    for e in &toutes {
        let m = e.message();
        assert!(m.ends_with('.') && !m.contains('\u{2014}') && !m.contains('\u{2013}'), "{m}");
        assert!(!m.contains("SMTP") && !m.contains("TLS"), "{m}");
        assert_eq!(e.rien_parti(), m.contains("Rien n'a été envoyé"), "{m}");
    }
}

/// Un nom de pièce jointe très long, avec des accents : découpé en morceaux numérotés, aucune ligne trop longue.
#[test]
fn nom_de_piece_tres_long() {
    let nom = format!("{} Été.pdf", "Curriculum vitæ de Jérôme, préparateur à l'entrepôt".repeat(3));
    let p = PieceJointe { nom, type_mime: "application/pdf".into(), octets: pdf() };
    let m = String::from_utf8(composer(&brouillon(LETTRE, OBJET, Some(p))).expect("composé")).expect("ascii");
    for l in m.split("\r\n") {
        assert!(l.len() <= 78, "{l}");
    }
    assert!(m.contains(" filename*0*=UTF-8''Curriculum%20vit%C3%A6"), "{m}");
    assert!(m.contains(" filename*1*="));
    assert_eq!(crate::mime::lire(1, &m.into_bytes()).pieces_jointes, 1);
}
