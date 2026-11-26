//! Les CV et les offres d'ici sont tous inventés (personnes et entreprises de fantaisie). La première moitié des CV a
//! servi à écrire les règles ; la seconde ([`seconde`]) et ses couples offre et fiche ont été inventés à part, sans
//! voir les règles, pour mesurer.

use super::*;
use crate::lecture::lire as lire_offre;

mod cinquieme;
mod quatrieme;
mod seconde;
mod troisieme;

pub(super) fn auj() -> Jour {
    Jour::nouveau(2026, 11, 25).unwrap()
}

/// La durée d'un métier, comme le CV la donne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum D {
    Mois(u16),
    Depuis(i32, u32),
    Rien,
}

pub(super) fn duree_de(m: &Metier) -> D {
    match (m.depuis, m.mois) {
        (Some((a, x)), _) => D::Depuis(a, x),
        (None, Some(x)) => D::Mois(x),
        (None, None) => D::Rien,
    }
}

fn metiers(p: &Parcours) -> Vec<(&str, D)> {
    p.metiers.iter().map(|m| (m.nom.as_str(), duree_de(m))).collect()
}

/// Une fiche déjà faite.
pub(super) fn fiche(metiers: &[(&str, D)], competences: &[&str], diplomes: &[&str], permis: &[&str]) -> Parcours {
    Parcours {
        metiers: metiers
            .iter()
            .map(|(nom, d)| Metier {
                nom: (*nom).to_string(),
                mois: if let D::Mois(m) = d { Some(*m) } else { None },
                depuis: if let D::Depuis(a, m) = d { Some((*a, *m)) } else { None },
            })
            .collect(),
        competences: competences.iter().map(|x| (*x).to_string()).collect(),
        diplomes: diplomes.iter().map(|x| (*x).to_string()).collect(),
        permis: permis.iter().map(|x| (*x).to_string()).collect(),
    }
}

pub(super) fn rapprocher(p: &Parcours, titre: &str, texte: &str) -> Rapprochement {
    comparer(p, titre, texte, &lire_offre(titre, texte), auj())
}

// ---------------------------------------------------------------------------------------------------------------
// La première moitié : six CV qui ont servi à écrire les règles
// ---------------------------------------------------------------------------------------------------------------

const CARISTE: &str = "Lucien Tartempion
12 rue des Fictions, 57000 Metz
06 00 00 00 00

Cariste

EXPÉRIENCES PROFESSIONNELLES
2019 - 2022 : Cariste chez Logi-Fiction, Woippy
- Conduite de chariots élévateurs (CACES 3 et 5)
- Chargement et déchargement des camions
- Rangement des palettes
2016 - 2019 : Préparateur de commandes - Entrepôts Imaginaires, Ennery
- Préparation de commandes au vocal
2015 : Manutentionnaire en intérim

FORMATION
2014 : CAP Agent d'entreposage et de messagerie - Lycée Inventé, Metz

PERMIS
Permis B, CACES 1, 3 et 5

LOISIRS
Football, pêche";

#[test]
fn cv_d_un_cariste() {
    let p = lire(CARISTE);
    assert_eq!(
        metiers(&p),
        [("Cariste", D::Mois(36)), ("Préparateur de commandes", D::Mois(36)), ("Manutentionnaire", D::Rien)]
    );
    assert_eq!(
        p.competences,
        [
            "Conduite de chariots élévateurs",
            "Chargement et déchargement des camions",
            "Rangement des palettes",
            "Préparation de commandes au vocal"
        ]
    );
    assert_eq!(p.diplomes, ["CAP Agent d'entreposage et de messagerie"]);
    assert_eq!(p.permis, ["Permis B", "CACES 1, 3, 5"]);
    assert_eq!(p.diplome_le_plus_haut(), Some(Diplome::CapBep));
    assert!(p.utilisable());
}

const VENDEUSE: &str = "Clémentine Fauxnom
Vendeuse

Expérience

Mars 2020 à aujourd'hui
Vendeuse en prêt-à-porter, Boutique Chiffon-Fictif (Nancy)
Accueil et conseil des clientes, encaissement, mise en rayon

Septembre 2017 - février 2020
Hôtesse de caisse, Supermarché Inventé
Tenue de la caisse et accueil

Compétences : encaissement, mise en rayon, réassort, conseil client, Excel
Qualités : souriante, ponctuelle, sens du contact

Diplômes
2017 - Bac pro Commerce
2015 - CAP Vente

Permis B";

