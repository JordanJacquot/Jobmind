//! La cinquième série : les défauts trouvés par l'audit (un « très proche » donné à tort, des mots communs qui
//! trompent, une durée prise pour un âge, un diplôme « (niveau) » compté comme obtenu), chacun avec sa
//! contre-épreuve ; puis une mesure sur quatre CV et vingt offres inventés à part, leur niveau attendu noté avant de
//! lancer ([`jamais_vus`]). Tout est inventé.

use super::*;

fn niveau(p: &Parcours, titre: &str, texte: &str) -> Proximite {
    rapprocher(p, titre, texte).niveau
}

/// A. Un nom de métier placé après « formateur », « chef d'équipe », « responsable » n'est pas le poste.
#[test]
fn le_metier_commande_par_un_autre_nom() {
    let cariste = fiche(&[("Cariste", D::Mois(36))], &[], &[], &[]);
    for titre in [
        "Formateur cariste H/F",
        "Formatrice caristes (H/F)",
        "Chef d'équipe caristes",
        "Responsable des caristes",
        "Moniteur cariste",
    ] {
        let r = rapprocher(&cariste, titre, "Poste en journée.");
        assert_eq!(r.niveau, Proximite::UnPeu, "{titre} : {r:?}");
        assert_eq!(r.raison, "Ce poste ressemble à un métier que tu as fait (cariste).", "{titre}");
    }
    // Contre-épreuve : le poste de cariste reste très proche, seul ou avec un autre nom de métier
    for titre in [
        "Cariste H/F",
        "Cariste",
        "Caristes (H/F)",
        "Magasinier cariste",
        "Cariste magasinier H/F",
        "Agent logistique cariste",
        "Conducteur de chariot élévateur",
    ] {
        let r = rapprocher(&cariste, titre, "Poste en journée.");
        assert_eq!(r.niveau, Proximite::TresProche, "{titre} : {r:?}");
        assert_eq!(r.raison, "Tu as été cariste 3 ans.", "{titre}");
    }
    // Une famille de plusieurs mots, pareil
    let prepa = fiche(&[("Préparateur de commandes", D::Mois(36))], &[], &[], &[]);
    assert_eq!(niveau(&prepa, "Formateur préparateurs de commandes", "Poste en journée."), Proximite::UnPeu);
    assert_eq!(niveau(&prepa, "Préparateur de commandes H/F", "Poste en journée."), Proximite::TresProche);
    let vendeur = fiche(&[("Vendeur", D::Mois(36))], &[], &[], &[]);
    assert_eq!(niveau(&vendeur, "Responsable de magasin, manager de vendeurs", "Poste en journée."), Proximite::UnPeu);
    // Celui qui a été chef d'équipe ou formateur retrouve son poste
    let chef = fiche(&[("Chef d'équipe caristes", D::Mois(36))], &[], &[], &[]);
    assert_eq!(niveau(&chef, "Chef d'équipe caristes H/F", "Poste en journée."), Proximite::TresProche);
    assert_eq!(niveau(&chef, "Cariste H/F", "Poste en journée."), Proximite::UnPeu);
    let formateur = fiche(&[("Formateur cariste", D::Mois(36))], &[], &[], &[]);
    assert_eq!(niveau(&formateur, "Formateur cariste H/F", "Poste en journée."), Proximite::TresProche);
}

