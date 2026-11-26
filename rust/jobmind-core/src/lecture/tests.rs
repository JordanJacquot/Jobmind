use super::*;

/// Le code d'une lecture, comme dans les jeux de phrases : diplôme et expérience (« non_dit » : rien).
fn codes(l: &LectureOffre) -> (String, String) {
    (
        l.diplome.as_ref().map_or("non_dit".into(), |d| d.valeur.code().to_string()),
        l.experience.as_ref().map_or("non_dit".into(), |e| e.valeur.code()),
    )
}

/// Le jeu écrit avec les règles : de vraies tournures d'annonces, et leurs pièges.
const JEU: &[(&str, &str, &str)] = &[
    ("Aucun diplôme requis. Débutants acceptés.", "aucun", "debutant"),
    ("Poste accessible sans diplôme, formation assurée en interne.", "aucun", "debutant"),
    ("Pas de diplôme exigé, vous serez formé au poste.", "aucun", "debutant"),
    ("Diplôme : non exigé. Expérience : débutant accepté.", "aucun", "debutant"),
    ("Débutants acceptés, formation assurée.", "aucun", "debutant"),
    ("Bac+2 souhaité mais débutants acceptés.", "bac2", "debutant"),
    ("Diplôme non exigé.", "aucun", "non_dit"),
    ("2 ans d'expérience minimum sur un poste similaire.", "non_dit", "duree:24"),
    ("Première expérience appréciée.", "non_dit", "souhaitee"),
    ("Expérience exigée : non", "non_dit", "debutant"),
    ("Expérience exigée : oui", "non_dit", "demandee"),
    ("EXPERIENCE EXIGEE : NON", "non_dit", "debutant"),
    ("DEBUTANTS ACCEPTES - SANS DIPLOME", "aucun", "debutant"),
    ("diplome non exige, debutant accepte", "aucun", "debutant"),
    ("Titulaire d'un CAP/BEP en électricité, vous avez 3 ans d'expérience.", "cap_bep", "duree:36"),
    ("Vous êtes titulaire d'un BTS MCO et justifiez d'une première expérience en magasin.", "bac2", "souhaitee"),
    ("De formation Bac+5 (école d'ingénieur ou master), vous avez au moins 5 ans d'expérience.", "bac5", "duree:60"),
    ("Licence professionnelle exigée.", "bac3", "non_dit"),
    ("Niveau Bac minimum, une expérience dans la vente est un plus.", "bac", "souhaitee"),
    ("Bac pro logistique apprécié. Débutant(e) accepté(e).", "bac", "debutant"),
    ("CACES 1, 3 et 5 obligatoires. Expérience de 6 mois en entrepôt.", "non_dit", "duree:6"),
    ("Vous avez une expérience de 1 an et demi en restauration.", "non_dit", "duree:18"),
    ("Expérience significative de 2 à 3 ans dans le BTP.", "non_dit", "duree:24"),
    ("Profil expérimenté, débutants s'abstenir.", "non_dit", "demandee"),
    ("Pas de débutant.", "non_dit", "demandee"),
    ("Cariste confirmé H/F", "non_dit", "demandee"),
    ("Aide à domicile H/F, débutante acceptée, permis B indispensable.", "non_dit", "debutant"),
    ("CDD de 6 mois renouvelable. Poste à pourvoir rapidement.", "non_dit", "non_dit"),
    ("Contrat de 12 mois, période d'essai de 2 mois.", "non_dit", "non_dit"),
    ("Vous devez avoir 18 ans minimum.", "non_dit", "non_dit"),
    ("Entreprise créée il y a 20 ans, nous recrutons un vendeur.", "non_dit", "non_dit"),
    ("Salaire selon expérience.", "non_dit", "non_dit"),
    ("Nettoyage des bacs à sable et des bacs de rétention.", "non_dit", "non_dit"),
    ("Cap sur l'avenir avec notre entreprise !", "non_dit", "non_dit"),
    ("Le restaurant dispose d'une licence IV.", "non_dit", "non_dit"),
    ("Vous travaillerez en équipe dans une ambiance conviviale.", "non_dit", "non_dit"),
    ("Le bac n'est pas obligatoire.", "aucun", "non_dit"),
    ("Pas besoin d'avoir de diplôme ni d'expérience.", "aucun", "debutant"),
    ("Aucune expérience n'est requise, nous vous formons.", "aucun", "debutant"),
    ("Sans expérience ? Pas de problème : on vous forme !", "aucun", "debutant"),
    ("Poste ouvert à tous, avec ou sans diplôme.", "aucun", "non_dit"),
    ("Accessible aux débutants.", "non_dit", "debutant"),
    ("Jeune diplômé accepté.", "non_dit", "debutant"),
    ("Une première expérience serait un plus.", "non_dit", "souhaitee"),
    ("Expérience souhaitée dans la grande distribution.", "non_dit", "souhaitee"),
    ("Expérience indispensable en conduite d'engins.", "non_dit", "demandee"),
    ("Vous justifiez d'une solide expérience en comptabilité.", "non_dit", "demandee"),
    ("Exp. 2 ans mini en préparation de commandes.", "non_dit", "duree:24"),
    ("Experience : 1 an", "non_dit", "duree:12"),
    ("Experience de deux ans exigee", "non_dit", "duree:24"),
    ("BTS ou DUT en informatique, 5 ans d'expérience en développement.", "bac2", "duree:60"),
    ("Du CAP au Bac+2 selon le poste.", "cap_bep", "non_dit"),
    ("CAP Petite enfance exigé.", "cap_bep", "non_dit"),
    // (le mot « cap », pas le diplôme : vrai cas d'une offre de directeur d'agence, 19/11/2026)
    (
        "Vous sentez qu'il est temps de passer un cap, celui de devenir acteur de votre propre réussite ?",
        "non_dit",
        "non_dit",
    ),
    ("Garder le cap dans un environnement exigeant, avec un profil autonome.", "non_dit", "non_dit"),
    ("Un nouveau cap pour votre carrière : profil commercial souhaité.", "non_dit", "non_dit"),
    ("Cap sur l'avenir avec notre équipe.", "non_dit", "non_dit"),
    ("Le cap de l'entreprise est clair : votre profil nous intéresse.", "non_dit", "non_dit"),
    ("Titulaire d'un CAP, vous aimez le contact client.", "cap_bep", "non_dit"),
    ("Vous avez un CAP cuisine et 2 ans d'expérience.", "cap_bep", "duree:24"),
    ("Diplôme d'ingénieur exigé, 10 ans d'expérience minimum.", "bac5", "duree:120"),
    ("Formation de niveau Bac+3 en ressources humaines.", "bac3", "non_dit"),
    ("Bac+2/3 en commerce, une première expérience est appréciée.", "bac2", "souhaitee"),
    ("Vous êtes débutant ou confirmé ? Rejoignez-nous.", "non_dit", "debutant"),
    ("Expérience non exigée, mais une première expérience en vente serait appréciée.", "non_dit", "debutant"),
    ("Aucune qualification requise. Travail en extérieur.", "aucun", "non_dit"),
    ("Débutants bienvenus ! Formation au poste assurée.", "aucun", "debutant"),
    ("Le permis C est exigé.", "non_dit", "non_dit"),
    ("Agent de quai (H/F) - CDI - Ennery", "non_dit", "non_dit"),
    ("Nous offrons une expérience enrichissante au sein d'une équipe soudée.", "non_dit", "non_dit"),
    ("Bac + 5 en finance, minimum 3 ans d'expérience en audit.", "bac5", "duree:36"),
    ("Expérience de 6 mois minimum souhaitée en caisse.", "non_dit", "duree:6"),
    ("Experience exigee de 3 ans", "non_dit", "duree:36"),
    ("Mission d'intérim de 3 mois, expérience en manutention appréciée.", "non_dit", "souhaitee"),
    ("Vous avez au moins trois ans d'expérience en tant que chef d'équipe.", "non_dit", "duree:36"),
    ("Baccalauréat obligatoire pour ce poste.", "bac", "non_dit"),
    // La rubrique « Profil souhaité » des offres France Travail (une ligne par élément)
    ("Profil souhaité\nExpérience\nDébutant accepté", "non_dit", "debutant"),
    ("Expérience\n2 An(s)\nExigé", "non_dit", "duree:24"),
    ("Expérience\n18 Mois\nExigé", "non_dit", "duree:18"),
    // (une durée seulement souhaitée n'est pas exigée)
    ("Expérience\n6 Mois\nSouhaité", "non_dit", "souhaitee"),
    ("Expérience\n- 3 An(s) Exigé", "non_dit", "duree:36"),
    ("Expérience exigée de 2 An(s)", "non_dit", "duree:24"),
    ("Expérience souhaitée de 1 An(s)", "non_dit", "souhaitee"),
    ("Expérience\n2 An(s)\nCuisine traditionnelle\nExigé", "non_dit", "duree:24"),
    ("Expérience\n1 An(s) Exigé\nFormation\nBac ou équivalent Souhaité", "bac", "duree:12"),
    ("Formation\nCAP, BEP et équivalents Cuisine Exigé", "cap_bep", "non_dit"),
    ("Formation\nBac+2 ou équivalents Commerce\nExigé", "bac2", "non_dit"),
    ("Formation\nBac+5 et plus ou équivalents", "bac5", "non_dit"),
    ("Formation\nAucune formation scolaire", "aucun", "non_dit"),
    ("Formation\nSans diplôme", "aucun", "non_dit"),
    ("Formation\nCAP\nExigé", "cap_bep", "non_dit"),
    ("Formation\nBac+3, Bac+4 ou équivalents Informatique\nSouhaité\nExpérience\nDébutant accepté", "bac3", "debutant"),
    // (deux formations : la plus basse exigée, la marque « Souhaité » n'est pas prise par la ligne d'avant)
    ("Formation\nBac ou équivalent\nBac+2 ou équivalents\nSouhaité", "bac", "non_dit"),
    ("Formation\nBac+2 ou équivalents Souhaité\nCAP, BEP et équivalents Exigé", "cap_bep", "non_dit"),
    ("Formation : Bac ou équivalent Exigé\nExpérience : 3 An(s) Exigé", "bac", "duree:36"),
    ("Formation :\nCAP, BEP et équivalents\nExigé", "cap_bep", "non_dit"),
    ("Expérience\nDébutant accepté\nFormation\nAucune formation scolaire", "aucun", "debutant"),
    // (la rubrique passe avant le texte libre)
    ("Une première expérience serait un plus.\nProfil souhaité\nExpérience\n2 An(s)\nExigé", "non_dit", "duree:24"),
    ("Titulaire d'un BTS, vous aimez la vente.\nFormation\nBac ou équivalent Exigé", "bac", "non_dit"),
    ("Débutants s'abstenir.\nExpérience\n2 An(s)\nSouhaité", "non_dit", "souhaitee"),
    // Les pièges
    ("Il est temps de passer un cap.\nFormation\nAssurée en interne par nos équipes.", "non_dit", "non_dit"),
    ("Formation\nFormation assurée en interne", "aucun", "debutant"),
    ("Formation\nBac à sable et jeux d'eau à entretenir.", "non_dit", "non_dit"),
    ("Nous offrons une expérience\nunique à nos clients.", "non_dit", "non_dit"),
    ("Expérience\nVous avez travaillé dans la vente.", "non_dit", "non_dit"),
    ("Permis\nB - Véhicule léger Exigé", "non_dit", "non_dit"),
    ("Permis B Exigé", "non_dit", "non_dit"),
    ("Qualification : Employé qualifié", "non_dit", "non_dit"),
    ("Qualification\nCadre\nExpérience\nDébutant accepté", "non_dit", "debutant"),
    ("Savoir-être professionnels\nAutonomie\nRigueur\nCompétences\nPréparer les commandes", "non_dit", "non_dit"),
    ("Type de contrat\nCDD - 6 Mois\nExpérience\nDébutant accepté", "non_dit", "debutant"),
    ("Langues\nAnglais Souhaité\nPermis\nC - Poids lourd Exigé", "non_dit", "non_dit"),
    // Un âge n'est pas une durée d'expérience
    ("Expérience auprès d'enfants de 3 à 6 ans souhaitée.", "non_dit", "souhaitee"),
    ("Vous encadrez des enfants de 0 à 3 ans en crèche.", "non_dit", "non_dit"),
    ("Animation auprès d'enfants (3-6 ans), expérience exigée.", "non_dit", "demandee"),
    ("Vous devez avoir 18 ans minimum, une expérience en restauration est exigée.", "non_dit", "demandee"),
    ("18 ans minimum, expérience en salle appréciée.", "non_dit", "souhaitee"),
    ("Contrat ouvert aux jeunes âgés de 16 à 25 ans, expérience en vente appréciée.", "non_dit", "souhaitee"),
    ("Être âgé de 18 ans révolus, expérience souhaitée.", "non_dit", "souhaitee"),
    ("Accueil de jeunes de moins de 26 ans, une expérience de 2 ans est demandée.", "non_dit", "duree:24"),
    ("Expérience de 3 à 5 ans auprès d'enfants de 6 à 12 ans.", "non_dit", "duree:36"),
    ("Minimum 3 ans d'expérience auprès de personnes âgées de 60 ans et plus.", "non_dit", "duree:36"),
    ("20 ans d'expérience dans le bâtiment.", "non_dit", "duree:240"),
    ("Niveau 3 et 2 ans d'expérience en atelier.", "non_dit", "duree:24"),
];