#[test]
fn cv_d_une_vendeuse() {
    let p = lire(VENDEUSE);
    assert_eq!(metiers(&p), [("Vendeuse en prêt-à-porter", D::Depuis(2020, 3)), ("Hôtesse de caisse", D::Mois(30))]);
    assert_eq!(
        p.competences,
        [
            "Encaissement",
            "Mise en rayon",
            "Réassort",
            "Conseil client",
            "Excel",
            "Accueil et conseil des clientes",
            "Tenue de la caisse et accueil"
        ]
    );
    assert_eq!(p.diplomes, ["Bac pro Commerce", "CAP Vente"]);
    assert_eq!(p.permis, ["Permis B"]);
    assert_eq!(p.diplome_le_plus_haut(), Some(Diplome::Bac));
    // (un poste en cours : sa durée se compte le jour venu, de mars 2020 à novembre 2026)
    assert_eq!(p.metiers[0].duree(auj()), Some(81));
    assert_eq!(p.metiers[0].duree_texte(auj()), "depuis 6 ans");
    assert_eq!(p.metiers[1].duree_texte(auj()), "2 ans");
}

const AIDE_SOIGNANT: &str = "BERTRAND INVENTÉ - Aide-soignant diplômé
Né le 03/05/1988 - Permis B et véhicule

PARCOURS PROFESSIONNEL
09/2015 - 06/2021 : Aide-soignant, EHPAD Les Mirages (Thionville)
  • Aide à la toilette et aux repas
  • Surveillance de l'état de santé des résidents
  • Transmissions écrites et orales
07/2021 - aujourd'hui : Aide-soignant de nuit - Clinique Imaginaire
02/2013 - 08/2015 : Agent de service hospitalier à l'Hôpital Fictif

FORMATION
2015 : Diplôme d'État d'aide-soignant (DEAS), IFAS de Nulle-Part
2012 : BEP Carrières sanitaires et sociales
AFGSU niveau 2 (2023)";

#[test]
fn cv_d_un_aide_soignant() {
    let p = lire(AIDE_SOIGNANT);
    assert_eq!(
        metiers(&p),
        [
            ("Aide-soignant", D::Mois(70)),
            ("Aide-soignant de nuit", D::Depuis(2021, 7)),
            ("Agent de service hospitalier", D::Mois(31))
        ]
    );
    // (la troisième puce, trop longue pour une ligne de missions, donne son geste et son objet)
    assert_eq!(
        p.competences,
        ["Aide à la toilette et aux repas", "Transmissions écrites et orales", "Surveillance de l'état de santé"]
    );
    assert_eq!(p.diplomes, ["Diplôme d'État d'aide-soignant", "BEP Carrières sanitaires et sociales"]);
    assert_eq!(p.permis, ["Permis B", "AFGSU"]);
    assert_eq!(p.diplome_le_plus_haut(), Some(Diplome::CapBep));
}

const CHAUFFEUR: &str = "Raoul Camionfictif, 52 ans
Chauffeur poids lourd depuis 2009 chez Transports Chimère (Saint-Avold) : régional et national, frigo et bâché.
Avant : chauffeur livreur pendant 4 ans, puis 3 ans de manutention.
Permis B, C et CE, FIMO et FCO à jour, carte conducteur, ADR de base.
CAP Conducteur routier marchandises (1998).";

#[test]
fn cv_d_un_chauffeur_poids_lourd() {
    let p = lire(CHAUFFEUR);
    assert_eq!(metiers(&p), [("Chauffeur poids lourd", D::Depuis(2009, 1)), ("Chauffeur livreur", D::Mois(48))]);
    assert_eq!(p.permis, ["Permis B", "Permis C", "Permis CE", "FIMO", "FCO", "ADR"]);
    assert_eq!(p.diplomes, ["CAP Conducteur routier marchandises"]);
    assert!(p.competences.is_empty(), "{:?}", p.competences);
    assert_eq!(p.metiers[0].duree_texte(auj()), "depuis 17 ans");
}

const DEVELOPPEUSE: &str = "Zoé Pixel-Imaginaire
Développeuse web full stack

À propos
Développeuse passionnée, 6 ans d'expérience en agence et en startup.

Expériences
Depuis janvier 2022 | Développeuse full stack | Startup Licorne-Fictive, Paris
Conception d'API REST en Node.js, front en React, tests automatisés.
2019 - 2021 | Développeuse front-end | Agence Web Inventée, Lyon
2018 (6 mois) | Stagiaire développeuse web | Studio Mirage

Compétences techniques
Langages : JavaScript, TypeScript, PHP, SQL
Outils : Git, Docker, Jira
Méthodes agiles (Scrum)

