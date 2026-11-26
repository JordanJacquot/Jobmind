//! La quatrième série : six CV inventés pour mesurer les compétences écrites dans une phrase ou entre parenthèses
//! (un savoir-faire dans une phrase d'expérience, dans une liste entre parenthèses, à la suite d'un poste), écrits
//! sans regarder la règle, avec ce qui en était attendu. La mesure faite telle quelle, avant toute correction, est
//! notée en tête de [`quatrieme_serie`].

use super::*;

/// Les compétences attendues d'un CV, et ce qui ne doit jamais en sortir.
struct Attendues {
    nom: &'static str,
    cv: &'static str,
    competences: &'static [&'static str],
    pas_competences: &'static [&'static str],
    /// Les lignes sorties sans être attendues, relues à la main : vraies et écrites dans le CV
    en_plus: &'static [&'static str],
}

const D1: Attendues = Attendues {
    nom: "D1, un préparateur de commandes qui écrit en phrases",
    cv: "Karim BIDULE
Préparateur de commandes

Expériences professionnelles
2022 - 2025 : Préparateur de commandes, Entrepôts Machin (Saint-Priest)
J'ai assuré la préparation de commandes au vocal et la conduite de chariots élévateurs. Je participais aussi à l'inventaire chaque fin de mois.
2020 - 2022 : Agent de quai, Transports Truc
Chargé de la réception des marchandises, du contrôle des bons de livraison et du chargement des camions.

Formation
2019 : CAP Opérateur logistique

Permis B, CACES 1 et 3
Sérieux et ponctuel",
    competences: &[
        "préparation de commandes",
        "conduite de chariots",
        "inventaire",
        "réception des marchandises",
        "contrôle des bons de livraison",
        "chargement des camions",
    ],
    pas_competences: &["sérieux", "ponctuel", "fin de mois", "j'ai", "chargé de"],
    en_plus: &[],
};

const D2: Attendues = Attendues {
    nom: "D2, une vendeuse, tout entre parenthèses",
    cv: "Solène FAUXNOM - Vendeuse
06 00 00 00 00

EXPÉRIENCE
Vendeuse en boulangerie - Au Pain Inventé, Reims - 2021 à 2024 (vente, encaissement, mise en rayon, nettoyage du magasin)
Employée polyvalente - Supérette Bidule - 2019 à 2021 (tenue de caisse, réassort, réception des livraisons)
Baby-sitting le week-end (2 enfants)

DIPLÔME
Bac pro Commerce, 2019

Souriante, dynamique, bon contact avec la clientèle",
    competences: &[
        "vente",
        "encaissement",
        "mise en rayon",
        "nettoyage du magasin",
        "tenue de caisse",
        "réassort",
        "réception des livraisons",
    ],
    pas_competences: &["souriante", "dynamique", "contact", "enfants", "inventé", "bidule"],
    en_plus: &[],
};

const D3: Attendues = Attendues {
    nom: "D3, une auxiliaire de vie, sans rubrique",
    cv: "Josiane Trucmuche, 52 ans, Limoges
Auxiliaire de vie depuis 2016 chez Domi-Bidule : j'accompagne des personnes âgées au quotidien. Je m'occupe de l'aide à la toilette, de la préparation des repas et de l'entretien du logement. Je fais aussi les courses.
Avant, j'ai été agent de service hospitalier pendant 8 ans (nettoyage des chambres, distribution des repas).
Diplôme d'État d'auxiliaire de vie sociale (2015). Permis B et voiture.",
    competences: &[
        "aide à la toilette",
        "préparation des repas",
        "entretien du logement",
        "nettoyage des chambres",
        "distribution des repas",
    ],
    pas_competences: &["quotidien", "voiture", "j'accompagne", "je fais", "personnes"],
    en_plus: &[],
};

const D4: Attendues = Attendues {
    nom: "D4, un commis de cuisine, des puces et des verbes",
    cv: "MARTIN Loïc
Commis de cuisine

Parcours
Depuis 2023 - Commis de cuisine, Brasserie du Faux-Pont
• Préparer les entrées froides et les desserts
• Mise en place avant le service
• Plonge et nettoyage de la cuisine
• Respect des règles d'hygiène HACCP
2021-2023 - Équipier polyvalent, Burger Machin
• Prise de commandes, encaissement, préparation des sandwichs

Études
CAP Cuisine (2021)

Langues : anglais scolaire
Loisirs : football, cuisine du monde",
    competences: &[
        "préparer les entrées froides",
        "mise en place",
        "plonge",
        "nettoyage de la cuisine",
        "prise de commandes",
        "encaissement",
        "préparation des sandwichs",
    ],
    pas_competences: &["football", "anglais", "cuisine du monde", "brasserie"],
    en_plus: &[],
};