#[test]
fn le_jeu_des_regles() {
    assert!(JEU.len() >= 60, "{}", JEU.len());
    let mut fautes = Vec::new();
    for (phrase, d, e) in JEU {
        let l = lire("", phrase);
        let (ld, le) = codes(&l);
        if ld != *d || le != *e {
            fautes.push(format!("{phrase} : {ld} {le} (attendu {d} {e})"));
        }
    }
    assert!(fautes.is_empty(), "{} fautes :\n{}", fautes.len(), fautes.join("\n"));
}

/// Les jeux jamais vus pendant l'écriture des règles (écrits à part, après, sans voir les règles) : le taux de bonnes
/// réponses, et chaque faute. Le premier a servi ensuite à corriger les règles (81 % à sa première mesure) ; le second
/// n'a jamais servi à les corriger.
#[test]
fn le_jeu_jamais_vu() {
    mesurer("1", include_str!("../../tests/jeu_jamais_vu.tsv"));
}

#[test]
fn le_second_jeu_jamais_vu() {
    mesurer("2", include_str!("../../tests/jeu_jamais_vu_2.tsv"));
}

fn mesurer(nom: &str, jeu: &str) {
    let (mut bons_d, mut bons_e, mut bons, mut n) = (0, 0, 0, 0);
    let mut fautes = Vec::new();
    for ligne in jeu.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let c: Vec<&str> = ligne.split('\t').collect();
        assert_eq!(c.len(), 3, "{ligne}");
        let (ld, le) = codes(&lire("", c[0]));
        n += 1;
        bons_d += usize::from(ld == c[1]);
        bons_e += usize::from(le == c[2]);
        if ld == c[1] && le == c[2] {
            bons += 1;
        } else {
            fautes.push(format!("{} : {ld} {le} (attendu {} {})", c[0], c[1], c[2]));
        }
    }
    println!(
        "Jeu jamais vu {nom} : {bons}/{n} phrases justes ({} %), diplôme {bons_d}/{n}, expérience {bons_e}/{n}",
        bons * 100 / n
    );
    for f in &fautes {
        println!("  {f}");
    }
    assert!(n >= 60);
    assert!(bons * 100 / n >= 70, "trop de fautes : {}", fautes.join("\n"));
}

