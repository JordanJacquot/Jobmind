use super::*;

/// Les 3 vraies offres France Travail (texte tel que Jobmind l'enregistre, relevé le 20/11/2026).
const BRUT1: &str = include_str!("exemples/brut1.txt");
const BRUT2: &str = include_str!("exemples/brut2.txt");
const BRUT3: &str = include_str!("exemples/brut3.txt");
const TITRE1: &str = "Agent / Agente d'entretien des bâtiments et espaces collectifs (H/F)";
const TITRE2: &str = "Agent de Production (h/f) (H/F)";
const TITRE3: &str = "Agent / Agente d'état des lieux (H/F)";

/// Une offre France Travail inventée, avec ce que les vraies n'avaient pas : « Formation », « Langues », des puces
/// « Exigé » et « Souhaité », un salaire mensuel, un CDI, des heures écrites.
const FT_INVENTEE: &str = "Offre n° 999TEST

Cariste (H/F)

METZ (57)Localiser avec Mappy

Actualisée le 3 novembre 2026

Entrepôt de la grande distribution, nous renforçons notre équipe.

Vos missions :
- Charger et décharger les camions
- Ranger les palettes en hauteur

Horaires : 6h-13h30 ou 13h30-21h

Profil souhaité

Expérience 2 An(s)

Formation

- Bac ou équivalent Logistique Exigé

- CAP, BEP et équivalents Souhaité

Langues

- Anglais Souhaité

Compétences

- Conduire un chariot élévateur Exigé

- Gérer les stocks

Savoir-être professionnels

- Autonomie

Permis

- B - Véhicule léger Exigé

- C - Poids lourd Souhaité

Critères de l’offre

Intitulé métier Cariste (N1101)

Lieu de travail 57 - METZ

Contrat CDI - Contrat travail

Conditions horaires

- 35H/semaine

- Travail en équipes

Salaire Mensuel de 1802 Euros sur 12 mois

- Prime de transport

L'employeur

LOGISTIQUE DE L'EST

Secteur d'activité Entreposage et stockage non frigorifique

Effectif 50 à 99 salariés

Une entreprise familiale.

Découvrir l’employeur
";

/// Une offre d'un autre site (forme Indeed ou Welcome to the Jungle), sans la structure France Travail.
const AUTRE_SITE: &str = "Nous sommes une boulangerie artisanale installée à Nancy depuis 1990.

Vos missions :
- Accueillir les clients
- Tenir la caisse

Votre profil :
- Vous êtes souriant et ponctuel
- Une expérience en vente est un plus

Salaire : 11,88 € brut de l'heure
Horaires : du mardi au samedi, de 7 h à 13 h
Avantages :
- Tickets restaurant
- Mutuelle";

const SANS_RUBRIQUE: &str = "Nous recherchons un vendeur pour notre magasin de Metz. Vous accueillez les clients, \
les conseillez et tenez la caisse.

Poste en CDI, 35 heures, débutant accepté.";

/// Tout le texte d'une fiche, champ après champ.
fn tout(f: &Fiche) -> String {
    let mut s: Vec<String> = vec![
        f.intro.clone(),
        f.lieu.clone(),
        f.poste_texte.clone(),
        f.experience.clone(),
        f.qualification.clone(),
        f.demande_texte.clone(),
        f.salaire.clone(),
        f.temps.clone(),
        f.contrat.clone(),
        f.conditions_texte.clone(),
        f.entreprise.clone(),
        f.secteur.clone(),
        f.effectif.clone(),
        f.entreprise_texte.clone(),
    ];
    for l in [&f.missions, &f.obligatoire, &f.un_plus, &f.savoir_faire, &f.qualites, &f.horaires, &f.avantages] {
        s.extend(l.iter().cloned());
    }
    s.join("\n")
}

fn v(l: &[&str]) -> Vec<String> {
    l.iter().map(|x| x.to_string()).collect()
}

const PARASITES: [&str; 9] = [
    "Localiser avec Mappy",
    "Offre n°",
    "Actualisée le",
    "Voir toutes les offres",
    "Découvrir l’employeur",
    "Intitulé métier",
    "Public cible",
    "Si ce poste vous intéresse",
    "Critères de l",
];

