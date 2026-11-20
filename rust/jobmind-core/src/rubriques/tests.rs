use super::*;

/// Une offre France Travail typique (sa forme, pas une vraie offre).
const FRANCE_TRAVAIL: &str = "Préparateur de commandes H/F
Au sein de notre entrepôt de 3000 m2, vous préparez les commandes de nos clients à l'aide d'un chariot et d'un \
lecteur de codes. Vous vérifiez la conformité des produits et participez au rangement.

Type de contrat
Contrat à durée indéterminée
Durée du travail
35H/semaine
Travail en journée
Salaire
Salaire brut : Mensuel de 1900 Euros sur 12 mois
Profil souhaité
Expérience
Débutant accepté
Savoirs et savoir-faire
Préparer les commandes
Contrôler la conformité des produits
Ranger les marchandises
Savoir-être professionnels
Rigueur
Travail en équipe
Permis
B - Véhicule léger Exigé
Informations complémentaires
Qualification : Employé non qualifié
Secteur d'activité : Entreposage et stockage non frigorifique
Entreprise
Entreprise familiale implantée en Moselle depuis 1985, nous livrons les commerces de proximité de toute la région \
Grand Est.
50 à 99 salariés";

/// Une offre d'agence d'intérim.
const INTERIM: &str = "Notre agence recrute pour son client, une PME de la logistique, un cariste H/F.

Vos missions :
- Charger et décharger les camions
- Ranger les palettes en hauteur
- Tenir les stocks à jour

Votre profil :
Vous êtes rigoureux, ponctuel et vous aimez le travail en équipe. Une première expérience est souhaitée.
CACES 1, 3 et 5 (exigé)

Rémunération et avantages :
• 11,88 € brut de l'heure
• Tickets restaurant
• Prime de fin de mission";

const UN_PARAGRAPHE: &str = "Nous recherchons un vendeur pour notre magasin de Metz. Vous accueillez les clients, \
les conseillez et tenez la caisse. Poste en CDI, 35 heures, débutant accepté.";

/// Une phrase avec « salaire » au milieu : pas un titre.
const SALAIRE_AU_MILIEU: &str = "Vendeur en boulangerie
Le salaire sera discuté lors de l'entretien.
Horaires du matin, week-end compris.";

const EXIGENCES: &str = "Profil souhaité
Expérience
2 ans - Exigé
Formation
CAP Boulangerie (souhaité)
Permis
B - Véhicule léger Exigé
C - Poids lourd Souhaité
Langues
Anglais : Souhaitée";

fn onglet<'a>(o: &'a [Onglet], id: &str) -> &'a Onglet {
    o.iter().find(|x| x.id == id).unwrap_or_else(|| panic!("pas d'onglet {id} : {o:#?}"))
}

fn rang<'a>(o: &'a [Onglet], id: &str, nom: &str) -> &'a Rang {
    onglet(o, id).rangs.iter().find(|r| r.nom == nom).unwrap_or_else(|| panic!("pas de rang {nom} : {o:#?}"))
}

fn ids(o: &[Onglet]) -> Vec<&str> {
    o.iter().map(|x| x.id).collect()
}