#[test]
fn les_preuves_sont_tirees_du_texte() {
    let texte = "Nous recherchons un préparateur de commandes.\nVous êtes titulaire d'un CAP/BEP.\n\
                 Débutant accepté, formation assurée par nos équipes. Permis B et CACES R489 cat 1, 3 et 5.";
    let l = lire("Préparateur de commandes (H/F)", texte);
    let d = l.diplome.unwrap();
    assert_eq!((d.valeur, d.confiance), (Diplome::CapBep, Confiance::Forte));
    assert_eq!(d.preuve, "Vous êtes titulaire d'un CAP/BEP");
    assert!(texte.contains(&d.preuve));
    let e = l.experience.unwrap();
    assert_eq!(e.valeur, Experience::Debutant);
    assert!(texte.contains(&e.preuve), "{}", e.preuve);
    assert!(e.preuve.starts_with("Débutant accepté"));
    assert_eq!(l.autres, ["Permis B", "CACES R489 1, 3, 5"]);
    // (une longue phrase : seulement le bout autour)
    let long = format!("{} vous êtes sans diplôme ? {}", "mot ".repeat(40), "suite ".repeat(40));
    let p = lire("", &long).diplome.unwrap().preuve;
    assert!(p.contains("sans diplôme") && p.split(' ').count() <= 20, "{p}");
    // Une tournure : confiance moyenne
    let l = lire("", "Débutants acceptés, formation assurée.");
    assert_eq!(l.diplome.unwrap().confiance, Confiance::Moyenne);
    assert!(!lire("", "Rien à voir.").classee());
    assert!(lire("", "Sans diplôme, débutant accepté.").classee());
}

