//! La réécriture vérifiée : ce que Claude rend est gardé, refusé ou surligné.

use super::*;

const CV: &str = "Jordan Exemple\nMetz (57), permis B\n2023 à 2025 : préparateur de commandes et cariste en intérim, \
                  entrepôt de Woippy\n2021 à 2022 : vendeur dans un magasin de sport, Metz\nCAP Opérateur logistique, \
                  2018\nCACES 1, 3 et 5";
const OFFRE: &str = "Agent de quai H/F chez Transports Moselle Fret. CACES 3 exigé. Poste en journée.";
const MES_TEXTES: &str = "Bonjour, je vous écris pour le poste. J'aime le travail bien fait et je suis ponctuel.";

const LETTRE: &str = "Madame, Monsieur,\n\nVotre offre pour le poste d'agent de quai chez Transports Moselle Fret a \
                      retenu toute mon attention. Afin de vous présenter mon parcours, voici l'essentiel.\n\nJ'ai \
                      travaillé comme préparateur de commandes et cariste en intérim. J'ai aussi encadré une équipe de \
                      six personnes.\n\nJe serais heureux de vous rencontrer lors d'un entretien.\n\nJe vous prie \
                      d'agréer, Madame, Monsieur, l'expression de mes salutations distinguées.\nJordan Exemple";

fn depart() -> Depart {
    Depart::depuis_lettre(
        LETTRE,
        &[("J'ai aussi encadré une équipe de six personnes.".into(), "Rien dans ton CV ne la soutient.".into())],
    )
}

fn s() -> Sources<'static> {
    Sources { profil: CV, offre: OFFRE, vocabulaire: MES_TEXTES }
}

fn p(t: &str, depuis: &[usize]) -> PhraseNouvelle {
    PhraseNouvelle { texte: t.into(), sur_moi: false, extraits: Vec::new(), depuis: depuis.to_vec() }
}

fn moi(t: &str, extrait: &str, depuis: &[usize]) -> PhraseNouvelle {
    PhraseNouvelle { texte: t.into(), sur_moi: true, extraits: vec![extrait.into()], depuis: depuis.to_vec() }
}

#[test]
fn le_depart_numerote() {
    let d = depart();
    assert_eq!(d.phrases().len(), 5);
    assert!(d.numerotee().starts_with("[1] Votre offre"));
    assert!(d.numerotee().contains("\n\n[3] J'ai travaillé"));
    assert_eq!(d.phrases()[3].a_verifier.as_deref(), Some("Rien dans ton CV ne la soutient."));
    assert_eq!(d.texte(), LETTRE);
    assert!(d.appel.is_some() && d.fin.as_deref().unwrap().ends_with("Jordan Exemple"));
}

#[test]
fn les_regles_seules() {
    let r = Regles::default();
    let x = avec_regles(&depart(), &r);
    assert!(x.texte.contains("Pour vous présenter mon parcours"), "{}", x.texte);
    assert_eq!(x.changements, 1);
    assert_eq!(x.a_verifier.len(), 1);
    assert_eq!(x.a_verifier[0].texte, "J'ai aussi encadré une équipe de six personnes.");
    assert!(x.texte.starts_with("Madame, Monsieur,") && x.texte.ends_with("Jordan Exemple"));
    // Le départ réglé garde ses numéros et ses phrases à vérifier
    let d = depart_regle(&depart(), &r);
    assert_eq!(d.phrases().len(), 5);
    assert!(d.phrases()[1].texte.starts_with("Pour vous"));
    assert!(d.phrases()[3].a_verifier.is_some());
}

#[test]
fn une_reecriture_propre_est_gardee() {
    let r = Regles::default();
    let rendu = vec![
        vec![p("Votre offre d'agent de quai chez Transports Moselle Fret m'intéresse.", &[1, 2])],
        vec![
            moi(
                "J'ai été préparateur de commandes et cariste en intérim.",
                "préparateur de commandes et cariste en intérim",
                &[3],
            ),
            moi("J'ai aussi encadré six personnes.", "chef d'équipe", &[4]),
        ],
        vec![p("Je serais heureux de vous rencontrer.", &[5])],
    ];
    let x = assembler(&depart(), &rendu, s(), &r).unwrap();
    assert_eq!(x.refusees, 0, "{x:?}");
    assert!(x.texte.starts_with("Madame, Monsieur,\n\nVotre offre d'agent"));
    assert!(x.texte.ends_with("salutations distinguées.\nJordan Exemple"));
    // La phrase à vérifier de départ le reste, réécrite
    assert_eq!(x.a_verifier.len(), 1);
    assert_eq!(x.a_verifier[0].texte, "J'ai aussi encadré six personnes.");
    assert_eq!(x.a_verifier[0].code, "depart");
}

