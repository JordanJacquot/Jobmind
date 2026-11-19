//! L'export des candidatures en PDF (étape 12) relu par un lecteur indépendant : jobmind-lecture (pdf-extract), celui
//! qui lit les CV. Il doit retrouver les entreprises, les postes, les statuts et les accents de candidatures jamais
//! vues par les tests de jobmind-core.

use jobmind_core::export;
use jobmind_core::{Candidature, Jour, Moyen, Statut};

fn j(a: i32, m: u32, d: u32) -> Jour {
    Jour::nouveau(a, m, d).unwrap()
}

fn cand(id: i64, entreprise: &str, poste: &str, ville: &str, statut: Statut, envoyee: Jour) -> Candidature {
    Candidature {
        id,
        entreprise: entreprise.into(),
        poste: poste.into(),
        ville: ville.into(),
        statut,
        statut_le: envoyee,
        envoyee_le: envoyee,
        moyen: Moyen::Mail,
        source: "Hellowork".into(),
        lien: String::new(),
        envoye: "Texte envoyé, jamais exporté".into(),
        cv: "CV secret.pdf".into(),
        notes: String::new(),
        relance_le: None,
        entretien: None,
    }
}

/// Le texte sans aucun blanc (pdf-extract rend ses propres espaces et retours à la ligne).
fn serre(t: &str) -> String {
    t.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn le_pdf_relu_par_pdf_extract() {
    let liste = vec![
        cand(11, "Fromagerie Vosgienne", "Conducteur de ligne", "Gérardmer", Statut::Positive, j(2026, 11, 3)),
        cand(12, "Ateliers de Haguenau", "Soudeur TIG", "Haguenau", Statut::Abandonnee, j(2026, 10, 21)),
        cand(13, "Pharmacie du Rempart", "Préparatrice en pharmacie", "Colmar", Statut::Relancee, j(2026, 11, 12)),
        cand(14, "Cœur d'Épinal Services", "Aide à domicile 1 800 €", "Épinal", Statut::Negative, j(2026, 11, 14)),
        cand(15, "Cuisines Éclat d'Alsace", "Commis de cuisine", "Mulhouse", Statut::SansReponse, j(2026, 9, 30)),
    ];
    let octets = export::pdf(&liste, j(2026, 11, 19));
    let lu = jobmind_lecture::lire(&octets).expect("le PDF se relit");
    assert_eq!(lu.format, jobmind_lecture::Format::Pdf);
    let texte = serre(&lu.texte);
    for attendu in [
        "Mes candidatures",
        "Exportées le jeudi 19 novembre 2026 : 5 candidatures",
        "Fromagerie Vosgienne",
        "Conducteur de ligne",
        "Gérardmer",
        "Réponse positive",
        "Ateliers de Haguenau",
        "Abandonnée",
        "Préparatrice en pharmacie",
        "Relancée",
        "Cœur d'Épinal Services",
        "Aide à domicile 1 800 €",
        "Réponse négative",
        "Cuisines Éclat d'Alsace",
        "Sans réponse",
        "Envoyée le",
        "14/11/2026",
        "Page 1 sur 1",
    ] {
        assert!(texte.contains(&serre(attendu)), "{attendu} manque dans {:?}", lu.texte);
    }
    assert!(!texte.contains("jamais") && !texte.contains("secret"));
    // Le tri : la plus récente (Épinal, le 14/11) avant la plus ancienne (Mulhouse, le 30/09)
    let a = texte.find(&serre("Cœur d'Épinal")).unwrap();
    let b = texte.find(&serre("Cuisines Éclat")).unwrap();
    assert!(a < b);
}

#[test]
fn plusieurs_pages_et_liste_vide_relues() {
    let liste: Vec<Candidature> = (0..100)
        .map(|i| {
            cand(
                i,
                &format!("Menuiserie Bôle {i}"),
                "Menuisier poseur",
                "Lunéville",
                Statut::Envoyee,
                j(2026, 11, 19).plus(-i),
            )
        })
        .collect();
    let lu = jobmind_lecture::lire(&export::pdf(&liste, j(2026, 11, 19))).unwrap();
    let texte = serre(&lu.texte);
    for attendu in ["Page 1 sur 3", "Page 2 sur 3", "Page 3 sur 3", "Menuiserie Bôle 0", "Menuiserie Bôle 99"] {
        assert!(texte.contains(&serre(attendu)), "{attendu}");
    }
    let vide = jobmind_lecture::lire(&export::pdf(&[], j(2026, 11, 19))).unwrap();
    assert!(serre(&vide.texte).contains(&serre("Aucune candidature pour l'instant.")), "{:?}", vide.texte);
}