Formation
2018 : Master Informatique, Université de Nulle-Part
2016 : Licence Informatique

Langues
Anglais courant";

#[test]
fn cv_d_une_developpeuse() {
    let p = lire(DEVELOPPEUSE);
    assert_eq!(
        metiers(&p),
        [
            ("Développeuse", D::Mois(72)),
            ("Développeuse full stack", D::Depuis(2022, 1)),
            ("Développeuse front-end", D::Mois(24)),
            ("Stagiaire développeuse web", D::Mois(6))
        ]
    );
    assert_eq!(
        p.competences,
        [
            "JavaScript",
            "TypeScript",
            "PHP",
            "SQL",
            "Git",
            "Docker",
            "Jira",
            "Méthodes agiles (Scrum)",
            "Conception d'API REST en Node.js",
            "Front en React",
            "Tests automatisés"
        ]
    );
    assert_eq!(p.diplomes, ["Master Informatique", "Licence Informatique"]);
    assert_eq!(p.diplome_le_plus_haut(), Some(Diplome::Bac5));
    assert!(p.permis.is_empty());
}

const COMPTABLE: &str = "Mme Odile Bilan-Fictif
Comptable confirmée

EXPERIENCE
Comptable général Cabinet Chiffres Imaginaires de janvier 2018 à mars 2024
Tenue de la comptabilité de 40 dossiers, déclarations de TVA, bilans
Aide-comptable Garage Inventé SARL de juin 2015 à décembre 2017
Saisie des factures, rapprochements bancaires

DIPLOMES
BTS Comptabilité et gestion (2015)
Baccalauréat STMG (2013)

LOGICIELS
Sage, Ciel Compta, Excel (tableaux croisés)

Divers : permis B, anglais scolaire";

#[test]
fn cv_d_une_comptable() {
    let p = lire(COMPTABLE);
    assert_eq!(metiers(&p), [("Comptable général", D::Mois(75)), ("Aide-comptable", D::Mois(31))]);
    assert_eq!(
        p.competences,
        [
            "Sage",
            "Ciel Compta",
            "Excel (tableaux croisés)",
            "Tenue de la comptabilité de 40 dossiers",
            "Déclarations de TVA",
            "Bilans",
            "Saisie des factures",
            "Rapprochements bancaires"
        ]
    );
    assert_eq!(p.diplomes, ["BTS Comptabilité et gestion", "Baccalauréat STMG"]);
    assert_eq!(p.diplome_le_plus_haut(), Some(Diplome::Bac2));
    assert_eq!(p.permis, ["Permis B"]);
}

/// Rien ne casse : un texte vide, illisible, presque vide, ou qui n'est pas un CV donne une fiche vide.
#[test]
fn textes_vides_ou_illisibles() {
    for t in [
        "",
        "   \n\n  ",
        "?????",
        "Bonjour",
        "Jean Exemple\n06 00 00 00 00",
        "\u{fffd}\u{fffd}\u{0}\u{1}",
        "lorem ipsum dolor sit amet",
        ":::::\n-----\n•••",
        "2019 - 2022",
        "EXPERIENCE\nFORMATION\nCOMPETENCES",
    ] {
        let p = lire(t);
        assert!(p.vide() && !p.utilisable(), "{t:?} : {p:?}");
        // (et une offre comparée à une fiche vide est loin, sans raison)
        let r = rapprocher(&p, "Cariste H/F", "CACES 3 exigé.");
        assert_eq!((r.niveau, r.raison.as_str(), r.rang), (Proximite::Loin, "", 0));
    }
    // (un très long texte sans retour à la ligne, des lignes de symboles : pas de panique)
    let long = "cariste ".repeat(5000);
    let _ = lire(&long);
    let _ = lire(&"é".repeat(40_000));
    let _ = lire(&"2019 - ".repeat(3000));
}