#[test]
fn vraie_offre_1_parc_animalier() {
    let f = ranger(TITRE1, BRUT1);
    assert!(f.intro.starts_with("Le Parc Animalier de Sainte Croix, lieu unique"));
    assert!(
        f.intro.ends_with(
            "\n\nAu sein du Pôle Technique du Parc Animalier de Sainte Croix et d'une équipe de 5 personnes,"
        )
    );
    assert_eq!(f.missions.len(), 5);
    assert_eq!(f.missions[0], "Garantir, le bon fonctionnement général de toutes les installations du site");
    assert!(f.missions[4].starts_with("Aider à l'organisation"));
    assert_eq!(f.lieu, "Rhodes (57)");
    assert_eq!(f.poste_texte, "");
    assert_eq!(f.obligatoire, v(&["Permis B, voiture"]));
    assert_eq!(f.un_plus, v(&["Habilitation électrique fortement recommandée"]));
    assert_eq!(f.experience, "6 mois sur un poste équivalent");
    assert_eq!(f.qualification, "Ouvrier qualifié (P1, P2)");
    assert_eq!(f.savoir_faire.len(), 15);
    assert_eq!(f.savoir_faire[0], "Compétences techniques variées (plomberie, électricité, menuiserie, soudure.)");
    assert_eq!(f.savoir_faire[1], "Constater une panne sur une installation (éclairage, chauffage, sanitaires)");
    assert_eq!(f.savoir_faire[14], "Règles de sécurité");
    assert_eq!(
        f.qualites,
        v(&[
            "Résolution de problème",
            "Organisation et rigueur dans l'exécution des tâches confiées",
            "Dynamisme et réactivité",
            "Écoute et respect des différents interlocuteurs",
            "Travail en équipe",
        ])
    );
    assert_eq!(f.demande_texte, "");
    assert_eq!(f.salaire, "Selon profil");
    assert_eq!(f.temps, "35 h par semaine");
    assert_eq!(f.horaires, v(&["Travail en journée", "Travail en astreinte", "Travail les week-ends et jours fériés"]));
    assert!(f.plages.is_empty());
    assert_eq!(f.contrat, "CDD de 3 mois");
    assert_eq!(f.avantages, v(&["CSE", "Intéressement / participation", "Titres restaurant / Prime de panier"]));
    assert_eq!(f.conditions_texte, "");
    assert_eq!(f.entreprise, "PARC ANIMALIER DE SAINTE CROIX");
    assert_eq!(f.secteur, "Gestion des jardins botaniques et zoologiques et des réserves naturelles");
    assert_eq!(f.effectif, "");
    assert!(f.entreprise_texte.starts_with("Au cSur de 120 hectares"));
    assert!(!f.entreprise_texte.contains("https://"));
    assert!(f.entreprise_texte.ends_with("dans nos lodges nature écolabellisés."));
}

#[test]
fn vraie_offre_2_interim() {
    let f = ranger(TITRE2, BRUT2);
    assert_eq!(
        f.intro,
        "Rejoignez une entreprise reconnue pour son savoir-faire dans le travail du bois !\n\nAdecco recherche pour \
         l'un de ses clients, un agent de production H/F.\n\nAu sein de l'atelier de production, vous jouez un rôle \
         essentiel dans le processus de fabrication :"
    );
    assert_eq!(
        f.missions,
        v(&[
            "Accrochage ou décrochage de pièces sur la ligne de peinture",
            "Disposer les menuiseries sur palettes et mettre en place les protections avant expedition",
            "Veiller au bon déroulement des opérations de découpe selon les longueurs définies",
            "Contrôler la qualité des pièces produites",
            "Maintenir votre poste de travail propre et sécurisé",
            "Respecter les consignes de sécurité et les procédures de production",
        ])
    );
    assert_eq!(f.lieu, "Waldhambach (67)");
    assert_eq!(f.poste_texte, "");
    assert!(f.obligatoire.is_empty());
    assert_eq!(
        f.un_plus,
        v(&[
            "Une première expérience en industrie ou dans le secteur du bois est un plus, mais les débutants motivés \
             sont les bienvenus"
        ])
    );
    assert_eq!(f.experience, "6 mois");
    assert_eq!(f.qualification, "Employé qualifié");
    assert_eq!(
        f.savoir_faire,
        v(&[
            "Conditionner des produits",
            "Approvisionner une machine en matière première ou en produit",
            "Inspecter visuellement les produits finis pour détecter les défauts",
        ])
    );
    assert_eq!(
        f.qualites,
        v(&[
            "Vous êtes manuel(le), dynamique et aimez le travail en atelier",
            "Vous appréciez le travail d'équipe et êtes rigoureux",
        ])
    );
    assert_eq!(f.salaire, "12,31 € brut de l'heure");
    assert_eq!(f.temps, "35 h par semaine");
    assert_eq!(f.horaires, v(&["Travail en 2x8", "Travail en journée"]));
    assert!(f.plages.is_empty());
    assert_eq!(f.contrat, "Intérim de 6 mois");
    assert!(f.avantages.is_empty());
    assert_eq!(f.entreprise, "ADECCO FRANCE");
    assert_eq!(f.secteur, "Activités des agences de travail temporaire");
    assert_eq!(f.effectif, "");
    assert_eq!(f.entreprise_texte, "");
}

