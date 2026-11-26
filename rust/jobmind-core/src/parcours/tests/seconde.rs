//! La seconde moitié : six CV et douze couples offre et fiche inventés à part (par une autre main, qui ne voyait pas
//! les règles), avec ce qui en était attendu. Mesurés une première fois tels quels, avant toute correction (le
//! résultat est noté en tête de [`mesure_de_la_seconde_moitie`]), puis les règles ont été corrigées.

use super::*;

/// Ce qui est attendu de la fiche d'un CV.
pub(super) struct Attendu {
    pub nom: &'static str,
    pub cv: &'static str,
    pub metiers: &'static [(&'static str, D)],
    /// Ce qui ne doit pas sortir comme métier (un mot du nom suffit à le reconnaître)
    pub pas_metiers: &'static [&'static str],
    pub permis: &'static [&'static str],
    pub diplome: Option<Diplome>,
    pub diplomes: &'static [&'static str],
    pub competences: &'static [&'static str],
    pub pas_competences: &'static [&'static str],
}

const A1: Attendu = Attendu {
    nom: "A1, un apprenti sans vraie expérience",
    cv: "Timéo PATATRAS
18 ans
3 allée des Cornichons, 72000 Le Mans
06 00 00 00 01 - timeo.patatras@exemple.fr
Permis B en cours (conduite accompagnée)

Recherche un contrat d'apprentissage en Bac pro Commerce - poste de vendeur conseil en magasin

FORMATION
2025 - 2026 : 1ère Bac pro Métiers du commerce et de la vente, Lycée Machin-Chose (en cours)
Juin 2025 : CAP Equipier polyvalent du commerce - obtenu
2023 : Brevet des collèges

EXPERIENCES
Juillet 2025 à août 2025 : Employé de mise en rayon (job d'été) - Supermarché Machin
- mise en rayon des produits frais
- rotation des dates
- rangement de la réserve
Janvier 2025 à février 2025 : Stagiaire vendeur - Sport-Bidule
- accueil des clients
- encaissement
- étiquetage
Stage de 3ème - Garage Tournevis (1 semaine d'observation)

CENTRES D'INTERET
Handball (en club depuis 6 ans), jeux vidéo, aide au club de foot le samedi (buvette)

QUALITES
Sérieux, ponctuel, motivé, souriant",
    metiers: &[("Employé de mise en rayon", D::Mois(2)), ("Stagiaire vendeur", D::Mois(2))],
    pas_metiers: &["conseil", "tournevis", "machin", "bidule", "handball", "bac"],
    permis: &[],
    diplome: Some(Diplome::CapBep),
    diplomes: &["CAP Equipier polyvalent du commerce", "Brevet des collèges"],
    competences: &["mise en rayon", "encaissement", "accueil des clients", "étiquetage"],
    pas_competences: &["sérieux", "ponctuel", "motivé", "souriant"],
};

const A2: Attendu = Attendu {
    nom: "A2, une reconversion",
    cv: "Gédéon TARTEMPION
47 ans - marié, 2 enfants
8 impasse du Pétrin, 21000 Dijon
Tél : 06 00 00 00 02

CONDUCTEUR ROUTIER (en reconversion professionnelle)

Après plus de 20 ans en boulangerie, j'ai passé mon permis poids lourd pour changer de métier.

FORMATION
2026 : Titre professionnel Conducteur du transport routier de marchandises sur porteur - Centre de formation Roule-Machin (obtenu en mai 2026)
Permis C + FIMO marchandises
Permis CE : en cours, examen prévu en janvier 2027
1998 : CAP Boulanger - CFA de la Miche

EXPERIENCE PROFESSIONNELLE
03/2026 - 04/2026 : Stagiaire conducteur poids lourd - Transports Zinzin
Livraisons en porteur 19 tonnes, tournées régionales
Chargement, sanglage et arrimage
Utilisation du chronotachygraphe
2012 - 2025 : Boulanger - Boulangerie Croustibidule
Fabrication des pains et viennoiseries, gestion des commandes de farine
Formation des apprentis
1999 - 2012 : Boulanger - Au Bon Pain de Trucmuche
Pétrissage, façonnage, cuisson

DIVERS
Permis B, véhicule personnel
Travail de nuit et horaires décalés : j'ai l'habitude
Rigoureux et ponctuel",
    metiers: &[("Boulanger", D::Mois(312)), ("Stagiaire conducteur poids lourd", D::Mois(2))],
    pas_metiers: &["routier", "transport", "zinzin", "croustibidule", "trucmuche", "apprentis"],
    // (dans l'ordre de la fiche : les permis, puis les habilitations)
    permis: &["Permis C", "Permis B", "FIMO"],
    diplome: Some(Diplome::CapBep),
    diplomes: &["Titre professionnel Conducteur du transport routier de marchandises sur porteur", "CAP Boulanger"],
    competences: &[
        "livraisons en porteur 19 tonnes",
        "chargement, sanglage et arrimage",
        "utilisation du chronotachygraphe",
        "fabrication des pains et viennoiseries",
        "gestion des commandes",
    ],
    pas_competences: &["rigoureux", "ponctuel"],
};

const A3: Attendu = Attendu {
    nom: "A3, un CV de deux lignes",
    cv: "Mauricette Bidulon - aide a domicile depuis 2021 chez Domi-Machin, avant agent d'entretien 3 ans (Net-Truc)
permis B, vehicule, BEP carrieres sanitaires et sociales, dispo de suite",
    metiers: &[("aide a domicile", D::Depuis(2021, 1)), ("agent d'entretien", D::Mois(36))],
    pas_metiers: &["machin", "truc", "bidulon", "carrieres"],
    permis: &["Permis B"],
    diplome: Some(Diplome::CapBep),
    diplomes: &["BEP Carrières sanitaires et sociales"],
    competences: &[],
    pas_competences: &["dispo"],
};

const A4: Attendu = Attendu {
    nom: "A4, un CV désordonné (deux colonnes mélangées)",
    cv: "Firmin CHARIOTTIN
Cariste magasinier
12 rue des Palettes 59000 Lille
06 00 00 00 04
firmin.chariottin@exemple.fr
COMPETENCES EXPERIENCES PROFESSIONNELLES
Conduite de chariots élévateurs mars 2020 à aujourd'hui
Chargement et déchargement de camions Cariste magasinier - Entrepôts Trucmuche, Lesquin
Gestion des stocks Réception des marchandises, rangement en palettier
Préparation de commandes Inventaires
Utilisation du logiciel de gestion d'entrepôt (WMS) 01/2018 - 02/2020
PERMIS ET HABILITATIONS Préparateur de commandes - Supermarché Machin Drive
CACES R489 catégories 1, 3 et 5 Préparation des commandes au scan
Permis B Filmage des palettes
SST (recyclage 2025) de juin 2015 à décembre 2017
LANGUES Cariste - Logi-Bidule
Anglais : notions Chargement des camions
LOISIRS FORMATION
Football en club, pêche 2015 Bac pro Logistique - Lycée Jules Machin",
    metiers: &[
        ("Cariste magasinier", D::Depuis(2020, 3)),
        ("Préparateur de commandes", D::Mois(26)),
        ("Cariste", D::Mois(31)),
    ],
    pas_metiers: &["trucmuche", "machin", "bidule", "football", "inventaires", "filmage", "chariottin"],
    permis: &["Permis B", "CACES 1, 3, 5", "SST"],
    diplome: Some(Diplome::Bac),
    diplomes: &["Bac pro Logistique"],
    competences: &[
        "conduite de chariots élévateurs",
        "chargement et déchargement de camions",
        "gestion des stocks",
        "préparation de commandes",
        "logiciel de gestion d'entrepôt",
    ],
    pas_competences: &["football", "pêche"],
};

const A5: Attendu = Attendu {
    nom: "A5, un CV tout en majuscules",
    cv: "CUNEGONDE FARFELU
14 RUE DES BRIOCHES 31000 TOULOUSE
TEL 06 00 00 00 05
PERMIS B - VEHICULE

EXPERIENCE PROFESSIONNELLE

DEPUIS 2024 : EMPLOYEE POLYVALENTE DE RESTAURATION - RESTO-BIDULE
PRISE DE COMMANDES, PREPARATION DES SANDWICHS ET SALADES, ENCAISSEMENT, NETTOYAGE DE LA SALLE

2022 - 2024 : HOTESSE DE CAISSE - SUPERMARCHE MACHIN
ENCAISSEMENT, ACCUEIL DES CLIENTS, GESTION DU FOND DE CAISSE

2019 - 2022 : VENDEUSE EN BOULANGERIE - BOULANGERIE CROUSTI-TRUC
VENTE, MISE EN VITRINE, OUVERTURE ET FERMETURE DU MAGASIN

FORMATION

2023 : FORMATION HACCP (HYGIENE ALIMENTAIRE) - 14 HEURES
2019 : NIVEAU BAC PRO COMMERCE (NON OBTENU)
2018 : CAP EMPLOYE DE VENTE SPECIALISE OPTION PRODUITS ALIMENTAIRES

QUALITES
SOURIANTE, PONCTUELLE, DYNAMIQUE, ESPRIT D'EQUIPE

LOISIRS
ZUMBA, CUISINE, BENEVOLE AUX RESTOS DU QUARTIER",
    metiers: &[
        ("EMPLOYEE POLYVALENTE DE RESTAURATION", D::Depuis(2024, 1)),
        ("HOTESSE DE CAISSE", D::Mois(24)),
        ("VENDEUSE EN BOULANGERIE", D::Mois(36)),
    ],
    pas_metiers: &["bidule", "machin", "truc", "specialise", "zumba", "benevole", "farfelu", "prise", "vitrine"],
    permis: &["Permis B", "HACCP"],
    diplome: Some(Diplome::CapBep),
    diplomes: &["CAP Employé de vente spécialisé option produits alimentaires"],
    competences: &[
        "encaissement",
        "accueil des clients",
        "prise de commandes",
        "mise en vitrine",
        "gestion du fond de caisse",
    ],
    pas_competences: &["souriante", "ponctuelle", "dynamique", "esprit", "zumba"],
};

const A6: Attendu = Attendu {
    nom: "A6, un CV avec des fautes",
    cv: "Gaston Brindezingue
5 rue du Moulin a Cafe 62000 arras
tel 06 00 00 00 06
permi B + voiture
caces 3 (passer en 2024)
permi C en cour

EXPAIRIENCE PROFESIONELLE

agent de quai depui janvier 2024 - Transport Bidule Express
dechargement des camion, tri des coli, transpalette electrique, chargement des tourné

maneuvre batiment de 2020 a 2023 che Maconnerie Trucmuche
preparation du mortier, aprovisionement du chantier, netoyage

manutentionaire 2 ans en interim (Interim Machin)

chaufeur livreur 03/2018 - 09/2019 Colis Zinzin
livraison che les client avec camionette 20m3, je fesai 80 point par jour

FORMASSION

niveau CAP macon (pas eu le diplome)
brevet des college

COMPETANSE
conduite de chariot elevateur
port de charge lourde
conduite camionette

QUALITER
serieu, courageu, jamais en retar, je sui motiver

LOISIR
peche, petanque, bricolage",
    metiers: &[
        ("agent de quai", D::Depuis(2024, 1)),
        ("maneuvre batiment", D::Mois(36)),
        ("manutentionaire", D::Mois(24)),
        ("chaufeur livreur", D::Mois(19)),
    ],
    pas_metiers: &["bidule", "trucmuche", "machin", "zinzin", "macon", "peche", "tri", "mortier"],
    permis: &["Permis B", "CACES 3"],
    diplome: None,
    diplomes: &["Brevet des collèges"],
    competences: &[
        "conduite de chariot elevateur",
        "port de charge lourde",
        "conduite camionette",
        "transpalette electrique",
    ],
    pas_competences: &["serieu", "courageu", "retar", "motiver", "peche", "petanque"],
};

/// Les vérifications d'un CV : (ce qui est vérifié, réussi ou non).
pub(super) fn verifier(a: &Attendu) -> Vec<(String, bool)> {
    let p = lire(a.cv);
    let mut v: Vec<(String, bool)> = Vec::new();
    let dans = |l: &[String], x: &str| {
        let k = normaliser(x);
        l.iter().any(|y| {
            let n = normaliser(y);
            n.contains(&k) || k.contains(&n)
        })
    };
    for (nom, d) in a.metiers {
        let k = cle_ligne(Liste::Metiers, nom);
        let trouve = p.metiers.iter().find(|m| cle_ligne(Liste::Metiers, &m.nom) == k);
        v.push((format!("métier « {nom} »"), trouve.is_some()));
        v.push((format!("durée de « {nom} » ({d:?})"), trouve.is_some_and(|m| duree_de(m) == *d)));
    }
    v.push((
        format!("aucun métier en trop ({:?})", p.metiers.iter().map(|m| &m.nom).collect::<Vec<_>>()),
        p.metiers.len() <= a.metiers.len(),
    ));
    for x in a.pas_metiers {
        v.push((
            format!("pas de métier « {x} »"),
            !p.metiers.iter().any(|m| normaliser(&m.nom).split(' ').any(|w| w == normaliser(x))),
        ));
    }
    let mut attendus: Vec<&str> = a.permis.to_vec();
    let mut trouves: Vec<&str> = p.permis.iter().map(String::as_str).collect();
    attendus.sort_unstable();
    trouves.sort_unstable();
    for x in a.permis {
        v.push((format!("permis « {x} »"), p.permis.iter().any(|y| y == x)));
    }
    v.push((format!("aucun permis en trop ({:?})", p.permis), trouves.iter().all(|x| attendus.contains(x))));
    v.push((
        format!("plus haut diplôme {:?} (trouvé {:?})", a.diplome, p.diplome_le_plus_haut()),
        p.diplome_le_plus_haut() == a.diplome,
    ));
    for x in a.diplomes {
        v.push((format!("diplôme « {x} » ({:?})", p.diplomes), dans(&p.diplomes, x)));
    }
    for x in a.competences {
        v.push((format!("compétence « {x} »"), dans(&p.competences, x)));
    }
    for x in a.pas_competences {
        v.push((
            format!("pas de compétence « {x} »"),
            !p.competences.iter().any(|c| normaliser(c).contains(&normaliser(x))),
        ));
    }
    v
}

/// Un couple offre et fiche, et les niveaux acceptés.
pub(super) struct Couple {
    pub nom: &'static str,
    pub fiche: fn() -> Parcours,
    pub titre: &'static str,
    pub texte: &'static str,
    pub acceptes: &'static [Proximite],
}

const COUPLES: &[Couple] = &[
    Couple {
        nom: "B1, chef de rayon et chef de projet",
        fiche: || {
            fiche(
                &[("Chef de rayon", D::Mois(84)), ("Employé libre-service", D::Mois(24))],
                &[
                    "gestion des stocks",
                    "passation des commandes",
                    "mise en rayon",
                    "encadrement d'une équipe de 4 personnes",
                ],
                &["Bac pro Commerce"],
                &["Permis B"],
            )
        },
        titre: "Chef de projet informatique H/F",
        texte: "Notre société de services numériques Info-Bidule recrute un chef de projet informatique pour son agence de Nantes.
Vous pilotez des projets de développement d'applications web pour nos clients grands comptes : recueil du besoin, rédaction des spécifications, planification, suivi du budget.
Vous animez une équipe de 6 développeurs en méthode Agile (Scrum).
Profil : Bac+5 en informatique (école d'ingénieur ou master), 3 ans d'expérience en gestion de projet informatique.
Maîtrise de Jira et bonnes connaissances en Java ou PHP. Anglais professionnel.
CDI, statut cadre, télétravail 2 jours par semaine.",
        acceptes: &[Proximite::Loin],
    },
    Couple {
        nom: "B2, le bon métier mais un diplôme hors de portée",
        fiche: || {
            fiche(
                &[("Aide-comptable", D::Mois(60))],
                &["saisie des factures", "rapprochements bancaires", "lettrage des comptes", "Excel"],
                &["Bac pro Comptabilité"],
                &["Permis B"],
            )
        },
        titre: "Aide-comptable H/F",
        texte: "Le cabinet d'expertise comptable Chiffres et Machin recherche un(e) aide-comptable pour renforcer son équipe.
Missions : saisie des factures d'achat et de vente, rapprochements bancaires, lettrage des comptes, préparation des déclarations de TVA.
Diplôme exigé : Bac+3 minimum en comptabilité (DCG ou licence). Les candidatures sans ce diplôme ne seront pas étudiées.
Expérience de 2 ans souhaitée en cabinet.
Bonne maîtrise d'Excel et d'un logiciel comptable.
CDI 35h, du lundi au vendredi, tickets restaurant.",
        acceptes: &[Proximite::Proche, Proximite::UnPeu],
    },
    Couple {
        nom: "B3, un métier jamais exercé mais des compétences communes",
        fiche: || {
            fiche(
                &[("Hôtesse de caisse", D::Mois(48))],
                &["encaissement", "accueil des clients", "tenue de caisse", "mise en rayon"],
                &["CAP Employé de commerce multi-spécialités"],
                &[],
            )
        },
        titre: "Employé(e) de station-service",
        texte: "La station-service Carbu-Truc située sur la rocade recherche un(e) employé(e) polyvalent(e).
Vous accueillez les clients, encaissez le carburant et les achats de la boutique.
Vous assurez la mise en rayon des produits et la propreté de la boutique et des pistes.
Vous tenez votre caisse et faites le comptage en fin de service.
Débutant accepté, formation assurée en interne.
Horaires en roulement, un week-end sur deux. CDD de 6 mois pouvant déboucher sur un CDI.",
        acceptes: &[Proximite::Proche, Proximite::UnPeu],
    },
    Couple {
        nom: "B4, cariste et conducteur de chariot élévateur",
        fiche: || {
            fiche(
                &[("Cariste", D::Mois(60)), ("Préparateur de commandes", D::Mois(18))],
                &[
                    "chargement et déchargement de camions",
                    "rangement en palettier",
                    "gestion des stocks",
                    "préparation de commandes",
                ],
                &["CAP Opérateur logistique"],
                &["Permis B", "CACES 1, 3, 5"],
            )
        },
        titre: "Conducteur de chariot élévateur H/F",
        texte: "Pour notre plateforme logistique de Saint-Machin, nous recrutons un conducteur de chariot élévateur.
Vous chargez et déchargez les camions à l'aide d'un chariot frontal.
Vous rangez les palettes en palettier et participez aux inventaires.
CACES R489 catégorie 3 obligatoire.
Expérience de 2 ans sur un poste similaire demandée.
Travail en 2x8, du lundi au vendredi. Prime de panier.",
        acceptes: &[Proximite::TresProche],
    },
    Couple {
        nom: "B5, un permis que la personne n'a pas",
        fiche: || {
            fiche(
                &[("Chauffeur livreur", D::Mois(30))],
                &[
                    "livraison en camionnette",
                    "chargement du véhicule",
                    "relation avec les clients",
                    "respect des tournées",
                ],
                &[],
                &["Permis B"],
            )
        },
        titre: "Chauffeur poids lourd - permis C",
        texte: "Les Transports Roule-Bidule recherchent un chauffeur poids lourd pour des tournées régionales en porteur 19 tonnes.
Vous livrez des magasins de bricolage, déchargez au hayon et au transpalette.
Permis C exigé, FIMO ou FCO à jour, carte conducteur.
Sans ces documents, votre candidature ne pourra pas être retenue.
Une première expérience en livraison est appréciée.
Départ 5h du dépôt, retour tous les soirs. CDI, 2100 euros brut plus frais de route.",
        acceptes: &[Proximite::UnPeu],
    },
    Couple {
        nom: "B6, une offre sans aucun rapport",
        fiche: || {
            fiche(
                &[("Aide-soignante", D::Depuis(2017, 9))],
                &["aide à la toilette", "aide aux repas", "prise des constantes", "transmissions écrites"],
                &["Diplôme d'Etat d'aide-soignant"],
                &["Permis B"],
            )
        },
        titre: "Soudeur TIG H/F",
        texte: "La chaudronnerie Métal-Machin recrute un soudeur TIG pour son atelier.
Vous réalisez des soudures sur des pièces en inox et en aluminium à partir de plans.
Vous contrôlez la qualité de vos soudures et effectuez le meulage et la finition.
Lecture de plans indispensable. Licences de soudure à jour appréciées.
CAP ou Bac pro Chaudronnerie, 2 ans d'expérience minimum.
Horaires de journée, 39h par semaine.",
        acceptes: &[Proximite::Loin],
    },
    Couple {
        nom: "B7, le bon métier, débutant accepté, avec le CACES",
        fiche: || {
            fiche(
                &[("Cariste", D::Mois(8)), ("Manutentionnaire", D::Mois(14))],
                &["conduite de chariot élévateur", "chargement de camions", "filmage de palettes"],
                &[],
                &["CACES 1, 3, 5"],
            )
        },
        titre: "Cariste H/F - débutant accepté",
        texte: "L'agence d'intérim Boulot-Truc recherche pour un de ses clients un cariste.
Vous chargez et déchargez les camions et alimentez les lignes de production en palettes.
CACES 3 en cours de validité obligatoire.
Débutant accepté, aucun diplôme demandé.
Mission de 3 mois renouvelable, horaires en 2x8.
Chaussures de sécurité fournies.",
        acceptes: &[Proximite::TresProche],
    },
    Couple {
        nom: "B8, 5 ans d'expérience demandés pour 6 mois faits",
        fiche: || {
            fiche(
                &[("Cuisinier", D::Mois(6)), ("Plongeur", D::Mois(12))],
                &[
                    "préparation des entrées froides",
                    "cuisson des viandes",
                    "nettoyage de la cuisine",
                    "respect des règles d'hygiène",
                ],
                &["CAP Cuisine"],
                &["HACCP"],
            )
        },
        titre: "Cuisinier confirmé H/F",
        texte: "Le restaurant traditionnel La Marmite de Trucmuche (80 couverts) recherche un cuisinier confirmé.
Vous travaillez en autonomie sur le poste chaud, gérez les commandes auprès des fournisseurs et encadrez un commis.
5 ans d'expérience minimum exigés sur un poste similaire.
CAP Cuisine demandé, connaissance des normes HACCP.
2 jours de repos consécutifs, coupures le week-end.
CDI, salaire selon expérience.",
        acceptes: &[Proximite::Proche, Proximite::UnPeu],
    },
    Couple {
        nom: "B9, conducteur de ligne et conducteur de bus",
        fiche: || {
            fiche(
                &[("Conducteur de ligne", D::Depuis(2019, 6)), ("Opérateur de production", D::Mois(30))],
                &["réglage des machines", "contrôle qualité", "changement de format", "maintenance de premier niveau"],
                &["Bac pro Pilote de ligne de production"],
                &["Permis B", "CACES 3"],
            )
        },
        titre: "Conducteur de bus H/F",
        texte: "Le réseau de transport urbain Bus-Machin recrute des conducteurs de bus.
Vous conduisez un bus sur les lignes régulières de l'agglomération, accueillez les voyageurs et vendez les titres de transport.
Vous veillez à la sécurité et au confort des passagers et respectez les horaires de passage.
Permis D et FIMO voyageurs exigés.
Sens du service et bonne présentation.
Horaires décalés, travail un week-end sur trois. CDI.",
        acceptes: &[Proximite::Loin],
    },
    Couple {
        nom: "B10, vendeuse et conseiller de vente",
        fiche: || {
            fiche(
                &[("Vendeuse", D::Mois(72))],
                &["conseil aux clients", "encaissement", "mise en rayon", "réassort"],
                &["CAP Vente"],
                &["Permis B"],
            )
        },
        titre: "Conseiller de vente H/F",
        texte: "La boutique de prêt-à-porter Fringues et Bidule recrute un conseiller de vente.
Vous accueillez et conseillez les clients, tenez la caisse et participez à la mise en rayon et au réassort.
Vous veillez à la bonne tenue du magasin et des cabines.
Une expérience d'un an en vente est demandée.
CAP ou BEP Vente apprécié.
CDI 35h, travail le samedi.",
        acceptes: &[Proximite::TresProche],
    },
    Couple {
        nom: "B11, chauffeur poids lourd et conducteur routier",
        fiche: || {
            fiche(
                &[("Chauffeur poids lourd", D::Depuis(2015, 4))],
                &[
                    "conduite de semi-remorque",
                    "livraisons nationales",
                    "sanglage et arrimage",
                    "utilisation du chronotachygraphe",
                ],
                &["CAP Conducteur routier marchandises"],
                &["Permis B", "Permis C", "Permis CE", "FIMO"],
            )
        },
        titre: "Conducteur routier national H/F",
        texte: "Les Transports Kilomètre-Truc recherchent un conducteur routier pour du transport national en semi-remorque bâchée.
Vous assurez les livraisons chez nos clients industriels, contrôlez le chargement et l'arrimage.
Vous respectez la réglementation sociale européenne (temps de conduite et de repos).
Permis CE, FIMO ou FCO et carte conducteur à jour obligatoires.
3 ans d'expérience en national demandés.
Départ le lundi, retour le vendredi. Découchés payés.",
        acceptes: &[Proximite::TresProche],
    },
    Couple {
        nom: "B12, le CACES 5 demandé, le CACES 3 seulement",
        fiche: || {
            fiche(
                &[("Cariste", D::Rien), ("Agent d'entretien", D::Mois(24))],
                &["conduite de chariot frontal", "chargement et déchargement de camions", "gestion des stocks"],
                &[],
                &["Permis B", "CACES 3"],
            )
        },
        titre: "Cariste chariot à mât rétractable (CACES 5)",
        texte: "Notre entrepôt Stock-Machin recherche un cariste pour le stockage en grande hauteur.
Vous rangez et sortez les palettes en hauteur (jusqu'à 11 mètres) avec un chariot à mât rétractable.
Vous utilisez le terminal radio pour enregistrer les mouvements de stock.
CACES R489 catégorie 5 obligatoire, en cours de validité.
Une expérience en grande hauteur est un plus.
Poste en équipe de nuit, majoration de 20 %.",
        acceptes: &[Proximite::Proche, Proximite::UnPeu],
    },
];

/// Le bilan d'une série : les CV entièrement justes, les vérifications réussies et leur nombre, les couples au bon
/// niveau, et le détail de ce qui est raté.
pub(super) fn bilan(cvs: &[&Attendu], couples_a_voir: &[Couple]) -> (usize, usize, usize, usize, String) {
    let mut rapport = String::new();
    let (mut cv_reussis, mut ok, mut total) = (0, 0, 0);
    for a in cvs {
        let v = verifier(a);
        let bons = v.iter().filter(|x| x.1).count();
        ok += bons;
        total += v.len();
        if bons == v.len() {
            cv_reussis += 1;
        }
        rapport.push_str(&format!("\n{} : {bons} sur {}", a.nom, v.len()));
        for (quoi, _) in v.iter().filter(|x| !x.1) {
            rapport.push_str(&format!("\n    RATE {quoi}"));
        }
    }
    let mut couples = 0;
    for c in couples_a_voir {
        let r = rapprocher(&(c.fiche)(), c.titre, c.texte);
        if c.acceptes.contains(&r.niveau) {
            couples += 1;
        } else {
            rapport.push_str(&format!("\n{} : RATE {:?} (attendu {:?}) « {} »", c.nom, r.niveau, c.acceptes, r.raison));
        }
    }
    (cv_reussis, ok, total, couples, rapport)
}

/// La mesure. Avant toute correction, avec les règles écrites sur la seule première moitié : 1 CV sur 6 entièrement
/// juste, 116 vérifications sur 152 (76 %), 9 couples sur 12. Ce qui manquait alors : les lignes de missions sans
/// puce, deux postes sur une même ligne, les colonnes mélangées, les fautes de frappe (« permi », « depui »,
/// « chaufeur »), « FIMO ou FCO », les verbes d'une offre face aux noms d'une fiche (« encaissez », « encaissement »),
/// et deux métiers de familles différentes qui portent le même premier mot. Après correction, tout passe : ce test
/// garde ce qui a été gagné, il ne mesure plus rien (voir la troisième série, jamais vue elle non plus).
#[test]
fn seconde_moitie_apres_correction() {
    let (cv, ok, total, couples, rapport) = bilan(&[&A1, &A2, &A3, &A4, &A5, &A6], COUPLES);
    assert_eq!((cv, ok, total, couples), (6, 152, 152, COUPLES.len()), "{rapport}");
}