/// Les durées, une à une.
#[test]
fn dates_et_durees() {
    let lit = |l: &str| {
        let mut j = jetons(l);
        let d = dates_de(l, &mut j);
        (d.vue, d.mois, d.depuis)
    };
    for (ligne, attendu) in [
        ("2019 - 2022", (true, Some(36), None)),
        ("2019-2022", (true, Some(36), None)),
        ("de 2019 à 2022", (true, Some(36), None)),
        ("2019/2022", (true, Some(36), None)),
        ("mars 2020 à juin 2020", (true, Some(4), None)),
        ("03/2018 - 09/2019", (true, Some(19), None)),
        ("de juin 2015 à décembre 2017", (true, Some(31), None)),
        ("Juillet 2025 à août 2025", (true, Some(2), None)),
        ("du 01/03/2020 au 30/06/2020", (true, Some(4), None)),
        ("mars 2020 à aujourd'hui", (true, None, Some((2020, 3)))),
        ("2021 - présent", (true, None, Some((2021, 1)))),
        ("depuis 2021", (true, None, Some((2021, 1)))),
        ("depuis le 15 septembre 2021", (true, None, Some((2021, 9)))),
        ("Depuis janvier 2024", (true, None, Some((2024, 1)))),
        ("3 ans", (true, Some(36), None)),
        ("pendant 6 mois", (true, Some(6), None)),
        ("1 an et demi", (true, Some(18), None)),
        ("deux ans et 3 mois", (true, Some(27), None)),
        ("2018 (6 mois)", (true, Some(6), None)),
        ("2019", (true, None, None)),
        ("2019 - 2019", (true, None, None)),
        // (un âge, un code postal, un téléphone, une année hors du temps : pas des durées)
        ("52 ans", (false, None, None)),
        ("âgé de 12 ans", (false, None, None)),
        ("57000 Metz", (false, None, None)),
        ("06 12 34 56 78", (false, None, None)),
        ("1 semaine", (false, None, None)),
        ("14 heures", (false, None, None)),
        ("en 1789", (false, None, None)),
    ] {
        assert_eq!(lit(ligne), attendu, "{ligne}");
    }
    assert_eq!(duree_en_mots(1), "1 mois");
    assert_eq!(duree_en_mots(11), "11 mois");
    assert_eq!(duree_en_mots(12), "1 an");
    assert_eq!(duree_en_mots(19), "1 an et demi");
    assert_eq!(duree_en_mots(24), "2 ans");
    assert_eq!(duree_en_mots(47), "3 ans");
}

/// Les mots ramenés au masculin singulier, et les noms de poste coupés avant l'entreprise ou la ville.
#[test]
fn mots_et_noms_de_poste() {
    for (mot, attendu) in [
        ("vendeuses", "vendeur"),
        ("conductrice", "conducteur"),
        ("caissière", "caissier"),
        ("employée", "employe"),
        ("hôtesse", "hote"),
        ("techniciennes", "technicien"),
        ("commandes", "commande"),
        ("poids", "poids"),
        ("commerciaux", "commercial"),
        ("vente", "vente"),
    ] {
        assert_eq!(racine(&normaliser(mot)), attendu, "{mot}");
    }
    let poste = |l: &str| {
        let mut j = jetons(l);
        lire_dates(l, &mut j);
        postes_de(l, &j).into_iter().next().map(|p| p.nom)
    };
    for (ligne, attendu) in [
        ("Cariste chez Lidl-Fictif", Some("Cariste")),
        ("Cariste Auchan-Inventé Metz", Some("Cariste")),
        ("Vendeuse en boulangerie à Metz", Some("Vendeuse en boulangerie")),
        ("Aide à domicile - Domi-Truc", Some("Aide à domicile")),
        ("CHAUFFEUR POIDS LOURD TRANSPORTS CHIMERE", Some("Chauffeur poids lourd")),
        ("Chauffeur Poids Lourd", Some("Chauffeur Poids Lourd")),
        ("Agent Logistique Entrepôts Imaginaires", Some("Agent Logistique")),
        ("Société Dupont-Fictif, agent de production (2 ans)", Some("Agent de production")),
        ("J'ai travaillé 3 ans comme cariste chez Lidl-Fictif", Some("Cariste")),
        ("Expérience de 5 ans en tant que vendeuse en prêt-à-porter", Some("Vendeuse en prêt-à-porter")),
        ("Apprenti boulanger, Boulangerie Fictive", Some("Apprenti boulanger")),
        ("Cariste expérimenté (5 ans)", Some("Cariste")),
        ("Technicien de maintenance en CDI", Some("Technicien de maintenance")),
        // (un nom large seul, un poste visé, une ligne de mission, un nom de personne ou d'entreprise : rien)
        ("Agent", None),
        ("Responsable", None),
        ("Je recherche un poste de cariste", None),
        ("Formation de cariste", None),
        ("Conduite de chariots", None),
        ("Jean Boulanger-Fictif", None),
        ("Stage de 3ème - Garage Tournevis", None),
        ("Carrefour-Fictif Market", None),
    ] {
        assert_eq!(poste(ligne).as_deref(), attendu, "{ligne}");
    }
}

