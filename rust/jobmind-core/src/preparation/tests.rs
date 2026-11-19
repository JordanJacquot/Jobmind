use super::*;

const CV: &str = "Jordan Exemple\nMetz (57), permis B\n\nExpérience\n2023 à 2025 : préparateur de commandes et cariste en \
                  intérim, entrepôt de Woippy\n2021 à 2022 : vendeur dans un magasin de sport, Metz\n\nFormation\nCAP \
                  Opérateur logistique, 2018\nCACES 1, 3 et 5";
const OFFRE: &str = "Agent de quai H/F. Chargement des camions, CACES 3 exigé. Poste en 2x8. Pourquoi souhaitez-vous \
                     rejoindre notre équipe ?";

fn s() -> Sources<'static> {
    Sources { profil: CV, offre: OFFRE }
}

fn ph(t: &str, sur_moi: bool, extraits: &[&str]) -> PhraseRendue {
    PhraseRendue { texte: t.into(), sur_moi, extraits: extraits.iter().map(|e| (*e).to_string()).collect() }
}

#[test]
fn une_phrase_soutenue_par_le_cv() {
    let p =
        verifier_phrase(&ph("J'ai travaillé comme cariste en intérim à Woippy.", true, &["cariste en intérim"]), s())
            .unwrap();
    assert_eq!(p.a_verifier, None);
    assert_eq!(p.appuis, vec!["cariste en intérim".to_string()]);
    // (sans accents ni majuscules, la ponctuation changée : trouvé quand même)
    let p = verifier_phrase(&ph("J'ai les CACES 1, 3 et 5.", true, &["caces 1 3 et 5"]), s()).unwrap();
    assert_eq!(p.a_verifier, None);
}

#[test]
fn une_phrase_non_soutenue_est_a_verifier() {
    // Un passage qui n'est pas dans le CV
    let p = verifier_phrase(&ph("J'ai encadré une équipe de dix personnes.", true, &["chef d'équipe"]), s()).unwrap();
    assert_eq!(p.a_verifier, Some(Raison::ExtraitAbsent));
    // Aucun passage
    let p = verifier_phrase(&ph("Je suis disponible tout de suite.", true, &[]), s()).unwrap();
    assert_eq!(p.a_verifier, Some(Raison::SansExtrait));
    // Claude dit qu'elle ne parle pas de Jordan, mais elle en parle
    let p = verifier_phrase(&ph("Je suis titulaire du permis C.", false, &[]), s()).unwrap();
    assert_eq!(p.a_verifier, Some(Raison::FaitNonDeclare));
    // Un passage d'un seul mot ne suffit pas
    let p = verifier_phrase(&ph("J'ai été cariste.", true, &["cariste"]), s()).unwrap();
    assert_eq!(p.a_verifier, Some(Raison::ExtraitAbsent));
}

#[test]
fn un_nombre_absent_est_a_verifier() {
    // « cinq ans » : ni 5 ans ni cinq dans le CV... mais 5 y est (CACES 5) : la vérification des nombres est large ;
    // « 7 ans » n'y est pas
    let p = verifier_phrase(&ph("J'ai 7 ans d'expérience en entrepôt.", true, &["entrepôt de Woippy"]), s()).unwrap();
    assert_eq!(p.a_verifier, Some(Raison::NombreAbsent("7".into())));
    let p =
        verifier_phrase(&ph("J'ai travaillé quatre ans en magasin.", true, &["vendeur dans un magasin"]), s()).unwrap();
    assert_eq!(p.a_verifier, Some(Raison::NombreAbsent("quatre".into())));
    // Les nombres de l'offre comptent aussi (« 2x8 » : ni 2 ni 8 isolés, mais « 2x8 » n'est pas un nombre)
    let p = verifier_phrase(&ph("Le CACES 3 que vous demandez, je l'ai.", true, &["CACES 1, 3 et 5"]), s()).unwrap();
    assert_eq!(p.a_verifier, None);
    // Une phrase libre (la politesse) avec un nombre inconnu
    let p = verifier_phrase(&ph("Je peux venir dans 15 jours.", false, &[]), s()).unwrap();
    assert_eq!(p.a_verifier, Some(Raison::NombreAbsent("15".into())));
}

#[test]
fn les_phrases_libres_restent_libres() {
    for t in [
        "Votre offre d'agent de quai a retenu mon attention.",
        "Je serais heureux de vous rencontrer pour en parler.",
        "Je vous prie d'agréer, Madame, Monsieur, mes salutations distinguées.",
    ] {
        assert_eq!(verifier_phrase(&ph(t, false, &[]), s()).unwrap().a_verifier, None, "{t}");
    }
    assert!(verifier_phrase(&ph("   ", false, &[]), s()).is_none());
    // Coupée, sans caractères de contrôle
    let long = format!("{}\u{7}", "a ".repeat(800));
    let p = verifier_phrase(&ph(&long, false, &[]), s()).unwrap();
    assert!(p.texte.chars().count() <= PHRASE_MAX && !p.texte.contains('\u{7}'));
}

#[test]
fn parle_de_moi_reconnait_le_parcours() {
    assert!(parle_de_moi("Fort de mon expérience en logistique"));
    assert!(parle_de_moi("Je suis diplômé d'un CAP"));
    assert!(parle_de_moi("J'ai déjà conduit des chariots"));
    assert!(!parle_de_moi("Je vous remercie de l'attention portée à ma candidature."));
    assert!(!parle_de_moi("Votre entreprise a bonne réputation."));
}

