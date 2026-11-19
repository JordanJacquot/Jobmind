use super::*;
use crate::candidature::Moyen;

fn j(m: u32, d: u32) -> Jour {
    Jour::nouveau(2026, m, d).unwrap()
}

fn cand(id: i64, e: &str, p: &str, statut: Statut, envoi: Jour) -> Candidature {
    Candidature {
        id,
        entreprise: e.into(),
        poste: p.into(),
        ville: "Metz".into(),
        statut,
        statut_le: envoi,
        envoyee_le: envoi,
        moyen: Moyen::Mail,
        source: String::new(),
        lien: String::new(),
        envoye: String::new(),
        cv: String::new(),
        notes: String::new(),
        relance_le: None,
        entretien: None,
    }
}

/// Le suivi inventé des essais (les mêmes entreprises que les exemples de la construction d'essai).
fn suivi() -> Vec<Candidature> {
    vec![
        cand(1, "Transports Moselle Fret", "Agent de quai", Statut::Envoyee, j(11, 3)),
        cand(2, "Logistique des Trois Frontières", "Préparateur de commandes", Statut::Entretien, j(11, 6)),
        cand(3, "Résidence Les Tilleuls", "Agent de service", Statut::Envoyee, j(11, 5)),
        cand(4, "Netéa Services", "Agent d'entretien", Statut::Envoyee, j(11, 17)),
        cand(5, "Brico Lorraine", "Vendeur rayon outillage", Statut::Relancee, j(10, 31)),
        cand(6, "Lidl", "Employé libre-service", Statut::Envoyee, j(11, 10)),
        cand(7, "Brico Lorraine", "Hôte de caisse", Statut::Envoyee, j(11, 12)),
    ]
}

fn mail(de: &str, nom: &str, objet: &str, texte: &str) -> MailVu {
    MailVu {
        de_adresse: de.into(),
        de_nom: nom.into(),
        objet: objet.into(),
        texte: texte.into(),
        jour: Some(j(11, 19)),
        ..MailVu::default()
    }
}

fn sur(r: &Rattachement) -> Option<i64> {
    match r {
        Rattachement::Sur { candidature, .. } => Some(*candidature),
        _ => None,
    }
}

#[test]
fn adresse_et_mot_de_passe() {
    assert_eq!(verifier_adresse("  Jordan.Exemple@Gmail.com ").unwrap(), "jordan.exemple@gmail.com");
    assert_eq!(verifier_adresse("<prenom+jobs@exemple.fr>").unwrap(), "prenom+jobs@exemple.fr");
    for faux in ["", "jordan", "jordan@", "@gmail.com", "a@b", "a b@gmail.com", "a@@gmail.com", "é@gmail.com", "a@.com"]
    {
        assert!(verifier_adresse(faux).is_err(), "{faux}");
    }
    assert_eq!(verifier_mot_de_passe("abcd efgh ijkl mnop").unwrap(), "abcdefghijklmnop");
    assert_eq!(verifier_mot_de_passe(" ABCDEFGHIJKLMNOP\n").unwrap(), "abcdefghijklmnop");
    for (faux, attendu) in [
        ("", "Colle ici"),
        ("abcd efgh ijkl", "16 lettres"),
        ("MonMotDePasse2026!", "que des lettres"),
        ("abcdefghijklmnopq", "16 lettres"),
    ] {
        let e = verifier_mot_de_passe(faux).unwrap_err();
        assert!(e.contains(attendu), "{faux} : {e}");
        assert!(faux.len() < 4 || !e.contains(faux), "le mot de passe est répété : {e}");
    }
    assert_eq!(apercu_mot_de_passe("abcdefghijklmnop"), "se termine par mnop");
}

#[test]
fn rattachements_surs() {
    let l = suivi();
    // Le domaine de l'entreprise, l'objet qui cite le poste
    let m = mail(
        "recrutement@transports-moselle-fret.fr",
        "Service recrutement",
        "Votre candidature au poste d'agent de quai",
        "Bonjour, nous avons bien reçu votre candidature.",
    );
    assert_eq!(sur(&rattacher(&m, &l, &[])), Some(1));
    // Le nom de l'entreprise dans le nom de l'expéditeur, par un outil de recrutement
    let m = mail(
        "no-reply@teamtailor.com",
        "Netéa Services",
        "Merci pour votre candidature",
        "Bonjour Jordan, merci pour votre candidature au poste d'agent d'entretien.",
    );
    assert_eq!(sur(&rattacher(&m, &l, &[])), Some(4));
    // Une réponse à un mail envoyé par Jobmind (le Message-ID cité) : sûr, même sans autre indice
    let mut m = mail("x@gmail.com", "Sophie", "Re: Candidature", "Bonjour, je reviens vers vous.");
    m.en_reponse_a = vec!["<jobmind-7@exemple.fr>".into()];
    let envoyes = [Envoye { candidature: 3, message_id: "jobmind-7@exemple.fr".into() }];
    assert_eq!(
        rattacher(&m, &l, &envoyes),
        Rattachement::Sur { candidature: 3, raison: "il répond au mail envoyé par Jobmind".into() }
    );
    // Deux candidatures chez la même entreprise : le poste départage
    let m = mail(
        "rh@brico-lorraine.fr",
        "Brico Lorraine",
        "Votre candidature : hôte de caisse",
        "Nous souhaiterions vous rencontrer.",
    );
    assert_eq!(sur(&rattacher(&m, &l, &[])), Some(7));
}