/// Les permis et les habilitations, et ceux qui ne sont pas encore là.
#[test]
fn permis_et_habilitations() {
    let p =
        lire("Permis B, C et CE\nCACES R489 cat 1 et 3\nCaces 5\nHabilitations électriques B1V, BR\nSST à jour, HACCP");
    assert_eq!(
        p.permis,
        ["Permis B", "Permis C", "Permis CE", "CACES 1, 3, 5", "Habilitation électrique", "SST", "HACCP"]
    );
    let p = lire("Permis B en cours\nPermis C prévu en 2027\nPas de permis\nCACES 3");
    assert_eq!(p.permis, ["CACES 3"]);
    assert_eq!(lire("Titulaire du permis de conduire").permis, ["Permis de conduire"]);
    assert_eq!(lire("Permis poids lourd et FIMO").permis, ["Permis poids lourd", "FIMO"]);
    assert_eq!(lire("Le permis, c'est prévu.").permis, Vec::<String>::new());
    assert_eq!(habilitations("habilité B0 H0V"), ["Habilitation électrique"]);
    assert!(habilitations("Je suis habile de mes mains").is_empty());
    // Ce que la personne a, face à ce qu'une offre demande
    let mien = |fiche: &str, demande: &str| {
        let f = droits(fiche);
        droits(demande).iter().all(|d| possede(&f, d))
    };
    assert!(mien("Permis B", "Permis B"));
    assert!(mien("Permis C", "Permis B"));
    assert!(mien("Permis CE", "Permis C"));
    assert!(mien("Permis CE", "Permis poids lourd"));
    assert!(!mien("Permis B", "Permis C"));
    assert!(!mien("Permis C", "Permis CE"));
    assert!(mien("CACES 1, 3, 5", "CACES 3"));
    assert!(mien("CACES 1, 3, 5", "CACES"));
    assert!(mien("CACES", "CACES 3"));
    assert!(!mien("CACES 3", "CACES 5"));
    assert!(!mien("Permis B", "CACES 3"));
    assert!(mien("Habilitation électrique", "habilitation électrique BR"));
    assert!(!mien("SST", "habilitation électrique"));
}

// ---------------------------------------------------------------------------------------------------------------
// La comparaison d'une offre à une fiche
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn offre_du_meme_metier_avec_le_caces() {
    let p = lire(CARISTE);
    let r = rapprocher(
        &p,
        "Cariste H/F",
        "Vous chargez et déchargez les camions. CACES 3 exigé. Débutant accepté, aucun diplôme demandé.",
    );
    assert_eq!(r.niveau, Proximite::TresProche, "{r:?}");
    assert_eq!(r.raison, "Tu as été cariste 3 ans, et tu as le CACES 3 demandé.");
    // (le même métier écrit autrement)
    let r = rapprocher(&p, "Conducteur de chariot élévateur (H/F)", "Poste en 2x8. CACES 5 obligatoire.");
    assert_eq!(r.niveau, Proximite::TresProche, "{r:?}");
    assert_eq!(r.raison, "Tu as été cariste 3 ans, et tu as le CACES 5 demandé.");
    // (un CACES que la personne n'a pas)
    let r = rapprocher(&p, "Cariste", "CACES 4 obligatoire.");
    assert_eq!(r.niveau, Proximite::Proche, "{r:?}");
    assert_eq!(r.raison, "Tu as été cariste 3 ans, mais il te manque le CACES 4.");
    // (un métier voisin : la logistique)
    let r = rapprocher(&p, "Magasinier H/F", "Réception des marchandises et tenue du magasin.");
    assert_eq!(r.niveau, Proximite::UnPeu, "{r:?}");
    assert_eq!(r.raison, "Ce poste ressemble à un métier que tu as fait (cariste).");
}