/// A. Le même nom de métier dans une spécialité que la fiche ne dit pas : proche, pas très proche.
#[test]
fn le_meme_metier_dans_une_autre_specialite() {
    let vendeuse = fiche(&[("Vendeuse en boulangerie", D::Mois(36))], &[], &[], &[]);
    let r = rapprocher(&vendeuse, "Vendeur automobile B2B", "Vente de véhicules aux entreprises.");
    assert_eq!(r.niveau, Proximite::Proche, "{r:?}");
    assert_eq!(r.raison, "Tu as été vendeuse en boulangerie 3 ans.");
    let secretaire = fiche(&[("Secrétaire", D::Mois(36))], &[], &[], &[]);
    for titre in ["Secrétaire juridique", "Secrétaire médicale H/F", "Secrétaire de direction"] {
        assert_eq!(niveau(&secretaire, titre, "Poste en journée."), Proximite::Proche, "{titre}");
    }
    // (même avec deux compétences demandées : jamais très proche)
    let avec = fiche(&[("Secrétaire", D::Mois(36))], &["Word", "Excel"], &[], &[]);
    assert_eq!(niveau(&avec, "Secrétaire juridique", "Maîtrise de Word et Excel."), Proximite::Proche);
    // Contre-épreuve : sans spécialité, ou avec celle de la fiche, très proche
    for titre in ["Vendeur en boulangerie H/F", "Vendeuse H/F", "Vendeur en magasin", "Vendeuse polyvalente"] {
        assert_eq!(niveau(&vendeuse, titre, "Poste en journée."), Proximite::TresProche, "{titre}");
    }
    for titre in ["Secrétaire H/F", "Secrétaire polyvalente", "Secrétaire comptable"] {
        assert_eq!(niveau(&secretaire, titre, "Poste en journée."), Proximite::TresProche, "{titre}");
    }
    let juridique = fiche(&[("Secrétaire juridique", D::Mois(36))], &[], &[], &[]);
    assert_eq!(niveau(&juridique, "Secrétaire juridique H/F", "Poste en journée."), Proximite::TresProche);
    assert_eq!(niveau(&juridique, "Secrétaire H/F", "Poste en journée."), Proximite::TresProche);
    assert_eq!(niveau(&juridique, "Secrétaire médicale", "Poste en journée."), Proximite::Proche);
}

/// B. Un mot qui a plusieurs sens ne range plus deux postes dans le même domaine.
#[test]
fn les_mots_communs_ne_font_pas_un_domaine() {
    let m = |nom: &str| fiche(&[(nom, D::Mois(36))], &[], &[], &[]);
    for (metier, titre) in [
        ("Auxiliaire de vie", "Conseiller en assurance vie"),
        ("Auxiliaire de vie", "Livreur à domicile"),
        ("Technicien de surface", "Employé de grande surface"),
        ("Technicien de surface", "Responsable de surface de vente"),
        ("Préparateur de commandes", "Opérateur sur commande numérique"),
        ("Préparateur de commandes", "Serveur prise de commande"),
        ("Conducteur de ligne", "Pilote de ligne"),
        ("Conducteur de ligne", "Vendeur en ligne"),
        ("Agent d'entretien", "Chargé d'entretiens de recrutement"),
        ("Agent d'entretien", "Mécanicien entretien avion"),
        ("Assistante de direction", "Chauffeur de direction"),
        ("Assistante de direction", "Directeur de la direction technique"),
        ("Hôtesse d'accueil", "Famille d'accueil"),
    ] {
        let r = rapprocher(&m(metier), titre, "Poste en journée.");
        assert_eq!((r.niveau, r.raison.as_str()), (Proximite::Loin, ""), "{metier} / {titre}");
    }
    // Contre-épreuve : les vrais voisins et les vraies familles restent
    for (metier, titre, attendu) in [
        ("Auxiliaire de vie", "Aide à domicile H/F", Proximite::TresProche),
        ("Auxiliaire de vie", "Assistante de vie aux familles", Proximite::TresProche),
        ("Auxiliaire de vie", "Aide-soignant en EHPAD", Proximite::UnPeu),
        ("Auxiliaire de vie", "Aide-ménagère", Proximite::TresProche),
        ("Technicien de surface", "Agent de propreté", Proximite::TresProche),
        ("Technicien de surface", "Laveur de vitres, nettoyage", Proximite::UnPeu),
        ("Préparateur de commandes", "Magasinier H/F", Proximite::UnPeu),
        ("Préparateur de commandes", "Agent logistique", Proximite::TresProche),
        ("Conducteur de ligne", "Opérateur de production", Proximite::UnPeu),
        ("Conducteur de ligne", "Conducteur de ligne de conditionnement", Proximite::TresProche),
        ("Agent d'entretien", "Agent de nettoyage H/F", Proximite::TresProche),
        ("Agent d'entretien", "Femme de ménage", Proximite::TresProche),
        ("Assistante de direction", "Secrétaire H/F", Proximite::UnPeu),
        ("Assistante de direction", "Assistant de direction H/F", Proximite::TresProche),
        ("Hôtesse d'accueil", "Agent d'accueil", Proximite::UnPeu),
        ("Hôtesse d'accueil", "Standardiste", Proximite::UnPeu),
    ] {
        assert_eq!(niveau(&m(metier), titre, "Poste en journée."), attendu, "{metier} / {titre}");
    }
}