#[test]
fn les_questions_inventees_sont_laissees() {
    let q = [
        QuestionRendue {
            question: "Pourquoi souhaitez-vous rejoindre notre équipe ?".into(),
            extrait_offre: "Pourquoi souhaitez-vous rejoindre notre équipe ?".into(),
            reponse: vec![ph(
                "J'ai travaillé en entrepôt à Woippy et ce métier me plaît.",
                true,
                &["entrepôt de Woippy"],
            )],
        },
        QuestionRendue {
            question: "Quel salaire souhaitez-vous ?".into(),
            extrait_offre: "Indiquez vos prétentions salariales".into(),
            reponse: vec![ph("Autour de 1 900 euros.", false, &[])],
        },
    ];
    let (l, laissees) = verifier_questions(&q, s());
    assert_eq!((l.len(), laissees), (1, 1));
    assert!(l[0].texte().contains("Woippy"));
}

#[test]
fn les_conseils_s_appuient_sur_le_cv_ou_l_offre() {
    let c = [
        ConseilRendu {
            genre: "mettre_en_avant".into(),
            texte: "Mets ton CACES 3 en haut du CV.".into(),
            extrait_cv: "CACES 1, 3 et 5".into(),
            extrait_offre: "CACES 3 exigé".into(),
        },
        ConseilRendu {
            genre: "mettre_en_avant".into(),
            texte: "Mets en avant ton BTS.".into(),
            extrait_cv: "BTS logistique".into(),
            extrait_offre: String::new(),
        },
        ConseilRendu {
            genre: "absent".into(),
            texte: "L'offre parle de chargement des camions.".into(),
            extrait_cv: String::new(),
            extrait_offre: "Chargement des camions".into(),
        },
        ConseilRendu {
            genre: "inventer".into(),
            texte: "x".into(),
            extrait_cv: String::new(),
            extrait_offre: String::new(),
        },
    ];
    let (l, laisses) = verifier_conseils(&c, CV, OFFRE);
    assert_eq!((l.len(), laisses), (2, 2));
    assert_eq!(l[0].genre, GenreConseil::MettreEnAvant);
    assert!(l[1].texte.ends_with(SEULEMENT_SI_VRAI) && l[1].extrait_cv.is_empty());
}

#[test]
fn la_lettre_et_ses_morceaux() {
    let p =
        |t: &str, v: bool| Phrase { texte: t.into(), appuis: Vec::new(), a_verifier: v.then_some(Raison::SansExtrait) };
    let l = Lettre {
        appel: "Madame, Monsieur,".into(),
        paragraphes: vec![vec![p("Votre offre m'intéresse.", false), p("Je suis disponible tout de suite.", true)]],
        politesse: "Je vous prie d'agréer mes salutations distinguées.".into(),
        signature: verifier_signature("Jordan Exemple", CV),
    };
    let t = l.texte();
    assert_eq!(
        t,
        "Madame, Monsieur,\n\nVotre offre m'intéresse. Je suis disponible tout de suite.\n\nJe vous prie d'agréer mes \
         salutations distinguées.\nJordan Exemple"
    );
    assert_eq!(l.a_verifier().len(), 1);
    let m = morceaux(&t, &["Je suis disponible tout de suite.".to_string()]);
    assert_eq!(m.len(), 3);
    assert_eq!(m[1].a_verifier, Some(0));
    assert_eq!(m.iter().map(|x| x.texte.as_str()).collect::<String>(), t);
    // Une phrase changée par Jordan n'est plus surlignée
    let m = morceaux("Je suis libre dès lundi.", &["Je suis disponible tout de suite.".to_string()]);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].a_verifier, None);
    // Retirer la phrase
    let r = retirer_phrase(&t, "Je suis disponible tout de suite.").unwrap();
    assert!(r.contains("Votre offre m'intéresse.\n\nJe vous prie"), "{r}");
    let r2 = retirer_phrase("A.\n\nSeule.\n\nB.", "Seule.").unwrap();
    assert_eq!(r2, "A.\n\nB.");
    assert!(retirer_phrase(&t, "absente").is_none());
    // Une signature absente du CV n'est pas gardée
    assert_eq!(verifier_signature("Jean Dupont", CV), "");
}

#[test]
fn la_lettre_modifiee_et_la_consigne() {
    assert_eq!(verifier_lettre("  Bonjour,\r\n\r\nMerci.\u{0}  ").unwrap(), "Bonjour,\n\nMerci.");
    assert!(verifier_lettre("   ").is_err());
    assert!(verifier_lettre(&"a".repeat(LETTRE_MAX + 1)).is_err());
    assert_eq!(verifier_consigne("  plus   court ").unwrap(), Some("plus court".into()));
    assert_eq!(verifier_consigne("").unwrap(), None);
    assert!(verifier_consigne(&"x".repeat(CONSIGNE_MAX + 1)).is_err());
    for r in [Raison::SansExtrait, Raison::ExtraitAbsent, Raison::NombreAbsent("3".into()), Raison::FaitNonDeclare] {
        assert!(!r.texte().contains(['\u{2013}', '\u{2014}']) && !r.code().is_empty());
    }
}