/// Une offre France Travail entière : le texte libre, puis la rubrique « Profil souhaité » (exemple inventé, d'après
/// la forme des pages). La rubrique est la référence, ses preuves sont des bouts du texte, mot pour mot.
#[test]
fn le_profil_souhaite_de_france_travail() {
    let texte = "Le restaurant du Parc (exemple inventé) recherche un cuisinier ou une cuisinière.\n\
                 Vous préparez les entrées et les plats du jour. Une première expérience serait un plus.\n\
                 Formation assurée en interne sur nos recettes.\n\
                 \n\
                 Profil souhaité\n\
                 Expérience\n\
                 2 An(s)\n\
                 Exigé\n\
                 Formation\n\
                 CAP, BEP et équivalents Cuisine Exigé\n\
                 Savoir-être professionnels\n\
                 Autonomie\n\
                 Travail en équipe\n\
                 Compétences\n\
                 Réaliser les préparations de base\n\
                 Permis\n\
                 B - Véhicule léger Souhaité\n\
                 Qualification\n\
                 Employé qualifié";
    let l = lire("Cuisinier / Cuisinière (H/F)", texte);
    let d = l.diplome.unwrap();
    assert_eq!((d.valeur, d.confiance), (Diplome::CapBep, Confiance::Forte));
    assert_eq!(d.preuve, "CAP, BEP et équivalents Cuisine Exigé");
    let e = l.experience.unwrap();
    assert_eq!((e.valeur, e.confiance), (Experience::Duree(24), Confiance::Forte));
    assert_eq!(e.preuve, "2 An(s)\nExigé");
    for p in [&d.preuve, &e.preuve] {
        assert!(texte.contains(p.as_str()), "{p}");
        assert!(citation_presente(texte, p), "{p}");
    }
    assert_eq!(l.autres, ["Permis B"]);
    // Le même avec des puces, « : » et la forme courte de l'expérience
    let texte = "Magasinier H/F, CDD de 6 mois.\n\
                 Profil souhaité :\n\
                 - Expérience souhaitée de 1 An(s)\n\
                 - Formation : Bac ou équivalent Souhaité\n\
                 - Permis : B - Véhicule léger Exigé\n\
                 Permis B exigé pour les livraisons.";
    let l = lire("Magasinier (H/F)", texte);
    let (d, e) = (l.diplome.unwrap(), l.experience.unwrap());
    assert_eq!(
        (d.valeur, d.confiance, d.preuve.as_str()),
        (Diplome::Bac, Confiance::Forte, "Bac ou équivalent Souhaité")
    );
    assert_eq!(
        (e.valeur, e.confiance, e.preuve.as_str()),
        (Experience::Souhaitee, Confiance::Forte, "Expérience souhaitée de 1 An(s)")
    );
    for p in [&d.preuve, &e.preuve] {
        assert!(texte.contains(p.as_str()) && citation_presente(texte, p), "{p}");
    }
    assert_eq!(l.autres, ["Permis B"]);
    // Une valeur trop courte pour prouver seule : la preuve commence au titre de la rubrique
    let texte = "Formation\nBTS\nExpérience\nDébutant accepté";
    let l = lire("", texte);
    let d = l.diplome.unwrap();
    assert_eq!((d.valeur, d.preuve.as_str()), (Diplome::Bac2, "Formation\nBTS"));
    assert!(citation_presente(texte, &d.preuve));
    assert_eq!(l.experience.unwrap().preuve, "Débutant accepté");
    // Débutant accepté dans la rubrique, « débutants s'abstenir » dans le texte : la rubrique l'emporte
    let l = lire("", "Débutants s'abstenir.\nProfil souhaité\nExpérience\nDébutant accepté");
    assert_eq!(l.experience.unwrap().valeur, Experience::Debutant);
    // Le permis de la rubrique : « Permis C », sans doublon avec le texte
    assert_eq!(
        lire("", "Permis C exigé.\nPermis\nC - Poids lourd Exigé\nBE - Véhicule léger + remorque").autres,
        ["Permis C", "Permis BE"]
    );
}