/// B. Une compétence d'un seul mot : le mot entier, pas un mot qui commence pareil.
#[test]
fn une_competence_d_un_mot_est_un_mot_entier() {
    let demandee = |c: &str, texte: &str| competence_demandee(c, &phrases_offre("Agent de tri", texte));
    for (c, texte) in [
        ("Informatique", "Pour plus d'informations, appelez-nous."),
        ("Informatique", "Vous informez les clients."),
        ("Accueil", "Nous vous accueillons dans une équipe soudée."),
        ("Accueil", "Un cadre accueillant."),
        ("Conduite", "Un conducteur confirmé, bon conducteur d'engins."),
        ("Gestion", "Notre gestionnaire vous rappelle."),
        ("Commerce", "Un poste de commercial."),
    ] {
        assert!(!demandee(c, texte), "{c} / {texte}");
    }
    // Contre-épreuve : le mot lui-même (au pluriel, au féminin), ou son verbe dit à la personne
    for (c, texte) in [
        ("Informatique", "Bonnes bases en informatique."),
        ("Accueil", "Accueil des visiteurs."),
        ("Accueil", "Vous accueillez les clients."),
        ("Accueil", "Votre mission : accueillir le public."),
        ("Encaissement", "Vous encaissez les achats."),
        ("Encaissement", "Encaissements en fin de journée."),
        ("Nettoyage", "Vous nettoyez votre poste."),
        ("Inventaire", "Inventaires tournants."),
        ("Excel", "Maîtrise d'Excel."),
        ("Vente", "Ventes en magasin."),
    ] {
        assert!(demandee(c, texte), "{c} / {texte}");
    }
    // Une compétence de plusieurs mots se retrouve comme avant
    assert!(demandee("Mise en rayon", "Vous assurez la mise en rayon des produits."));
    assert!(demandee("Accueil et conseil des clientes", "Vous accueillez et conseillez les clients."));
    // Dans la comparaison : l'offre ouverte aux débutants qui ne demande rien de ce que la personne sait reste loin
    let p = fiche(&[], &["Informatique", "Accueil"], &[], &[]);
    let r = rapprocher(
        &p,
        "Agent de tri",
        "Nous vous accueillons dans nos locaux. Pour plus d'informations, appelez-nous. Débutant accepté.",
    );
    assert_eq!((r.niveau, r.raison.as_str()), (Proximite::Loin, ""), "{r:?}");
    let r = rapprocher(&p, "Agent de tri", "Bonnes bases en informatique. Accueil des chauffeurs. Débutant accepté.");
    assert_eq!(r.niveau, Proximite::UnPeu, "{r:?}");
    assert_eq!(
        r.raison,
        "Tu n'as pas fait ce métier, mais l'offre demande des choses que tu sais faire : Informatique, accueil."
    );
}