#[test]
fn vraie_offre_3_independant() {
    let f = ranger(TITRE3, BRUT3);
    assert_eq!(
        f.intro,
        "Rejoignez notre réseau d'experts en états des lieux - Devenez votre propre patron !\n\nVous souhaitez allier \
         indépendance et collaboration avec les acteurs majeurs de l'immobilier ?\nNous recrutons des partenaires \
         indépendants pour réaliser des états des lieux d'entrée et de sortie auprès de nos clients (agences \
         immobilières, administrateurs de biens, bailleurs sociaux)."
    );
    assert!(f.missions.is_empty());
    assert_eq!(f.lieu, "Sarrebourg (57)");
    assert_eq!(
        f.poste_texte,
        "Envie de donner un nouveau tournant à votre carrière ?\n\nRejoignez-nous dès maintenant pour une activité \
         valorisante, dynamique, et en pleine expansion !"
    );
    assert!(f.obligatoire.is_empty());
    assert!(f.un_plus.is_empty());
    assert_eq!(f.experience, "Débutant accepté");
    assert_eq!(f.qualification, "Employé qualifié");
    assert_eq!(f.savoir_faire.len(), 6);
    assert!(f.savoir_faire[0].starts_with("Connaissance minimale des termes techniques"));
    assert_eq!(f.savoir_faire[5], "Droits et obligations des locataires");
    assert_eq!(
        f.qualites,
        v(&[
            "Minutie, rigueur et sens de l'observation : chaque détail compte",
            "Excellentes compétences relationnelles et pédagogiques : vous serez l'interlocuteur clé entre locataires \
             et bailleurs",
            "Faire preuve de rigueur et de précision",
            "Faire preuve de contrôle de soi",
            "Etre ouvert aux changements",
        ])
    );
    assert_eq!(f.salaire, "");
    assert_eq!(f.temps, "");
    assert!(f.horaires.is_empty());
    assert_eq!(f.contrat, "Indépendant (non salarié)");
    assert_eq!(f.avantages.len(), 3);
    assert!(f.avantages[0].starts_with("Formation complète de 3 jours"));
    assert_eq!(f.avantages[2], "Autonomie : développez votre chiffre d'affaires tout en gérant votre emploi du temps");
    assert_eq!(f.conditions_texte, "Déplacements : quotidiens, zone départementale");
    assert_eq!(f.entreprise, "CONSTATIMMO");
    assert_eq!(f.secteur, "Analyses, essais et inspections techniques");
    assert_eq!(f.effectif, "");
    assert_eq!(f.entreprise_texte, "");
}

#[test]
fn aucun_doublon_ni_parasite() {
    for (titre, texte) in [(TITRE1, BRUT1), (TITRE2, BRUT2), (TITRE3, BRUT3), ("Cariste (H/F)", FT_INVENTEE)] {
        let t = tout(&ranger(titre, texte));
        for p in PARASITES {
            assert!(!t.contains(p), "« {p} » est resté dans la fiche de « {titre} »");
        }
        assert!(!t.contains(titre), "le titre « {titre} » est répété");
        // aucune ligne en double d'un champ à l'autre
        let lignes: Vec<&str> = t.lines().filter(|l| !l.trim().is_empty()).collect();
        for (i, l) in lignes.iter().enumerate() {
            assert!(!lignes[i + 1..].contains(l), "« {l} » est en double dans « {titre} »");
        }
    }
    let f = ranger(TITRE3, BRUT3);
    assert!(!f.intro.contains("Minutie"));
    assert!(!f.poste_texte.contains("Minutie"));
    assert!(f.qualites.iter().any(|q| q.starts_with("Minutie, rigueur et sens de l'observation")));
    assert_eq!(tout(&f).matches("Débutant accepté").count(), 1);
    assert_eq!(f.experience, "Débutant accepté");
}