#[test]
fn rattachements_douteux_et_contre_epreuves() {
    let l = suivi();
    // Deux candidatures chez Brico Lorraine, sans le poste : à ranger, pas rangé au hasard
    let m = mail("rh@brico-lorraine.fr", "Brico Lorraine", "Votre candidature", "Merci pour votre candidature.");
    assert!(matches!(rattacher(&m, &l, &[]), Rattachement::ARanger { suggestion: Some(5 | 7), .. }), "{m:?}");
    // Une boîte Gmail, le poste seulement : à ranger avec la candidature probable
    let m = mail(
        "rh.recrutement75@gmail.com",
        "Ressources humaines",
        "Votre candidature",
        "Nous avons bien reçu votre candidature pour le poste de préparateur de commandes.",
    );
    assert_eq!(
        rattacher(&m, &l, &[]),
        Rattachement::ARanger { suggestion: Some(2), raison: "Peut-être : le mail cite le poste.".into() }
    );
    // Un mail de recrutement sans aucun indice : à ranger, sans candidature proposée
    let m = mail("talents@cabinet-exemple.fr", "Cabinet Exemple", "Votre profil", "Votre profil nous intéresse.");
    assert_eq!(
        rattacher(&m, &l, &[]).clone(),
        Rattachement::ARanger { suggestion: None, raison: "Ce mail parle d'une candidature.".into() }
    );
    // Contre-épreuves : rien à voir, pas gardé
    for m in [
        mail("news@brico-lorraine.fr", "Brico Lorraine", "Les promos de la semaine", "Perceuses à moitié prix."),
        mail("maman@orange.fr", "Maman", "Dimanche", "Tu viens manger dimanche ?"),
        mail("info@lidl-promo.fr", "Promo", "Arrivage", "Nouveaux produits en magasin."),
        mail("facture@edf.fr", "EDF", "Votre facture", "Votre facture de novembre est disponible."),
    ] {
        assert_eq!(rattacher(&m, &l, &[]), Rattachement::Rien, "{m:?}");
    }
    // Un envoi de masse de l'entreprise qui ne parle pas de recrutement : rien, même avec son domaine
    let mut m = mail("news@transports-moselle-fret.fr", "Transports Moselle Fret", "Notre lettre", "Nos camions.");
    m.masse = true;
    assert_eq!(rattacher(&m, &l, &[]), Rattachement::Rien);
    // Un mail plus ancien que la candidature ne lui répond pas
    let mut m = mail("rh@netea-services.fr", "Netéa Services", "Votre candidature", "Merci.");
    m.jour = Some(j(11, 2));
    assert_ne!(sur(&rattacher(&m, &l, &[])), Some(4));
    // Un nom d'entreprise trop court ou trop courant n'est pas reconnu dans un mot plus long
    let l2 = vec![cand(9, "MA", "Cariste", Statut::Envoyee, j(11, 1))];
    let m = mail("x@marketing.fr", "Marketing", "Votre candidature", "Nous avons bien reçu votre candidature.");
    assert_ne!(sur(&rattacher(&m, &l2, &[])), Some(9));
    // « Lidl » (4 lettres) est reconnu dans l'objet
    let m =
        mail("noreply@jobs.example.com", "Recrutement", "Lidl : votre candidature", "Merci pour votre candidature.");
    assert_eq!(sur(&rattacher(&m, &l, &[])), Some(6));
    // Les domaines de tout le monde et des outils ne sont jamais celui d'une entreprise
    assert!(domaine_commun("gmail.com") && domaine_commun("eu.myworkday.com") && domaine_commun("emails.indeed.com"));
    assert!(!domaine_commun("transports-moselle-fret.fr"));
}