#[test]
fn codes_et_noms() {
    for d in Diplome::TOUS {
        assert_eq!(Diplome::depuis_code(d.code()), Some(d));
    }
    for e in [Experience::Debutant, Experience::Duree(18), Experience::Demandee, Experience::Souhaitee] {
        assert_eq!(Experience::depuis_code(&e.code()), Some(e));
    }
    assert_eq!(Experience::depuis_code("duree:0"), None);
    assert_eq!(Experience::depuis_code("duree:x"), None);
    assert_eq!(Experience::Duree(24).nom(), "2 ans d'expérience");
    assert_eq!(Experience::Duree(6).demande(), "demande 6 mois d'expérience");
    assert_eq!(duree_texte(12), "1 an");
    assert_eq!(duree_texte(18), "1 an et demi");
    assert_eq!(duree_texte(14), "1 an et 2 mois");
    assert_eq!(LuPar::Ia.nom(), "lu par Claude");
    for t in Diplome::TOUS.iter().flat_map(|d| [d.nom(), d.demande()]) {
        assert!(!t.contains(['\u{2013}', '\u{2014}']), "{t}");
    }
}

#[test]
fn les_citations() {
    let t = "Poste ouvert aux débutants. Le CAP n'est pas exigé.";
    assert!(citation_presente(t, "ouvert aux débutants"));
    assert!(citation_presente(t, "OUVERT AUX DEBUTANTS"));
    assert!(citation_presente(t, "le CAP n'est pas exigé"));
    // Inventée, ou trop courte pour prouver quoi que ce soit
    assert!(!citation_presente(t, "aucun diplôme demandé"));
    assert!(!citation_presente(t, "débutants"));
    assert!(!citation_presente(t, ""));
    assert!(!citation_presente(t, "ouvert aux débutant"));
}