#[test]
fn une_offre_france_travail() {
    let o = decouper("Préparateur de commandes (H/F)", FRANCE_TRAVAIL);
    assert_eq!(ids(&o), ["poste", "demande", "conditions", "entreprise"]);
    assert_eq!(o[1].nom, "Ce qu'ils demandent");
    // Le titre répété n'est pas repris ; la description est la valeur, sans nom
    let p = &onglet(&o, "poste").rangs;
    assert_eq!(p.len(), 1, "{p:#?}");
    assert_eq!(p[0].nom, "");
    assert!(p[0].valeur.starts_with("Au sein de notre entrepôt"), "{}", p[0].valeur);
    // Les conditions
    assert_eq!(rang(&o, "conditions", "Type de contrat").valeur, "Contrat à durée indéterminée");
    let d = rang(&o, "conditions", "Durée du travail");
    assert_eq!(d.puces, ["35H/semaine", "Travail en journée"]);
    assert_eq!(d.valeur, "");
    assert_eq!(rang(&o, "conditions", "Salaire brut").valeur, "Mensuel de 1900 Euros sur 12 mois");
    // Ce qu'ils demandent (« Profil souhaité », vide, n'est pas rendu)
    assert!(onglet(&o, "demande").rangs.iter().all(|r| r.nom != "Profil souhaité"));
    assert_eq!(rang(&o, "demande", "Expérience").valeur, "Débutant accepté");
    assert_eq!(rang(&o, "demande", "Expérience").exigence, None);
    assert_eq!(rang(&o, "demande", "Savoirs et savoir-faire").puces.len(), 3);
    assert_eq!(rang(&o, "demande", "Savoir-être professionnels").puces, ["Rigueur", "Travail en équipe"]);
    let permis = rang(&o, "demande", "Permis");
    assert_eq!(permis.valeur, "B - Véhicule léger");
    assert_eq!(permis.exigence, Some(Exigence::Exige));
    assert_eq!(rang(&o, "demande", "Qualification").valeur, "Employé non qualifié");
    // L'entreprise
    assert_eq!(rang(&o, "entreprise", "Secteur d'activité").valeur, "Entreposage et stockage non frigorifique");
    let e = rang(&o, "entreprise", "Entreprise");
    assert!(e.valeur.starts_with("Entreprise familiale"), "{}", e.valeur);
    assert!(e.valeur.ends_with("\n50 à 99 salariés"), "{}", e.valeur);
    // Les horaires en court
    assert_eq!(horaires(&o).as_deref(), Some("35H/semaine"));
}

#[test]
fn une_offre_d_interim() {
    let o = decouper("Cariste H/F", INTERIM);
    assert_eq!(ids(&o), ["poste", "demande", "conditions"]);
    let p = &onglet(&o, "poste").rangs;
    assert!(p[0].valeur.starts_with("Notre agence recrute"));
    let m = rang(&o, "poste", "Vos missions");
    assert_eq!(
        m.puces,
        ["Charger et décharger les camions", "Ranger les palettes en hauteur", "Tenir les stocks à jour"]
    );
    assert_eq!(m.valeur, "");
    // « est souhaitée » dans une phrase reste dans la phrase ; « (exigé) » est retiré
    let profil: Vec<&Rang> = onglet(&o, "demande").rangs.iter().filter(|r| r.nom == "Votre profil").collect();
    assert_eq!(profil.len(), 2, "{profil:#?}");
    assert!(profil[0].valeur.ends_with("Une première expérience est souhaitée."));
    assert_eq!(profil[0].exigence, None);
    assert_eq!(profil[1].valeur, "CACES 1, 3 et 5");
    assert_eq!(profil[1].exigence, Some(Exigence::Exige));
    let r = rang(&o, "conditions", "Rémunération et avantages");
    assert_eq!(r.puces, ["11,88 € brut de l'heure", "Tickets restaurant", "Prime de fin de mission"]);
    assert_eq!(horaires(&o), None);
}

#[test]
fn un_seul_paragraphe() {
    let o = decouper("Vendeur H/F", UN_PARAGRAPHE);
    assert_eq!(ids(&o), ["poste"]);
    assert_eq!(o[0].nom, "Le poste");
    assert_eq!(o[0].rangs.len(), 1);
    assert_eq!(o[0].rangs[0].valeur, UN_PARAGRAPHE);
    assert!(o[0].rangs[0].puces.is_empty());
}

#[test]
fn salaire_au_milieu_d_une_phrase() {
    let o = decouper("Vendeur en boulangerie", SALAIRE_AU_MILIEU);
    assert_eq!(ids(&o), ["poste"]);
    assert_eq!(
        o[0].rangs[0].valeur,
        "Le salaire sera discuté lors de l'entretien.\nHoraires du matin, week-end compris."
    );
}