#[test]
fn le_meme_mot_dans_un_autre_sens() {
    let chef = fiche(&[("Chef de rayon", D::Mois(84))], &["Mise en rayon", "Gestion des stocks"], &[], &["Permis B"]);
    let r = rapprocher(&chef, "Chef de projet informatique H/F", "Vous pilotez des projets web. Bac+5 exigé.");
    assert_eq!((r.niveau, r.raison.as_str()), (Proximite::Loin, ""));
    let r = rapprocher(&chef, "Chef de rayon fruits et légumes", "Vous animez votre rayon.");
    assert_eq!(r.niveau, Proximite::TresProche, "{r:?}");
    assert_eq!(r.raison, "Tu as été chef de rayon 7 ans.");
    let ligne = fiche(&[("Conducteur de ligne", D::Mois(40))], &[], &[], &[]);
    assert_eq!(rapprocher(&ligne, "Conducteur de bus H/F", "Permis D exigé.").niveau, Proximite::Loin);
    assert_eq!(
        rapprocher(&ligne, "Conducteur de ligne de production", "Travail en 3x8.").niveau,
        Proximite::TresProche
    );
    let entretien = fiche(&[("Agent d'entretien", D::Mois(24))], &[], &[], &[]);
    assert_eq!(rapprocher(&entretien, "Agent de sécurité H/F", "Carte professionnelle.").niveau, Proximite::Loin);
    assert_eq!(rapprocher(&entretien, "Agent de propreté", "Nettoyage de bureaux.").niveau, Proximite::TresProche);
    let aide = fiche(&[("Aide-soignante", D::Mois(60))], &[], &[], &[]);
    assert_eq!(rapprocher(&aide, "Aide-comptable", "Saisie des factures.").niveau, Proximite::Loin);
    assert_eq!(rapprocher(&aide, "Aide-soignant(e) en EHPAD", "Soins d'hygiène.").niveau, Proximite::TresProche);
}

#[test]
fn diplome_hors_de_portee_et_experience() {
    let p = lire(COMPTABLE);
    let r =
        rapprocher(&p, "Comptable H/F", "Tenue de la comptabilité. Diplôme exigé : Master (Bac+5) en comptabilité.");
    assert_eq!(r.niveau, Proximite::UnPeu, "{r:?}");
    assert_eq!(r.raison, "Tu as été comptable général 6 ans, mais l'offre demande un Bac+5.");
    let r = rapprocher(&p, "Comptable H/F", "BTS comptabilité exigé. Maîtrise de Sage et d'Excel.");
    assert_eq!(r.niveau, Proximite::TresProche, "{r:?}");
    assert_eq!(
        r.raison,
        "Tu as été comptable général 6 ans, et l'offre demande des choses que tu sais faire : Sage, Excel."
    );
    // (l'expérience demandée dépasse de loin celle du métier exercé)
    let court = fiche(&[("Cuisinier", D::Mois(6))], &[], &["CAP Cuisine"], &[]);
    let r = rapprocher(&court, "Cuisinier confirmé", "5 ans d'expérience minimum exigés sur un poste similaire.");
    assert_eq!(r.niveau, Proximite::UnPeu, "{r:?}");
    assert_eq!(r.raison, "Tu as été cuisinier 6 mois, mais l'offre demande 5 ans d'expérience.");
}

#[test]
fn metier_jamais_exerce_mais_competences_communes() {
    let p = lire(VENDEUSE);
    let r = rapprocher(
        &p,
        "Employé polyvalent de restauration rapide H/F",
        "Vous prenez les commandes et assurez l'encaissement. Mise en rayon des produits et nettoyage de la salle.",
    );
    assert_eq!(r.niveau, Proximite::UnPeu, "{r:?}");
    assert_eq!(
        r.raison,
        "Tu n'as pas fait ce métier, mais l'offre demande des choses que tu sais faire : encaissement, mise en rayon."
    );
    // (sans aucun rapport : loin, sans raison)
    let r = rapprocher(&p, "Soudeur TIG", "Soudure sur inox. Lecture de plans.");
    assert_eq!((r.niveau, r.raison.as_str()), (Proximite::Loin, ""));
    // (le tri : la plus proche a le plus grand rang)
    let proche = rapprocher(&p, "Vendeuse prêt-à-porter", "Encaissement et réassort.");
    assert!(proche.rang > r.rang && proche.niveau == Proximite::TresProche, "{proche:?}");
    assert_eq!(
        proche.raison,
        "Tu es vendeuse en prêt-à-porter depuis 6 ans, et l'offre demande des choses que tu sais faire : encaissement, réassort."
    );
}