#[test]
fn aucune_perte_phrases_choisies() {
    let cas = [
        (TITRE1, BRUT1, "Habilitation électrique"),
        (TITRE1, BRUT1, "Organisation et rigueur dans l'exécution des tâches confiées"),
        (TITRE1, BRUT1, "1 500 animaux issus de 100 espèces"),
        (TITRE1, BRUT1, "Travail en astreinte"),
        (TITRE2, BRUT2, "Travail en 2x8"),
        (TITRE2, BRUT2, "les débutants motivés sont les bienvenus"),
        (TITRE2, BRUT2, "Au sein de l'atelier de production"),
        (TITRE3, BRUT3, "Accompagnement continu pour booster votre réussite"),
        (TITRE3, BRUT3, "Envie de donner un nouveau tournant"),
        (TITRE3, BRUT3, "Etre ouvert aux changements"),
        (TITRE3, BRUT3, "Connaissance minimale des termes techniques"),
    ];
    for (titre, texte, phrase) in cas {
        assert!(tout(&ranger(titre, texte)).contains(phrase), "« {phrase} » est perdue");
    }
}

/// Chaque ligne du texte libre de l'employeur (entre l'en-tête et « Profil souhaité ») se retrouve dans la fiche, sauf
/// les sous-titres et « Si ce poste vous intéresse ».
#[test]
fn aucune_perte_texte_libre() {
    for (titre, texte) in [(TITRE1, BRUT1), (TITRE2, BRUT2), (TITRE3, BRUT3)] {
        let t = normaliser(&tout(&ranger(titre, texte)));
        let lignes: Vec<&str> = texte.lines().map(str::trim).collect();
        let debut = lignes.iter().position(|l| l.starts_with("Actualisée le")).unwrap() + 1;
        let fin = lignes.iter().position(|l| *l == "Profil souhaité").unwrap();
        for l in &lignes[debut..fin] {
            if l.is_empty() || titre_libre(l).is_some() || parasite_libre(l) {
                continue;
            }
            let coeur = normaliser(&propre(puce(l).1).replace(" est fortement ", " fortement "));
            assert!(t.contains(&coeur), "« {l} » est perdue dans « {titre} »");
        }
    }
}

#[test]
fn france_travail_inventee() {
    let f = ranger("Cariste (H/F)", FT_INVENTEE);
    assert_eq!(f.intro, "Entrepôt de la grande distribution, nous renforçons notre équipe.");
    assert_eq!(f.missions, v(&["Charger et décharger les camions", "Ranger les palettes en hauteur"]));
    assert_eq!(f.lieu, "Metz (57)");
    assert_eq!(f.experience, "2 ans");
    assert_eq!(
        f.obligatoire,
        v(&["Bac ou équivalent Logistique", "Conduire un chariot élévateur", "Permis B, voiture"])
    );
    assert_eq!(f.un_plus, v(&["CAP, BEP et équivalents", "Anglais", "Permis C, poids lourd"]));
    assert_eq!(f.savoir_faire, v(&["Gérer les stocks"]));
    assert_eq!(f.qualites, v(&["Autonomie"]));
    assert_eq!(f.salaire, "1 802 € brut par mois");
    assert_eq!(f.temps, "35 h par semaine");
    assert_eq!(f.horaires, v(&["6h-13h30 ou 13h30-21h", "Travail en équipes"]));
    assert_eq!(f.plages, vec![Plage { debut: 360, fin: 810 }, Plage { debut: 810, fin: 1260 }]);
    assert_eq!(f.contrat, "CDI");
    assert_eq!(f.avantages, v(&["Prime de transport"]));
    assert_eq!(f.entreprise, "LOGISTIQUE DE L'EST");
    assert_eq!(f.secteur, "Entreposage et stockage non frigorifique");
    assert_eq!(f.effectif, "50 à 99 salariés");
    assert_eq!(f.entreprise_texte, "Une entreprise familiale.");
    assert_eq!(f.poste_texte, "");
    assert_eq!(f.demande_texte, "");
}

#[test]
fn plages_lues() {
    let p = |a: u16, b: u16| Plage { debut: a, fin: b };
    assert_eq!(plages_de("Horaires : 6h-13h30 ou 13h30-21h"), vec![p(360, 810), p(810, 1260)]);
    assert_eq!(plages_de("de 8 h à 12 h et de 14 h à 17 h"), vec![p(480, 720), p(840, 1020)]);
    assert_eq!(plages_de("08:00 - 12:00 / 13:30 - 17:00"), vec![p(480, 720), p(810, 1020)]);
    assert_eq!(plages_de("de 9 heures à 17 heures 30"), vec![p(540, 1050)]);
    // aucune heure
    for t in ["35H/semaine", "Travail en 2x8", "Travail en journée", "Poste à pourvoir le 12/01", "3 jours sur 7"] {
        assert!(plages_de(t).is_empty(), "{t}");
    }
    // une plage de nuit est laissée de côté
    assert!(plages_de("22h-6h").is_empty());
    assert!(ranger(TITRE1, BRUT1).plages.is_empty());
}