#[test]
fn exige_et_souhaite() {
    let o = decouper("", EXIGENCES);
    assert_eq!(ids(&o), ["demande"]);
    let e = rang(&o, "demande", "Expérience");
    assert_eq!((e.valeur.as_str(), e.exigence), ("2 ans", Some(Exigence::Exige)));
    let f = rang(&o, "demande", "Formation");
    assert_eq!((f.valeur.as_str(), f.exigence), ("CAP Boulangerie", Some(Exigence::Souhaite)));
    // Deux permis, deux exigences : deux rangs, rien de mélangé
    let p: Vec<&Rang> = o[0].rangs.iter().filter(|r| r.nom == "Permis").collect();
    assert_eq!(p.len(), 2);
    assert_eq!((p[0].valeur.as_str(), p[0].exigence), ("B - Véhicule léger", Some(Exigence::Exige)));
    assert_eq!((p[1].valeur.as_str(), p[1].exigence), ("C - Poids lourd", Some(Exigence::Souhaite)));
    let l = rang(&o, "demande", "Langues");
    assert_eq!((l.valeur.as_str(), l.exigence), ("Anglais", Some(Exigence::Souhaite)));
    assert_eq!(Exigence::Exige.code(), "exige");
    assert_eq!(Exigence::Souhaite.code(), "souhaite");
}

#[test]
fn exigence_en_fin_de_ligne() {
    let cas: &[(&str, &str, Option<Exigence>)] = &[
        ("Permis B - Exigé", "Permis B", Some(Exigence::Exige)),
        ("B - Véhicule léger Exigé", "B - Véhicule léger", Some(Exigence::Exige)),
        ("2 ans : exigée", "2 ans", Some(Exigence::Exige)),
        ("Bac pro (Souhaité)", "Bac pro", Some(Exigence::Souhaite)),
        ("CACES R489 SOUHAITEE", "CACES R489", Some(Exigence::Souhaite)),
        ("Exigé", "", Some(Exigence::Exige)),
        // Pas une exigence
        ("CAP (E)", "CAP (E)", None),
        ("Une première expérience est souhaitée.", "Une première expérience est souhaitée.", None),
        ("Expérience fortement souhaitée", "Expérience fortement souhaitée", None),
        (
            "Vous avez déjà travaillé dans un entrepôt et le permis cariste est exigé",
            "Vous avez déjà travaillé dans un entrepôt et le permis cariste est exigé",
            None,
        ),
        ("Débutant accepté", "Débutant accepté", None),
        ("Rigueur", "Rigueur", None),
    ];
    for (ligne, texte, ex) in cas {
        assert_eq!(exigence_finale(ligne), (texte.to_string(), *ex), "{ligne}");
    }
}

#[test]
fn exige_seul_sur_sa_ligne() {
    let o = decouper("", "Expérience\n2 ans\nExigé\nPermis\nB\n(Souhaité)");
    assert_eq!(rang(&o, "demande", "Expérience").valeur, "2 ans");
    assert_eq!(rang(&o, "demande", "Expérience").exigence, Some(Exigence::Exige));
    assert_eq!(rang(&o, "demande", "Permis").exigence, Some(Exigence::Souhaite));
    // « Exigé » sans rien avant : il reste un texte
    let o = decouper("", "Permis\nExigé");
    assert_eq!(rang(&o, "demande", "Permis").valeur, "Exigé");
}

#[test]
fn un_texte_vide() {
    assert!(decouper("Cariste", "").is_empty());
    assert!(decouper("Cariste", "  \n\n \t").is_empty());
    // Seulement le titre répété
    assert!(decouper("Cariste H/F", "Cariste (H/F)\n").is_empty());
}

#[test]
fn les_titres_reconnus() {
    // Majuscules, accents, « : » et ponctuation ne comptent pas
    for t in
        ["VOS MISSIONS :", "Profil souhaite", "Qui sommes-nous ?", "** Rémunération **", "A propos", "Lieu de travail:"]
    {
        assert!(titre_de(t).is_some(), "{t}");
    }
    assert_eq!(titre_de("VOS MISSIONS :"), Some((0, "VOS MISSIONS".to_string())));
    assert_eq!(titre_de("Qui sommes-nous ?").map(|x| x.0), Some(3));
    // Trop long, une puce, ou pas dans la liste
    for t in [
        "Le salaire sera discuté",
        "- Salaire",
        "Salaire selon profil et expérience",
        "Rigueur",
        "Préparer les commandes",
    ] {
        assert!(titre_de(t).is_none(), "{t}");
    }
    // Les libellés
    assert_eq!(libelle("Qualification : Employé qualifié"), Some((1, "Qualification".into(), "Employé qualifié")));
    assert_eq!(libelle("Horaires: 8:00 à 16:00"), Some((2, "Horaires".into(), "8:00 à 16:00")));
    assert_eq!(libelle("Salaire :"), None);
    assert_eq!(libelle("Note : apporter son CV"), None);
    assert_eq!(libelle("- Salaire : 12 € de l'heure"), None);
    // Chaque titre de la liste est bien court et se reconnaît
    for (o, liste) in TITRES.iter().enumerate() {
        for t in *liste {
            assert_eq!(onglet_du_titre(t), Some(o), "{t}");
        }
    }
}