#[test]
fn textes_extremes_sans_planter() {
    for t in ["", ".", "bac", "cap", "2 ans", "permis", "caces", "•••", &"é".repeat(5000), "Bac+", "niveau 9"] {
        let _ = lire(t, t);
    }
}

/// Les lettres d'un permis : une lettre isolée (jamais « c'est », « d'un », « à », « ce poste »), et toutes celles
/// d'une liste.
#[test]
fn les_lettres_des_permis() {
    let permis = |t: &str| lire("", t).autres;
    let rien: [&str; 0] = [];
    // Une lettre isolée, pas le début d'un mot
    assert_eq!(permis("Le permis, c'est un plus."), rien);
    assert_eq!(permis("Le permis c’est un plus."), rien);
    assert_eq!(permis("Le permis c est un plus."), rien);
    assert_eq!(permis("Un permis d'exploitation est demandé."), rien);
    assert_eq!(permis("Permis de chasse apprécié."), rien);
    assert_eq!(permis("Le permis à points n'est pas demandé."), rien);
    assert_eq!(permis("Permis B, c'est indispensable."), ["Permis B"]);
    assert_eq!(permis("Permis B à jour."), ["Permis B"]);
    assert_eq!(permis("Permis B a jour"), ["Permis B"]);
    assert_eq!(permis("Permis B, ce poste demande des déplacements."), ["Permis B"]);
    assert_eq!(permis("Permis B de plus de 3 ans."), ["Permis B"]);
    assert_eq!(permis("PERMIS B, CE POSTE EST A POURVOIR"), ["Permis B"]);
    // Une lettre seule
    assert_eq!(permis("Le permis C est exigé."), ["Permis C"]);
    assert_eq!(permis("permis b exigé"), ["Permis B"]);
    assert_eq!(permis("Permis CE exigé, FIMO à jour."), ["Permis CE"]);
    assert_eq!(permis("Titulaire du permis A"), ["Permis A"]);
    assert_eq!(permis("Permis de conduire B obligatoire."), ["Permis B"]);
    assert_eq!(permis("Permis de conduire exigé."), ["Permis de conduire"]);
    for l in ["B", "C", "CE", "D", "BE", "C1", "C1E", "D1", "A", "A1", "A2", "AM"] {
        assert_eq!(permis(&format!("Permis {l}")), [format!("Permis {l}")]);
        assert_eq!(permis(&format!("Titulaire du permis {l}, vous êtes disponible.")), [format!("Permis {l}")]);
    }
    // Une liste
    assert_eq!(permis("Permis B, C et CE exigés."), ["Permis B", "Permis C", "Permis CE"]);
    assert_eq!(permis("Permis B, C et CE"), ["Permis B", "Permis C", "Permis CE"]);
    assert_eq!(permis("permis B/C"), ["Permis B", "Permis C"]);
    assert_eq!(permis("permis B + C"), ["Permis B", "Permis C"]);
    assert_eq!(permis("Permis B ou C1E souhaité"), ["Permis B", "Permis C1E"]);
    assert_eq!(permis("Permis de conduire B et BE."), ["Permis B", "Permis BE"]);
    assert_eq!(permis("Permis C, CE, D1 et AM"), ["Permis C", "Permis CE", "Permis D1", "Permis AM"]);
    assert_eq!(permis("Le permis A2 ou A est demandé."), ["Permis A2", "Permis A"]);
    assert_eq!(permis("PERMIS B, C, CE - VEHICULE PERSONNEL"), ["Permis B", "Permis C", "Permis CE"]);
    assert_eq!(permis("permis b, c et ce, véhicule personnel"), ["Permis B", "Permis C", "Permis CE"]);
    assert_eq!(permis("Permis B, C, B et C"), ["Permis B", "Permis C"]);
    // La liste s'arrête au premier mot qui n'est pas une lettre de permis
    assert_eq!(permis("Permis B et véhicule exigés, CACES 3 apprécié."), ["Permis B", "CACES 3"]);
    assert_eq!(permis("Permis B et d'une voiture."), ["Permis B"]);
    assert_eq!(permis("Permis B. C'est un poste en CDI."), ["Permis B"]);
}