#[test]
fn un_fait_ajoute_est_refuse() {
    let r = Regles::default();
    // Un nombre nouveau : la phrase de départ est remise telle quelle
    let rendu = vec![vec![
        moi(
            "J'ai été préparateur de commandes et cariste en intérim pendant quatre ans.",
            "préparateur de commandes et cariste en intérim",
            &[3],
        ),
        p("Je serais heureux de vous rencontrer.", &[5]),
    ]];
    let x = assembler(&depart(), &rendu, s(), &r).unwrap();
    assert_eq!(x.refusees, 1);
    assert!(x.texte.contains("J'ai travaillé comme préparateur de commandes et cariste en intérim."));
    assert!(!x.texte.contains("quatre"));
    assert!(x.notes[0].contains("« quatre »"), "{:?}", x.notes);
    // Un nom propre nouveau (un logiciel), même dans une phrase qui ne parle pas de Jordan
    let rendu = vec![vec![p("Votre entrepôt utilise sûrement SAP, comme chez Transports Moselle Fret.", &[1])]];
    let x = assembler(&depart(), &rendu, s(), &r).unwrap();
    assert_eq!(x.refusees, 1);
    assert!(x.notes[0].contains("« SAP »"), "{:?}", x.notes);
    assert!(x.texte.contains("a retenu toute mon attention"));
    // Une phrase sur Jordan sans passage du CV : refusée aussi
    let rendu = vec![vec![PhraseNouvelle {
        texte: "Je connais bien la conduite des chariots.".into(),
        sur_moi: true,
        extraits: Vec::new(),
        depuis: vec![3],
    }]];
    let x = assembler(&depart(), &rendu, s(), &r).unwrap();
    assert_eq!(x.refusees, 1);
    assert!(x.notes[0].contains("un fait que ton CV ne dit pas"));
    // Un passage cité qui n'est pas dans le CV : refusée
    let rendu = vec![vec![moi("J'ai conduit des chariots.", "conduite de chariots pendant dix ans", &[3])]];
    assert_eq!(assembler(&depart(), &rendu, s(), &r).unwrap().refusees, 1);
}

#[test]
fn sans_phrase_de_depart_elle_est_surlignee() {
    let r = Regles::default();
    let rendu = vec![vec![
        p("Votre offre m'intéresse.", &[1]),
        p("Je connais aussi Excel.", &[]),
        p("Phrase avec un numéro inconnu chez Durand.", &[42]),
    ]];
    let x = assembler(&depart(), &rendu, s(), &r).unwrap();
    assert!(x.texte.contains("Je connais aussi Excel."));
    let codes: Vec<&str> = x.a_verifier.iter().map(|a| a.code.as_str()).collect();
    assert_eq!(codes, ["nom_absent", "nom_absent"], "{x:?}");
}

#[test]
fn un_mot_nouveau_est_surligne_pas_refuse() {
    let r = Regles::default();
    let rendu = vec![vec![p("J'ai dirigé le quai de chargement.", &[3])]];
    let x = assembler(&depart(), &rendu, s(), &r).unwrap();
    assert_eq!(x.refusees, 0);
    assert_eq!(x.a_verifier.len(), 1);
    assert_eq!(x.a_verifier[0].code, "mot_absent");
    // Les mots des textes de Jordan, eux, sont permis
    let rendu = vec![vec![p("J'aime le travail bien fait.", &[3])]];
    assert!(assembler(&depart(), &rendu, s(), &r).unwrap().a_verifier.is_empty());
}

#[test]
fn un_mot_familier_est_refuse() {
    let r = Regles::default();
    let rendu = vec![vec![p("Du coup, votre offre m'intéresse.", &[1]), p("Salut à toute l'équipe.", &[])]];
    let x = assembler(&depart(), &rendu, s(), &r).unwrap();
    assert_eq!(x.refusees, 2);
    assert!(x.texte.contains("a retenu toute mon attention"));
    assert!(!x.texte.contains("Salut") && !x.texte.contains("Du coup"));
    assert!(x.notes.iter().any(|n| n.contains("« Du coup »")));
    assert!(x.notes.iter().any(|n| n.contains("retirée")));
}

#[test]
fn le_plancher_et_les_regles_apres_claude() {
    let r = Regles::default();
    let rendu = vec![vec![p("Votre offre \u{2014} au sein de Transports Moselle Fret \u{2014} m'intéresse !", &[1])]];
    let x = assembler(&depart(), &rendu, s(), &r).unwrap();
    assert!(x.texte.contains("Votre offre, dans Transports Moselle Fret, m'intéresse."), "{}", x.texte);
    // Rien d'utilisable : rien
    assert!(assembler(&depart(), &[vec![p("   ", &[1])]], s(), &r).is_none());
    assert!(assembler(&depart(), &[], s(), &r).is_none());
    // Une lettre sans appel ni politesse : le plancher les ajoute
    let d = Depart::depuis_lettre("J'aime ce poste.", &[]);
    let x = assembler(&d, &[vec![p("Ce poste me plaît.", &[1])]], s(), &r).unwrap();
    assert!(x.texte.starts_with(plancher::APPEL) && x.texte.ends_with(plancher::POLITESSE));
}