#[test]
fn le_texte_avant_les_rubriques_et_les_paragraphes() {
    let t = "Boulanger H/F\nPremier paragraphe.\n\nSecond paragraphe, après une ligne vide.\n- une puce dans \
             l'introduction\nVos missions\nPétrir la pâte, surveiller la cuisson et ranger le fournil chaque soir \
             avant de partir.\nAccueillir les clients le samedi matin, encaisser les achats et tenir la vitrine.";
    let o = decouper("Boulanger", t);
    let intro = &onglet(&o, "poste").rangs[0];
    assert_eq!(intro.nom, "");
    assert_eq!(intro.valeur, "Premier paragraphe.\n\nSecond paragraphe, après une ligne vide.");
    assert_eq!(intro.puces, ["une puce dans l'introduction"]);
    // Deux longues lignes : un paragraphe
    let m = rang(&o, "poste", "Vos missions");
    assert!(m.puces.is_empty());
    assert_eq!(m.valeur.lines().count(), 2);
}

#[test]
fn le_titre_repete_n_est_repris_que_s_il_est_en_tete() {
    let o = decouper("Cariste H/F", "Cariste\nVous conduisez un chariot.\nProfil\nCariste");
    assert_eq!(onglet(&o, "poste").rangs[0].valeur, "Vous conduisez un chariot.");
    assert_eq!(rang(&o, "demande", "Profil").valeur, "Cariste");
}

#[test]
fn les_horaires_en_court() {
    let o = decouper("", "Horaires\nDu lundi au vendredi de 8 h à 16 h avec une heure de pause le midi");
    assert_eq!(horaires(&o).as_deref(), Some("Du lundi au vendredi de 8 h à 16 h avec…"));
    assert!(horaires(&o).unwrap().chars().count() <= 41);
    let o = decouper("", "Horaires : de 9 h à 17 h");
    assert_eq!(horaires(&o).as_deref(), Some("de 9 h à 17 h"));
}

/// Aucune ligne perdue : chaque ligne non vide se retrouve dans un rang (sauf le titre répété et les titres de
/// rubrique), dans tous les exemples.
#[test]
fn aucune_ligne_perdue() {
    let exemples = [
        ("Préparateur de commandes (H/F)", FRANCE_TRAVAIL),
        ("Cariste H/F", INTERIM),
        ("Vendeur", UN_PARAGRAPHE),
        ("Vendeur en boulangerie", SALAIRE_AU_MILIEU),
        ("", EXIGENCES),
        ("", "Expérience\n2 ans\nExigé\nPermis\nB\n(Souhaité)"),
        ("Cariste H/F", "Cariste\nVous conduisez un chariot.\nProfil\nCariste"),
    ];
    for (titre, texte) in exemples {
        let o = decouper(titre, texte);
        let mut tout = String::new();
        for r in o.iter().flat_map(|x| &x.rangs) {
            for t in std::iter::once(&r.nom).chain(std::iter::once(&r.valeur)).chain(&r.puces) {
                tout.push(' ');
                tout.push_str(&normaliser(t));
            }
        }
        tout.push(' ');
        for (i, l) in texte.lines().map(str::trim).enumerate() {
            if l.is_empty() || (i == 0 && meme_titre(l, titre)) || titre_de(l).is_some() {
                continue;
            }
            let (_, sans) = sans_puce(l);
            let attendu = normaliser(&exigence_finale(sans).0);
            if attendu.is_empty() {
                continue;
            }
            assert!(tout.contains(&format!(" {attendu} ")), "ligne perdue : {l}\n{o:#?}");
        }
        // Aucun tiret long ajouté, aucun rang vide
        for r in o.iter().flat_map(|x| &x.rangs) {
            assert!(!r.valeur.is_empty() || !r.puces.is_empty(), "{r:#?}");
        }
    }
}