/// C. Une durée de 16 ans ou plus, juste après un nom de poste, n'est pas un âge.
#[test]
fn une_longue_duree_apres_un_poste() {
    let lus = |cv: &str| lire(cv).metiers.iter().map(|m| (m.nom.clone(), duree_de(m))).collect::<Vec<_>>();
    let un = |nom: &str, mois: u16| vec![(nom.to_string(), D::Mois(mois))];
    assert_eq!(lus("Expérience\nChauffeur routier, 20 ans"), un("Chauffeur routier", 240));
    assert_eq!(lus("Chauffeur routier, 20 ans\nPermis C"), un("Chauffeur routier", 240));
    assert_eq!(lus("Expérience\nCariste 17 ans chez Logi-Faux"), un("Cariste", 204));
    assert_eq!(lus("Expérience\nJ'ai été aide à domicile 25 ans"), un("Aide à domicile", 300));
    assert_eq!(lus("Expérience\nFemme de ménage (30 ans)"), un("Femme de ménage", 360));
    // (une durée courte se lisait déjà)
    assert_eq!(lus("Expérience\nCariste, 3 ans"), un("Cariste", 36));
    // Tapé par la personne dans la liste des métiers : toujours une durée, même pour un métier inconnu
    for (saisi, nom, mois) in [
        ("Chauffeur routier, 20 ans", "Chauffeur routier", Some(240)),
        ("Thanatopracteur, 20 ans", "Thanatopracteur", Some(240)),
        ("Cariste, 3 ans", "Cariste", Some(36)),
        ("Boulanger", "Boulanger", None),
    ] {
        let m = metier_saisi(saisi);
        assert_eq!((m.nom.as_str(), m.mois, m.depuis), (nom, mois, None), "{saisi}");
    }
    // Contre-épreuve : un âge reste un âge (le nom de la personne, une ligne d'état civil, « de 25 ans »)
    let lit = |l: &str| {
        let mut j = jetons(l);
        dates_de(l, &mut j).vue
    };
    for ligne in [
        "Raoul Camionfictif, 52 ans",
        "52 ans",
        "Cariste, 28 ans, permis B, véhiculé",
        "Vendeuse, 31 ans, mariée, deux enfants",
        "Jeune vendeuse de 25 ans",
        "Vendeuse dynamique de 25 ans",
        "Cariste âgé de 40 ans",
        "Homme, 45 ans",
        "Responsable, 45 ans",
    ] {
        assert!(!lit(ligne), "{ligne}");
    }
    for cv in [
        "Jean Exemple, 45 ans\nExpérience\n2019 - 2022 : Cariste, Logi-Faux",
        "Jean Exemple\nCariste, 45 ans, permis B\nExpérience\n2019 - 2022 : Cariste, Logi-Faux",
    ] {
        assert_eq!(lus(cv), un("Cariste", 36), "{cv}");
    }
}