const D5: Attendues = Attendues {
    nom: "D5, un technicien de maintenance, « Missions : » et des phrases",
    cv: "Technicien de maintenance - Yanis Exemple

EXPERIENCES PROFESSIONNELLES

Technicien de maintenance | Usine Bidule-Plast | 2018 - aujourd'hui
Missions : maintenance préventive et curative des presses à injecter, diagnostic des pannes, remplacement des pièces d'usure.
J'interviens aussi sur le réglage des machines lors des changements de série.

Électricien du bâtiment | SARL Courant-Faux | 2014 - 2018
Pose de tableaux électriques, tirage de câbles et mise en service des installations chez les particuliers.

FORMATIONS
BTS Maintenance des systèmes (2014)
Habilitation électrique B2V, BR

ATOUTS
Rigueur, autonomie, esprit d'équipe",
    competences: &[
        "maintenance préventive et curative",
        "diagnostic des pannes",
        "remplacement des pièces d'usure",
        "réglage des machines",
        "pose de tableaux électriques",
        "tirage de câbles",
        "mise en service des installations",
    ],
    pas_competences: &["rigueur", "autonomie", "esprit", "j'interviens", "particuliers", "changements"],
    en_plus: &[],
};

const D6: Attendues = Attendues {
    nom: "D6, une débutante, des qualités écrites en phrases",
    cv: "Emma Petitexemple
20 ans - Tours

Je cherche un premier emploi comme secrétaire ou hôtesse d'accueil. Je suis sérieuse, j'ai le sens de l'organisation et une bonne gestion du stress.

Stages
Mai 2025 (4 semaines) : stagiaire assistante administrative, Cabinet Machin : accueil téléphonique, classement des dossiers et saisie de courriers sous Word.
Juin 2024 : stage d'observation à la mairie de Truc-sur-Loire

Diplômes
Bac pro Gestion-Administration (2025)

Divers
Permis B en cours
Animation d'ateliers pour enfants au centre de loisirs (bénévole)",
    competences: &["accueil téléphonique", "classement des dossiers", "saisie de courriers"],
    pas_competences: &["stress", "organisation", "sérieuse", "observation", "premier emploi", "semaines"],
    // (du bénévolat écrit sous « Divers » : un vrai savoir-faire, pas prévu en écrivant le CV)
    en_plus: &["Animation d'ateliers"],
};

const SERIE: [&Attendues; 6] = [&D1, &D2, &D3, &D4, &D5, &D6];

/// Le bilan des compétences d'une série : les attendues trouvées, leur nombre, les lignes fausses (interdites, ou ni
/// attendues ni relues à la main), et le détail.
fn bilan_competences(cvs: &[&Attendues]) -> (usize, usize, usize, String) {
    let (mut trouvees, mut total, mut fausses) = (0, 0, 0);
    let mut rapport = String::new();
    for a in cvs {
        let p = lire(a.cv);
        let n = |x: &str| normaliser(x);
        rapport.push_str(&format!("\n{} : {:?}", a.nom, p.competences));
        for x in a.competences {
            total += 1;
            if p.competences.iter().any(|c| n(c).contains(&n(x))) {
                trouvees += 1;
            } else {
                rapport.push_str(&format!("\n    MANQUE {x}"));
            }
        }
        for c in &p.competences {
            let interdite = a.pas_competences.iter().any(|x| n(c).contains(&n(x)));
            let prevue = a.competences.iter().any(|x| n(c).contains(&n(x))) || a.en_plus.contains(&c.as_str());
            if interdite || !prevue {
                fausses += 1;
                rapport.push_str(&format!("\n    FAUSSE {c}"));
            }
        }
    }
    (trouvees, total, fausses, rapport)
}

/// La mesure. Telle quelle, avant toute correction : 30 compétences trouvées sur 35 attendues (86 %), aucune ligne
/// fausse dans ces six CV, une ligne en plus, vraie (« Animation d'ateliers »). Ce qui manquait : « contrôle des bons
/// de livraison » (l'objet coupé à son complément, « bons » pris pour une qualité), « vente » seule entre
/// parenthèses, et, sur une ligne « Missions : », ce que la règle des lignes de missions laissait tomber
/// (« maintenance préventive et curative », « tirage de câbles », « mise en service des installations »). La même
/// règle passée sur les vingt CV des autres séries a montré une ligne coupée trop tôt (« Surveillance de l'état »).
/// Après correction (le complément de l'objet, un geste seul dans une liste, les lignes de missions relues aussi
/// comme des phrases, les savoir-faire cherchés entre les noms de poste d'une même ligne) : 35 sur 35. Ce test garde
/// ce qui a été gagné, il ne mesure plus rien.
#[test]
fn quatrieme_serie() {
    let (trouvees, total, fausses, rapport) = bilan_competences(&SERIE);
    assert_eq!((trouvees, total, fausses), (35, 35, 0), "{rapport}");
}
