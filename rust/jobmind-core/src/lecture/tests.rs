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
    // (version 4 des règles : une durée seulement souhaitée n'est pas exigée ; avant, « duree:6 »)
    ("Expérience de 6 mois minimum souhaitée en caisse.", "non_dit", "souhaitee"),
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
    // (version 4 des règles : « formation assurée » seule ne dit plus « aucun diplôme » ; avant, « aucun »)
    ("Formation\nFormation assurée en interne", "non_dit", "debutant"),
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

/// Les phrases jamais vues de l'audit du 26/11/2026 : écrites avec leur réponse attendue avant d'être mesurées, et
/// jamais utilisées pour corriger les règles.
#[test]
fn le_jeu_jamais_vu_de_l_audit() {
    mesurer_au_moins("3", include_str!("../../tests/jeu_jamais_vu_3.tsv"), 30);
}

fn mesurer(nom: &str, jeu: &str) {
    mesurer_au_moins(nom, jeu, 60);
}

fn mesurer_au_moins(nom: &str, jeu: &str, lignes: usize) {
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
    assert!(n >= lignes, "{n}");
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

// ---------------------------------------------------------------------------------------------------------------
// Les défauts de l'audit du 26/11/2026 (version 4 des règles) : pour chacun, les phrases qui trompaient, et les
// vraies phrases qui doivent continuer à être lues
// ---------------------------------------------------------------------------------------------------------------

/// Ce qui est lu d'une phrase : (diplôme, expérience), en codes.
fn lu(t: &str) -> (String, String) {
    codes(&lire("", t))
}

fn experience_lue(t: &str) -> String {
    lu(t).1
}

fn diplome_lu(t: &str) -> String {
    lu(t).0
}

/// 1. Un avantage (salaire, prime, treizième mois) n'est pas une durée d'expérience.
#[test]
fn un_avantage_n_est_pas_une_duree() {
    for t in [
        "Rémunération sur 13 mois selon expérience.",
        "Prime après 6 mois d'ancienneté.",
        "13e mois après un an d'ancienneté.",
        "Mutuelle prise en charge au bout de 3 mois d'ancienneté.",
        "Salaire selon expérience, versé sur 13 mois.",
        "Prime d'ancienneté dès 2 ans.",
        "Salaire : 13 mois, selon expérience.",
    ] {
        assert_eq!(experience_lue(t), "non_dit", "{t}");
    }
    // La vraie durée n'est plus écrasée par celle de l'avantage
    assert_eq!(experience_lue("3 ans d'expérience exigés. Rémunération sur 13 mois selon expérience."), "duree:36");
    assert_eq!(experience_lue("Expérience de 2 ans exigée, prime après 6 mois d'ancienneté."), "duree:24");
    // Les vraies durées
    assert_eq!(experience_lue("Expérience de 6 mois exigée."), "duree:6");
    assert_eq!(experience_lue("2 à 5 ans d'expérience."), "duree:24");
    assert_eq!(experience_lue("CDI, salaire 1 800 € brut, 2 ans d'expérience exigés."), "duree:24");
    assert_eq!(experience_lue("2 ans d'expérience minimum, salaire selon profil."), "duree:24");
    assert_eq!(experience_lue("Une ancienneté de 2 ans minimum dans un poste similaire est exigée."), "duree:24");
    assert_eq!(experience_lue("Au moins 6 mois dans la restauration."), "duree:6");
}

/// 2. « Master » n'est un Bac+5 que quand il s'agit du diplôme.
#[test]
fn le_master_est_un_diplome_pas_un_fourgon() {
    for t in [
        "Livraisons en Renault Master (permis B).",
        "Conduite d'un fourgon Master 20 m3, permis B exigé.",
        "Vous conduisez un Master L2H2 pour vos tournées.",
        "Scrum Master H/F",
        "Animation d'une master class de pâtisserie.",
        "Responsable Master Data H/F",
    ] {
        assert_eq!(diplome_lu(t), "non_dit", "{t}");
    }
    // Le fourgon ne gagne plus contre « aucun diplôme exigé »
    let l = lire("", "Livraisons en Renault Master (permis B). Aucun diplôme exigé.");
    let d = l.diplome.unwrap();
    assert_eq!((d.valeur, d.confiance), (Diplome::Aucun, Confiance::Forte));
    // Les vrais masters
    for t in [
        "Master en logistique",
        "Master en logistique exigé, permis B souhaité.",
        "Titulaire d'un Master 2 en droit social.",
        "De formation Bac+5 (école d'ingénieur ou master).",
        "Juriste droit social - Master 2 - 2 à 4 ans d'expérience",
        "Responsable qualité : Master qualité, expérience de 3 ans minimum.",
        "Mastère spécialisé en gestion de projet.",
        "Responsable des livraisons : Master en logistique exigé.",
    ] {
        assert_eq!(diplome_lu(t), "bac5", "{t}");
    }
    let sur = lire("", "Titulaire d'un Master 2 en droit social.").diplome.unwrap();
    assert_eq!(sur.confiance, Confiance::Forte);
    // Côté CV (la note de pertinence lit le plus haut diplôme par la même règle)
    assert_eq!(diplome_le_plus_haut("2019 : chauffeur-livreur, livraisons en Renault Master"), None);
    assert_eq!(diplome_le_plus_haut("2018 : Master Informatique, université de Nulle-Part"), Some(Diplome::Bac5));
    assert_eq!(diplome_le_plus_haut("2019 : BTS MCO (non obtenu)"), None);
    assert_eq!(diplome_le_plus_haut("2019 : BTS MCO (non obtenu)\n2017 : Bac pro commerce"), Some(Diplome::Bac));
    assert_eq!(diplome_le_plus_haut("Bac pro commerce obtenu en 2017, BTS MCO non validé"), Some(Diplome::Bac));
    assert_eq!(diplome_le_plus_haut("2019 : BTS MCO, lycée de Metz"), Some(Diplome::Bac2));
    assert_eq!(diplome_le_plus_haut("BTS MCO obtenu en 2019"), Some(Diplome::Bac2));
    let r = crate::recherche::verifier_recherche(&crate::recherche::SaisieRecherche {
        postes: "Vendeur".into(),
        lieu: "ville".into(),
        ville: "Metz".into(),
        rayon: 30,
        teletravail: "peu_importe".into(),
        ..crate::recherche::SaisieRecherche::default()
    })
    .unwrap();
    assert_eq!(crate::pertinence::Profil::nouveau(&r, "2019 : BTS MCO (non obtenu)").diplome_cv, None);
    assert_eq!(crate::pertinence::Profil::nouveau(&r, "Livraisons en Renault Master").diplome_cv, None);
}

/// 3. Deux nombres voisins ne font pas une fourchette, et le chiffre d'un « Bac+2 » n'est pas une durée.
#[test]
fn deux_nombres_voisins_ne_font_pas_une_fourchette() {
    let paire = |d: &str, e: &str| (d.to_string(), e.to_string());
    assert_eq!(lu("Bac+2, 5 ans d'expérience"), paire("bac2", "duree:60"));
    assert_eq!(lu("Bac+5, 3 ans d'expérience"), paire("bac5", "duree:36"));
    assert_eq!(lu("Bac+2/3, 5 ans d'expérience"), paire("bac2", "duree:60"));
    assert_eq!(lu("Niveau Bac, 3 ans d'expérience"), paire("bac", "duree:36"));
    assert_eq!(lu("Bac+2 et 3 ans d'expérience"), paire("bac2", "duree:36"));
    assert_eq!(lu("Bac + 2 ans d'expérience minimum exigés"), paire("bac", "duree:24"));
    assert_eq!(lu("CAP exigé, 2 ans d'expérience, 5 ans souhaités"), paire("cap_bep", "duree:24"));
    // Les vraies fourchettes, et les vrais niveaux
    assert_eq!(experience_lue("2 à 5 ans d'expérience"), "duree:24");
    assert_eq!(experience_lue("2 ou 3 ans d'expérience"), "duree:24");
    assert_eq!(experience_lue("3-5 ans d'expérience exigés"), "duree:36");
    assert_eq!(experience_lue("3/5 ans d'expérience"), "duree:36");
    assert_eq!(experience_lue("Entre 2 et 5 ans d'expérience"), "duree:24");
    assert_eq!(experience_lue("Une à deux années d'expérience"), "duree:12");
    assert_eq!(diplome_lu("Bac+2 exigé"), "bac2");
    assert_eq!(diplome_lu("Bac + 3 exigé"), "bac3");
    assert_eq!(diplome_lu("Bac +2/+3 en commerce"), "bac2");
    assert_eq!(diplome_lu("Niveau bac 2 minimum"), "bac2");
    assert_eq!(diplome_lu("Bac+3 années d'études en informatique"), "bac3");
}

/// 4. Un « débutant accepté » nié demande une expérience.
#[test]
fn un_debutant_accepte_nie() {
    for t in [
        "Débutant accepté : non",
        "Débutant(e) accepté(e) : non",
        "Poste non ouvert aux débutants",
        "Ce poste n'est pas accessible aux débutants",
        "Sans expérience significative s'abstenir",
        "Sans expérience s'abstenir",
        "Débutants s'abstenir.",
    ] {
        let e = lire("", t).experience.unwrap_or_else(|| panic!("{t}"));
        assert_eq!((e.valeur, e.confiance), (Experience::Demandee, Confiance::Forte), "{t}");
    }
    // La durée de la même phrase est lue quand même
    assert_eq!(experience_lue("Débutants s'abstenir, 3 ans d'expérience minimum."), "duree:36");
    // Les vrais « débutant accepté »
    for t in [
        "Débutant accepté",
        "Débutant accepté : oui",
        "Débutant(e) accepté(e).",
        "Poste ouvert aux débutants",
        "Ce poste est accessible aux débutants",
        "Sans expérience ? Pas de problème, débutant accepté.",
        "Diplôme exigé : non - Débutant accepté",
        "Diplôme : non Débutant accepté",
        "Expérience exigée : non",
        "Débutant accepté - Non fumeur",
        "N'hésitez pas, débutants acceptés !",
        "Pas d'expérience exigée.",
    ] {
        let e = lire("", t).experience.unwrap_or_else(|| panic!("{t}"));
        assert_eq!((e.valeur, e.confiance), (Experience::Debutant, Confiance::Forte), "{t}");
    }
}

/// 5. « Vous serez formé » seul ne dit pas « aucun diplôme demandé ».
#[test]
fn etre_forme_ne_dit_rien_du_diplome() {
    let paire = |d: &str, e: &str| (d.to_string(), e.to_string());
    let l = lire("", "Infirmier H/F. Vous serez formé à nos protocoles.");
    assert_eq!(l.diplome, None);
    let e = l.experience.unwrap();
    assert_eq!((e.valeur, e.confiance), (Experience::Debutant, Confiance::Moyenne));
    assert_eq!(lu("Formation assurée en interne."), paire("non_dit", "debutant"));
    // Avec « débutant accepté » dit en clair, « aucun diplôme » se déduit toujours
    assert_eq!(lu("Débutants acceptés, formation assurée."), paire("aucun", "debutant"));
    assert_eq!(lu("Sans expérience ? Pas de problème : on vous forme !"), paire("aucun", "debutant"));
    assert_eq!(lu("Débutant accepté. Vous serez formé à nos méthodes."), paire("aucun", "debutant"));
}

/// 6. « Vous justifiez d'un CAP », « le groupe confirme » ne parlent pas d'expérience.
#[test]
fn justifier_et_confirmer_sans_experience() {
    assert_eq!(lu("Vous justifiez d'un CAP petite enfance."), ("cap_bep".to_string(), "non_dit".to_string()));
    for t in [
        "Le groupe confirme sa croissance.",
        "Notre enseigne confirme son ancrage local.",
        "Votre rendez-vous sera confirmé par téléphone.",
        "CDI confirmé après la période d'essai.",
        "Vous justifiez d'un bon relationnel.",
        "LE GROUPE CONFIRME SA CROISSANCE",
    ] {
        assert_eq!(experience_lue(t), "non_dit", "{t}");
    }
    // Les vraies demandes
    for t in [
        "Vous justifiez d'une expérience en crèche.",
        "Vous justifiez d'une solide expérience en comptabilité.",
        "Cariste confirmé H/F",
        "Cariste confirmé(e)",
        "CARISTE CONFIRME H/F",
        "Profil confirmé.",
        "Pizzaiolo confirmé recherché pour la saison.",
        "Vous êtes un technicien confirmé.",
        "Expérience confirmée exigée.",
        "Experience confirmee.",
    ] {
        assert_eq!(experience_lue(t), "demandee", "{t}");
    }
    assert_eq!(experience_lue("Vous justifiez de 2 ans d'expérience."), "duree:24");
    assert_eq!(experience_lue("Vous êtes débutant ou confirmé ? Rejoignez-nous."), "debutant");
}

/// 7. L'expérience de l'entreprise n'est pas celle qu'elle demande.
#[test]
fn l_experience_de_l_entreprise() {
    for t in [
        "Forte de 15 ans d'expérience, notre agence recrute.",
        "Forte de plus de 15 ans d'expérience, notre agence vous propose un poste de vendeur.",
        "Riche d'une expérience de 30 ans, le groupe poursuit son développement.",
        "Notre société, forte de 20 ans d'expérience dans le bâtiment, recrute un maçon.",
        "Notre agence compte plus de 15 ans d'expérience dans l'intérim.",
        "Nous avons 20 ans d'expérience dans la rénovation.",
        "Le groupe a 30 ans d'expérience.",
        "Avec 25 ans d'existence et d'expérience, la maison recrute.",
    ] {
        assert_eq!(experience_lue(t), "non_dit", "{t}");
    }
    assert_eq!(
        experience_lue("Forte de 15 ans d'expérience, notre agence recrute. 2 ans d'expérience exigés."),
        "duree:24"
    );
    // (le mot « expérience » de l'entreprise est écarté, pas celui qui suit)
    assert_eq!(
        experience_lue("Forte de 15 ans d'expérience, notre agence recherche un vendeur. Expérience en vente exigée."),
        "demandee"
    );
    assert_eq!(experience_lue("Le groupe a 30 ans d'expérience, une expérience en vente est exigée."), "demandee");
    assert_eq!(experience_lue("Bac+5 - 3 ans d'expérience minimum"), "duree:36");
    // Celle du candidat
    assert_eq!(experience_lue("Fort d'une expérience de 3 ans minimum, vous maîtrisez la soudure."), "duree:36");
    assert_eq!(experience_lue("Fort de 5 ans d'expérience en agence, vous rejoignez notre équipe."), "duree:60");
    assert_eq!(experience_lue("Vous rejoignez notre agence avec 2 ans d'expérience minimum."), "duree:24");
    assert_eq!(experience_lue("Nous recherchons un vendeur qui a 2 ans d'expérience."), "duree:24");
    assert_eq!(experience_lue("Notre client recherche un cariste avec 3 ans d'expérience."), "duree:36");
    assert_eq!(experience_lue("Une forte expérience de 5 ans est exigée."), "duree:60");
    assert_eq!(experience_lue("20 ans d'expérience dans le bâtiment."), "duree:240");
}

/// 8. Une durée seulement souhaitée n'est pas exigée.
#[test]
fn une_duree_seulement_souhaitee() {
    for t in [
        "Une expérience de 2 ans serait un plus.",
        "Expérience de 2 ans souhaitée.",
        "Idéalement 2 ans d'expérience en vente.",
        "2 ans d'expérience appréciés.",
        "Expérience souhaitée : 1 an",
        "Expérience de 6 mois minimum souhaitée en caisse.",
        "6 mois d'expérience en industrie souhaités.",
        "6 mois d'expérience souhaités mais pas obligatoires.",
    ] {
        let e = lire("", t).experience.unwrap_or_else(|| panic!("{t}"));
        assert_eq!((e.valeur, e.confiance), (Experience::Souhaitee, Confiance::Forte), "{t}");
        assert!(t.contains(&e.preuve), "{t} : {}", e.preuve);
    }
    // Les durées exigées
    assert_eq!(experience_lue("Expérience de 6 mois exigée."), "duree:6");
    assert_eq!(experience_lue("Expérience de 2 ans."), "duree:24");
    assert_eq!(experience_lue("Plus de 2 ans d'expérience."), "duree:24");
    assert_eq!(experience_lue("Expérience de 2 ans exigée, permis B souhaité."), "duree:24");
    assert_eq!(experience_lue("CAP souhaité, 2 ans d'expérience."), "duree:24");
    assert_eq!(experience_lue("2 ans d'expérience exigés, 5 ans seraient un plus."), "duree:24");
    assert_eq!(experience_lue("2 ans d'expérience indispensables, idéalement en grande distribution."), "duree:24");
    // (une expérience exigée dans la même phrase qu'une durée souhaitée reste exigée)
    assert_eq!(experience_lue("Expérience en vente exigée, 5 ans souhaités."), "demandee");
}