/// Aucune raison ne porte de note chiffrée, de tiret long ni de balise.
#[test]
fn les_raisons_sont_des_phrases_simples() {
    let offres = [
        ("Cariste H/F", "CACES 3 exigé. Débutant accepté."),
        ("Vendeur en magasin", "Encaissement. Bac+2 exigé."),
        ("Comptable", "5 ans d'expérience exigés. Excel."),
        ("Aide-soignant de nuit", "DEAS exigé."),
        ("Chauffeur PL", "Permis C et FIMO exigés."),
        ("Développeur web", "PHP, SQL, Git."),
    ];
    for cv in [CARISTE, VENDEUSE, AIDE_SOIGNANT, CHAUFFEUR, DEVELOPPEUSE, COMPTABLE] {
        let p = lire(cv);
        for (titre, texte) in offres {
            let r = rapprocher(&p, titre, texte);
            assert_eq!(r.raison.is_empty(), r.niveau == Proximite::Loin, "{r:?}");
            assert!(!r.raison.contains(['\u{2013}', '\u{2014}', '<', '>', '%']), "{}", r.raison);
            assert!(!r.raison.contains("  ") && !r.raison.contains("/100"), "{}", r.raison);
            assert!(r.raison.is_empty() || r.raison.ends_with('.'), "{}", r.raison);
        }
    }
    assert_eq!(Proximite::TresProche.nom(), "Très proche de ton parcours");
    assert_eq!(Proximite::Proche.nom(), "Proche de ton parcours");
    assert_eq!(Proximite::UnPeu.nom(), "Un peu proche de ton parcours");
    assert_eq!(Proximite::Loin.nom(), "");
    assert_eq!(
        [Proximite::TresProche, Proximite::Proche, Proximite::UnPeu, Proximite::Loin].map(Proximite::code),
        ["tres", "proche", "peu", "loin"]
    );
}

// ---------------------------------------------------------------------------------------------------------------
// Les corrections de la personne
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn corrections_retraits_et_ajouts() {
    let p = lire(CARISTE);
    let retrait =
        |l: Liste, texte: &str| Correction { liste: l, ajout: false, cle: cle_ligne(l, texte), texte: String::new() };
    let ajout = |l: Liste, texte: &str| {
        let t = verifier_ajout(l, texte).unwrap();
        Correction { liste: l, ajout: true, cle: cle_ligne(l, &t), texte: t }
    };
    let c = [
        retrait(Liste::Metiers, "Manutentionnaire"),
        retrait(Liste::Competences, "Rangement des palettes"),
        retrait(Liste::Permis, "permis b"),
        // (une ligne retirée que la fiche n'a pas : rien ne change)
        retrait(Liste::Diplomes, "Master Droit"),
        ajout(Liste::Metiers, "magasinier, 2 ans"),
        ajout(Liste::Metiers, "Agent de quai depuis 2023"),
        ajout(Liste::Competences, "inventaire"),
        ajout(Liste::Diplomes, "bac pro Logistique"),
        ajout(Liste::Permis, "ce"),
        ajout(Liste::Permis, "habilitation électrique"),
        // (une ligne ajoutée que la fiche a déjà : pas de doublon)
        ajout(Liste::Metiers, "caristes"),
        ajout(Liste::Permis, "CACES 1, 3, 5"),
    ];
    let q = p.corrige(&c);
    assert_eq!(
        metiers(&q),
        [
            ("Cariste", D::Mois(36)),
            ("Préparateur de commandes", D::Mois(36)),
            ("Magasinier", D::Mois(24)),
            ("Agent de quai", D::Depuis(2023, 1))
        ]
    );
    assert_eq!(
        q.competences,
        [
            "Conduite de chariots élévateurs",
            "Chargement et déchargement des camions",
            "Préparation de commandes au vocal",
            "Inventaire"
        ]
    );
    assert_eq!(q.diplomes, ["CAP Agent d'entreposage et de messagerie", "Bac pro Logistique"]);
    assert_eq!(q.permis, ["CACES 1, 3, 5", "Permis CE", "Habilitation électrique"]);
    assert_eq!(q.diplome_le_plus_haut(), Some(Diplome::Bac));
    // La fiche corrigée sert à comparer : le permis ajouté compte, la ligne retirée ne compte plus
    let r = rapprocher(&q, "Chauffeur poids lourd", "Permis CE exigé.");
    assert!(r.raison.contains("tu as le permis CE demandé"), "{r:?}");
    assert_eq!(rapprocher(&p, "Manutentionnaire", "Port de charges.").niveau, Proximite::Proche);
    assert_eq!(rapprocher(&q, "Manutentionnaire", "Port de charges.").niveau, Proximite::UnPeu);
    // Sans correction, la fiche est la même ; les mêmes corrections sur un autre CV : seules celles qui s'y
    // retrouvent jouent, les ajouts restent
    assert_eq!(p.corrige(&[]), p);
    let autre = lire(VENDEUSE).corrige(&c);
    assert_eq!(autre.permis, ["Permis CE", "Habilitation électrique", "CACES 1, 3, 5"]);
    assert_eq!(autre.metiers.len(), 5);
    // Les lignes refusées
    assert!(verifier_ajout(Liste::Metiers, "  ").is_err());
    assert!(verifier_ajout(Liste::Competences, "x").is_err());
    assert!(verifier_ajout(Liste::Diplomes, &"a".repeat(61)).is_err());
    assert_eq!(verifier_ajout(Liste::Permis, "b").unwrap(), "Permis B");
    assert_eq!(verifier_ajout(Liste::Competences, "  conduite   de chariot\n").unwrap(), "Conduite de chariot");
    for l in Liste::TOUTES {
        assert_eq!(Liste::depuis_code(l.code()), Some(l));
        assert!(l.code().bytes().all(|b| b.is_ascii_lowercase()) && l.code().len() <= 8);
    }
    assert_eq!(Liste::depuis_code("autre"), None);
    assert_ne!(empreinte(CARISTE), empreinte(VENDEUSE));
    assert_eq!(empreinte(CARISTE), empreinte(CARISTE));
}