/// C. « (niveau) », « niveau » après le diplôme, « (non obtenu) » : le diplôme n'a pas été obtenu.
#[test]
fn un_diplome_de_niveau_n_est_pas_obtenu() {
    let p = lire("Formation\nBac pro commerce (niveau)\nCAP Vente");
    assert_eq!(p.diplomes, ["Bac pro commerce (niveau)", "CAP Vente"]);
    assert_eq!(p.diplome_le_plus_haut(), Some(Diplome::CapBep));
    let p = lire("Formation\n2015 : Bac pro commerce (niveau), Lycée Inventé");
    assert_eq!(p.diplomes, ["Bac pro commerce (niveau)"]);
    assert_eq!(p.diplome_le_plus_haut(), None);
    assert_eq!(lire("Formation\nBac pro commerce niveau").diplome_le_plus_haut(), None);
    assert_eq!(lire("Formation\nNiveau bac pro commerce").diplome_le_plus_haut(), None);
    assert_eq!(lire("Formation\nBac pro commerce (non obtenu)").diplomes, Vec::<String>::new());
    // Une ligne tapée par la personne
    for ligne in [
        "Bac pro commerce (niveau)",
        "Bac pro commerce niveau",
        "Bac pro commerce, niveau",
        "Bac pro commerce (non obtenu)",
        "BTS MCO pas obtenu",
        "BTS MCO (en cours)",
        "Niveau bac",
    ] {
        assert_eq!(niveau_diplome(ligne), None, "{ligne}");
    }
    // Contre-épreuve : un diplôme obtenu garde son niveau, et « niveau 4 » dit une valeur
    for (ligne, attendu) in [
        ("Bac pro commerce", Diplome::Bac),
        ("Bac pro commerce (obtenu en 2015)", Diplome::Bac),
        ("CAP Cuisine", Diplome::CapBep),
        ("BTS Comptabilité et gestion", Diplome::Bac2),
    ] {
        assert_eq!(niveau_diplome(ligne), Some(attendu), "{ligne}");
    }
    let p = lire("Formation\n2015 : Bac pro commerce, Lycée Inventé\nTitre professionnel de niveau 4 (2019)");
    assert_eq!(p.diplomes[0], "Bac pro commerce");
    assert!(!p.diplomes.iter().any(|d| d.ends_with("(niveau)")), "{:?}", p.diplomes);
    assert_eq!(p.diplome_le_plus_haut(), Some(Diplome::Bac));
    // L'offre qui demande un Bac+2 : hors de portée avec un Bac « (niveau) » seul
    let q = fiche(&[("Vendeuse", D::Mois(36))], &[], &["BTS MCO (niveau)"], &[]);
    let r = rapprocher(&q, "Vendeuse H/F", "BTS exigé.");
    assert!(r.raison.ends_with("mais l'offre demande un Bac+2."), "{r:?}");
    let q = fiche(&[("Vendeuse", D::Mois(36))], &[], &["BTS MCO"], &[]);
    assert_eq!(niveau(&q, "Vendeuse H/F", "BTS exigé."), Proximite::TresProche);
}

// ---------------------------------------------------------------------------------------------------------------
// La mesure sur des cas jamais vus
// ---------------------------------------------------------------------------------------------------------------

const U1: &str = "Mireille Fauxnom
Secrétaire médicale, 22 ans
Assistante administrative, 4 ans
BTS Assistant de gestion (niveau)
Compétences : accueil, frappe de courriers, Word";

const U2: &str = "Expérience
Maçon, 18 ans
Manœuvre en intérim, 2 ans
Formation
CAP Maçon
Permis B";

const U3: &str = "Sandrine Inventée, 34 ans
Expérience
2015 - 2024 : Aide à domicile, Asso Bidon
2012 - 2015 : Caissière, Supermarché Faux
Compétences : entretien du logement, courses, informatique";

const U4: &str = "Expérience
Développeur web depuis 2019, Agence Fictive
Livreur, 2 ans
Compétences : PHP, SQL, commande de matériel";