#[test]
fn valeurs_remises_en_forme() {
    assert_eq!(salaire_propre("Mensuel de 1802 Euros sur 12 mois"), "1 802 € brut par mois");
    assert_eq!(
        salaire_propre("Mensuel de 1802.00 Euros à 2100.00 Euros sur 13 mois"),
        "1 802 à 2 100 € brut par mois, sur 13 mois"
    );
    assert_eq!(salaire_propre("Annuel de 30000 Euros sur 12 mois"), "30 000 € brut par an");
    assert_eq!(salaire_propre("Horaire de 12.31 Euros"), "12,31 € brut de l'heure");
    assert_eq!(salaire_propre("Horaire de 12.5 Euros"), "12,50 € brut de l'heure");
    assert_eq!(salaire_propre("selon profils et expériences"), "Selon profil");
    assert_eq!(salaire_propre("N.A"), "");
    assert_eq!(salaire_propre("A négocier"), "A négocier");
    assert_eq!(contrat_propre("CDD - 3 Mois - Contrat travail"), "CDD de 3 mois");
    assert_eq!(contrat_propre("Intérim - 6 Mois - Contrat travail"), "Intérim de 6 mois");
    assert_eq!(contrat_propre("CDI - Contrat travail"), "CDI");
    assert_eq!(contrat_propre("Profession libérale - Emploi non salarié"), "Indépendant (non salarié)");
    assert_eq!(contrat_propre("CDD - 12 Jour(s) - Contrat apprentissage"), "CDD de 12 jours, contrat apprentissage");
    assert_eq!(experience_propre("1 An(s)"), "1 an");
    assert_eq!(lieu_propre("57 - SAINT-JEAN-DE-ROHRBACH"), "Saint-Jean-de-Rohrbach (57)");
    assert_eq!(lieu_propre("Waldhambach (67)"), "Waldhambach (67)");
    assert_eq!(temps_propre("35H30/semaine").as_deref(), Some("35 h 30 par semaine"));
    assert_eq!(permis_propre("BE - Véhicule léger + remorque"), "Permis BE, voiture avec remorque");
}

#[test]
fn autre_site_par_les_rubriques() {
    let f = ranger("Vendeur en boulangerie H/F", AUTRE_SITE);
    assert_eq!(f.intro, "Nous sommes une boulangerie artisanale installée à Nancy depuis 1990.");
    assert_eq!(f.missions, v(&["Accueillir les clients", "Tenir la caisse"]));
    assert_eq!(f.qualites, v(&["Vous êtes souriant et ponctuel"]));
    assert_eq!(f.un_plus, v(&["Une expérience en vente est un plus"]));
    assert_eq!(f.salaire, "11,88 € brut de l'heure");
    assert_eq!(f.horaires, v(&["du mardi au samedi, de 7 h à 13 h"]));
    assert_eq!(f.plages, vec![Plage { debut: 420, fin: 780 }]);
    assert_eq!(f.avantages, v(&["Tickets restaurant", "Mutuelle"]));
    assert_eq!(f.entreprise, "");
}

/// La forme France Travail sans en-tête de page (celle des tests de `rubriques`) passe aussi par les rubriques.
#[test]
fn rubriques_forme_france_travail() {
    let texte = "Préparateur de commandes H/F
Vous préparez les commandes de nos clients.

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
Permis
B - Véhicule léger Exigé
Secteur d'activité : Entreposage et stockage non frigorifique";
    let f = ranger("Préparateur de commandes H/F", texte);
    assert_eq!(f.intro, "Vous préparez les commandes de nos clients.");
    assert_eq!(f.contrat, "CDI");
    assert_eq!(f.temps, "35 h par semaine");
    assert_eq!(f.horaires, v(&["Travail en journée"]));
    assert_eq!(f.salaire, "1 900 € brut par mois");
    assert_eq!(f.experience, "Débutant accepté");
    assert_eq!(f.obligatoire, v(&["Permis B, voiture"]));
    assert_eq!(f.secteur, "Entreposage et stockage non frigorifique");
}

#[test]
fn texte_vide_et_sans_rubrique() {
    assert_eq!(ranger("Vendeur", ""), Fiche::default());
    assert_eq!(ranger("Vendeur", "  \n\n  "), Fiche::default());
    let f = ranger("Vendeur H/F", SANS_RUBRIQUE);
    assert_eq!(f.intro, SANS_RUBRIQUE);
    assert_eq!(Fiche { intro: String::new(), ..f }, Fiche::default());
}