/// Le CV d'exemple de la construction d'essai (jobmind-app, src\essai.rs, `CV_EXEMPLE`, recopié ici) : il n'a pas de
/// rubrique « Compétences », et ses deux lignes d'expérience ne disent que des noms de poste. Rien n'est inventé à
/// partir d'un nom de poste (« préparateur de commandes » ne donne pas « préparation de commandes ») ; le seul
/// savoir-faire écrit est sous « À côté ».
#[test]
fn cv_d_exemple_de_l_essai() {
    let cv = "Jordan Exemple\nMetz (57), permis B\n\nExpérience\n2023 à 2025 : préparateur de commandes et cariste en \
              intérim, entrepôt de Woippy\n2021 à 2022 : vendeur dans un magasin de sport, Metz\n\nFormation\nCAP \
              Opérateur logistique, 2018\nCACES 1, 3 et 5\n\nÀ côté\nDépannage et montage d'ordinateurs pour les proches";
    let p = lire(cv);
    assert_eq!(p.competences, ["Dépannage et montage d'ordinateurs"]);
    assert_eq!(p.diplomes, ["CAP Opérateur logistique"]);
    assert_eq!(p.permis, ["Permis B", "CACES 1, 3, 5"]);
}

/// Les savoir-faire écrits dans une phrase ou entre parenthèses : un geste de métier et son objet, jamais une
/// qualité, jamais un nom de poste, jamais un bout de phrase.
#[test]
fn savoir_faire_dans_les_phrases() {
    let c = |ligne: &str| lire(&format!("Expérience\n2020 - 2023 : Cariste, Logi-Faux\n{ligne}")).competences;
    let rien: [&str; 0] = [];
    assert_eq!(
        c("J'ai assuré la préparation de commandes et la conduite de chariots élévateurs."),
        ["Préparation de commandes", "Conduite de chariots élévateurs"]
    );
    assert_eq!(c("Je m'occupais de la réception des marchandises chaque matin."), ["Réception des marchandises"]);
    assert_eq!(c("En charge du contrôle des bons de livraison."), ["Contrôle des bons de livraison"]);
    assert_eq!(c("Je faisais l'encaissement et la mise en rayon."), ["Encaissement", "Mise en rayon"]);
    assert_eq!(c("Je participais à l'inventaire."), ["Inventaire"]);
    // Entre parenthèses, sur la ligne du poste
    assert_eq!(
        lire("Expérience\n2020 - 2023 : Vendeuse, Boutique Fausse (vente, encaissement, mise en rayon)").competences,
        ["Vente", "Encaissement", "Mise en rayon"]
    );
    // Un nom de poste n'est pas un savoir-faire
    assert_eq!(lire("Expérience\n2020 - 2023 : Agent d'entretien des locaux, Mairie de Faux").competences, rien);
    assert_eq!(lire("Expérience\n2020 - 2023 : Préparateur de commandes en intérim").competences, rien);
    // Ni une qualité, ni un endroit, ni un geste sans son objet, ni l'activité de l'entreprise
    assert_eq!(c("J'ai une bonne gestion du stress et le sens de l'accueil."), rien);
    assert_eq!(c("J'étais à l'accueil de la mairie."), rien);
    assert_eq!(c("J'ai participé à la préparation et à la conduite."), rien);
    assert_eq!(c("Une société de nettoyage industriel de la région."), rien);
    assert_eq!(c("J'étais au service de la clientèle."), rien);
    // L'objet s'arrête avant la suite de la phrase
    assert_eq!(c("J'assurais la livraison de colis pour les clients du quartier."), ["Livraison de colis"]);
    assert_eq!(c("La réception des marchandises de 6 h à 14 h."), ["Réception des marchandises"]);
    assert_eq!(c("La préparation de commandes était ma mission."), ["Préparation de commandes"]);
}