/// Des réponses écrites à part, comme les envoient les recruteurs (des tournures variées).
#[test]
fn statuts_proposes() {
    let cas: &[(&str, &str, Option<Proposition>)] = &[
        (
            "Votre candidature",
            "Madame, Monsieur,\nNous vous remercions de l'intérêt porté à notre société. Après étude de votre dossier, \
             nous sommes au regret de vous informer que votre candidature n'a pas été retenue.\nCordialement",
            Some(Proposition::Refus),
        ),
        (
            "Réponse à votre candidature",
            "Bonjour,\nMalheureusement, nous ne pouvons pas donner une suite favorable à votre candidature.",
            Some(Proposition::Refus),
        ),
        (
            "Agent de quai",
            "Bonjour, le poste a été pourvu en interne. Nous gardons votre CV.",
            Some(Proposition::Refus),
        ),
        (
            "Candidature",
            "Bonjour Jordan,\nVotre profil a retenu notre attention. Nous souhaiterions vous rencontrer pour un \
             entretien. Seriez-vous disponible jeudi 26 novembre à 10 h ?\nBien à vous",
            Some(Proposition::Entretien),
        ),
        (
            "Entretien téléphonique",
            "Bonjour, pouvez-vous me donner vos disponibilités pour un échange téléphonique cette semaine ?",
            Some(Proposition::Entretien),
        ),
        (
            "Bonne nouvelle",
            "Bonjour, nous avons le plaisir de vous annoncer que votre candidature a été retenue. Vous commencez lundi.",
            Some(Proposition::Positive),
        ),
        (
            "Votre dossier",
            "Bonjour, merci de nous transmettre une copie de votre CACES et de votre permis.",
            Some(Proposition::Infos),
        ),
        (
            "Accusé de réception",
            "Bonjour, nous avons bien reçu votre candidature. Si votre profil correspond, nous vous proposerons un \
             entretien. Sans réponse de notre part sous trois semaines, considérez que votre candidature n'a pas été \
             retenue.",
            Some(Proposition::Accuse),
        ),
        ("Merci", "Merci pour votre candidature, elle sera étudiée avec attention.", Some(Proposition::Accuse)),
    ];
    for (objet, texte, attendu) in cas {
        let lu = proposer(objet, texte);
        assert_eq!(lu.proposition.as_ref().map(|(p, _)| *p), *attendu, "{texte}");
        // La preuve est une phrase du mail, telle quelle
        if let Some((_, c)) = &lu.proposition {
            assert!(texte.contains(c.as_str()) || objet.contains(c.as_str()), "preuve inventée : {c}");
        }
    }
    // Des phrases qui se contredisent, ou rien de clair : flou (Claude ou Jordan)
    let lu = proposer(
        "Votre candidature",
        "Nous ne pouvons pas donner suite pour le poste de cariste, mais nous souhaiterions vous rencontrer pour un \
         autre poste.",
    );
    assert!(lu.flou && lu.proposition.is_none());
    let lu = proposer("Infos", "Bonjour Jordan, je vous rappelle demain matin au sujet de notre échange d'hier.");
    assert!(lu.flou && lu.proposition.is_none());
    assert!(!proposer("Ok", "Merci").flou);
    // Le mail cité dessous ne compte pas
    let lu = proposer(
        "Re: Candidature",
        "Bonjour, je vous confirme notre rendez-vous de jeudi, je vous rappelle.\n\nLe 18 nov. 2026, Jordan a écrit :\n\
         > nous ne pouvons pas donner suite",
    );
    assert_ne!(lu.proposition.map(|(p, _)| p), Some(Proposition::Refus));
    for p in Proposition::TOUTES {
        assert_eq!(Proposition::depuis_code(p.code()), Some(p));
        assert!(!p.nom().contains(['\u{2013}', '\u{2014}']));
    }
    assert_eq!(Proposition::Refus.statut(), Some(Statut::Negative));
    assert_eq!(Proposition::Infos.statut(), None);
}

/// Un texte immense ou bizarre ne fait jamais paniquer ni durer.
#[test]
fn textes_pieges() {
    let l = suivi();
    let gros = "Nous avons bien reçu votre candidature. ".repeat(20_000);
    let t = std::time::Instant::now();
    let _ = proposer(&"x".repeat(10_000), &gros);
    let _ = rattacher(&mail("a@b.fr", "", "", &gros), &l, &[]);
    assert!(t.elapsed().as_secs() < 5);
    for t in ["", "\u{0}\u{1}", "....!!!???", "é\u{301}", ">>>> > >", "\n\n\n"] {
        let _ = proposer(t, t);
        let _ = rattacher(&mail(t, t, t, t), &l, &[]);
    }
}