/// Quatre CV courts et vingt offres inventés après les corrections, sans s'en servir pour corriger ; le niveau
/// attendu a été noté avant de lancer. Mesure faite telle quelle au premier lancement, sans rien corriger ensuite :
/// 19 offres sur 20 et 6 lectures sur 6. Le raté, laissé tel quel : « Agent d'entretien » pour une aide à domicile
/// (attendu un peu proche, lu loin : le soin et le nettoyage sont deux domaines, et « entretien du logement » n'est
/// pas retrouvé dans « entretien des locaux »).
#[test]
fn jamais_vus() {
    use Proximite::{Loin, Proche, TresProche, UnPeu};
    let cvs = [lire(U1), lire(U2), lire(U3), lire(U4)];
    let offres: [(usize, &str, &str, Proximite); 20] = [
        (0, "Secrétaire médicale H/F", "Accueil des patients, prise de rendez-vous.", TresProche),
        (0, "Secrétaire juridique H/F", "Frappe d'actes.", Proche),
        (0, "Formateur en secrétariat médical", "Vous formez des adultes.", Loin),
        (0, "Responsable des secrétaires médicales", "Encadrement de 8 personnes.", UnPeu),
        (
            0,
            "Conseiller en assurance vie H/F",
            "Nous vous accueillons dans une équipe jeune. Pour plus d'informations, écrivez-nous.",
            Loin,
        ),
        (1, "Maçon H/F", "Travaux de gros œuvre.", TresProche),
        (1, "Chef d'équipe maçons", "Vous dirigez 5 maçons.", UnPeu),
        (1, "Maçon coffreur", "Coffrage et ferraillage.", TresProche),
        (1, "Conducteur de ligne de production", "Travail en 3x8, débutant accepté.", Loin),
        (1, "Formateur maçon", "Formation d'adultes, titre de formateur exigé.", UnPeu),
        (1, "Technicien de surface", "Nettoyage de bureaux.", Loin),
        (2, "Auxiliaire de vie H/F", "Aide aux personnes âgées.", TresProche),
        (2, "Conseiller en assurance vie", "Vente de contrats.", Loin),
        (2, "Livreur à domicile", "Livraison de repas en scooter.", Loin),
        (2, "Technicien de maintenance informatique", "Pour plus d'informations sur le poste, appelez-nous.", Loin),
        (2, "Hôte de caisse H/F", "Encaissement des clients.", TresProche),
        (2, "Responsable de caisse", "Vous encadrez les hôtes de caisse.", UnPeu),
        (2, "Agent d'entretien", "Entretien des locaux.", UnPeu),
        (3, "Développeur PHP H/F", "PHP et SQL exigés.", TresProche),
        (3, "Chauffeur-livreur VL", "Permis B exigé.", TresProche),
    ];
    let mut rapport = String::new();
    let mut ok = 0;
    for (cv, titre, texte, attendu) in offres {
        let r = rapprocher(&cvs[cv], titre, texte);
        if r.niveau == attendu {
            ok += 1;
        } else {
            rapport.push_str(&format!(
                "U{} / {titre} : attendu {attendu:?}, lu {:?} ({})\n",
                cv + 1,
                r.niveau,
                r.raison
            ));
        }
    }
    // La lecture des quatre CV (les durées longues, l'âge, le diplôme « (niveau) »)
    let lecture: [(&str, bool); 6] = [
        ("U1 : secrétaire médicale 22 ans", cvs[0].metiers.first().is_some_and(|m| m.mois == Some(264))),
        ("U1 : le BTS (niveau) n'est pas obtenu", cvs[0].diplome_le_plus_haut().is_none()),
        ("U2 : maçon 18 ans", cvs[1].metiers.first().is_some_and(|m| m.mois == Some(216))),
        ("U2 : CAP obtenu", cvs[1].diplome_le_plus_haut() == Some(Diplome::CapBep)),
        ("U3 : 34 ans est un âge", cvs[2].metiers.iter().all(|m| m.mois == Some(108) || m.mois == Some(36))),
        ("U4 : livreur 2 ans", cvs[3].metiers.iter().any(|m| m.nom == "Livreur" && m.mois == Some(24))),
    ];
    let lus = lecture.iter().filter(|x| x.1).count();
    for (nom, _) in lecture.iter().filter(|x| !x.1) {
        rapport.push_str(&format!("lecture : {nom}\n"));
    }
    eprintln!("jamais vus : {ok}/20 offres, {lus}/6 lectures\n{rapport}");
    for (i, p) in cvs.iter().enumerate() {
        eprintln!("U{} : {:?} / {:?} / {:?} / {:?}", i + 1, p.metiers, p.competences, p.diplomes, p.permis);
    }
    assert!(ok >= 19 && lus == 6, "{ok}/20 offres, {lus}/6 lectures\n{rapport}");
}
