//! La fiche d'une offre rangée champ par champ (0.1.5, « En bref d'abord ») : [`ranger`] lit le texte d'une offre et
//! remplit une [`Fiche`] (le poste, ce qu'ils demandent, horaires et salaire, l'entreprise).
//!
//! Deux lectures :
//! - une page France Travail (elle a « Critères de l’offre » ou commence par « Offre n° ») est lue par sa structure :
//!   l'en-tête parasite (numéro, titre, « Localiser avec Mappy », « Actualisée le ») disparaît, le texte libre de
//!   l'employeur est rangé d'après ses sous-titres (« Vos missions : », « Profil recherché », « Ce que nous vous
//!   apportons : »...), puis les blocs « Profil souhaité », « Critères de l’offre » et « L'employeur » sont lus ligne à
//!   ligne (« Expérience 6 Mois - Poste équivalent », « Contrat CDD - 3 Mois - Contrat travail »...) ;
//! - tout autre texte (Indeed, Welcome to the Jungle, Adzuna...) passe par [`decouper`] et ses rangs sont rangés dans
//!   les champs.
//!
//! Aucun doublon : une information va à un seul endroit. Aucune perte : ce qui n'est pas parasite finit dans un champ,
//! les champs « _texte » servant de reste. Les valeurs sont des morceaux du texte, au plus remis en forme (« 12.31
//! Euros » devient « 12,31 € », « RHODES » devient « Rhodes »).

use crate::offre::normaliser;
use crate::rubriques::{Exigence, Rang, decouper};

/// Une plage horaire, en minutes depuis minuit ; `fin` est toujours après `debut` (pas de plage de nuit).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plage {
    pub debut: u16,
    pub fin: u16,
}

/// La fiche rangée d'une offre (voir le module). Un champ vide n'est pas connu.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fiche {
    // Le poste
    /// la présentation (texte libre avant les missions), paragraphes séparés par "\n\n"
    pub intro: String,
    /// les missions, une par ligne, sans puce ni « : » final
    pub missions: Vec<String>,
    /// « Rhodes (57) »
    pub lieu: String,
    /// le reste du texte libre qui ne va nulle part ailleurs (souvent vide)
    pub poste_texte: String,
    // Ce qu'ils demandent
    /// puces « Exigé », et les permis et formations sans mention
    pub obligatoire: Vec<String>,
    /// puces « Souhaité », « fortement recommandée », « est un plus »
    pub un_plus: Vec<String>,
    /// « Débutant accepté », « 6 mois sur un poste équivalent », « 6 mois »
    pub experience: String,
    /// « Employé qualifié », « Ouvrier qualifié (P1, P2) »
    pub qualification: String,
    /// bloc « Compétences » ou « Savoirs et savoir-faire »
    pub savoir_faire: Vec<String>,
    /// « Savoir-être professionnels » et les listes « Aptitudes », « Ce que nous recherchons », « Qualités »
    pub qualites: Vec<String>,
    /// le reste (phrases du profil qui ne sont pas des puces)
    pub demande_texte: String,
    // Horaires et salaire
    /// « 12,31 € brut de l'heure », « 1 802 € brut par mois », « Selon profil » ; vide si inconnu
    pub salaire: String,
    /// « 35 h par semaine »
    pub temps: String,
    /// « Travail en journée », « Travail les week-ends et jours fériés », « Travail en 2x8 »
    pub horaires: Vec<String>,
    /// seulement si des heures sont écrites (« 6h-13h30 », « de 8 h à 17 h »)
    pub plages: Vec<Plage>,
    /// « CDD de 3 mois », « Intérim de 6 mois », « CDI », « Indépendant (non salarié) »
    pub contrat: String,
    /// bloc « Avantages » et liste « Ce que nous vous apportons »
    pub avantages: Vec<String>,
    /// « Déplacements : quotidiens, zone départementale »... (une info par ligne)
    pub conditions_texte: String,
    // L'entreprise
    /// « PARC ANIMALIER DE SAINTE CROIX » (tel quel)
    pub entreprise: String,
    pub secteur: String,
    /// « Non disponible » devient vide
    pub effectif: String,
    /// le paragraphe de présentation
    pub entreprise_texte: String,
}

/// Range le texte d'une offre (voir le module). Un texte vide donne une fiche vide.
pub fn ranger(titre: &str, texte: &str) -> Fiche {
    let lignes: Vec<&str> = texte.lines().map(str::trim).collect();
    let mut f = Fiche::default();
    if lignes.iter().all(|l| l.is_empty()) {
        return f;
    }
    if est_france_travail(&lignes) {
        france_travail(&mut f, titre, &lignes);
    } else {
        depuis_rubriques(&mut f, titre, texte);
    }
    finir(&mut f);
    f
}

/// La page a-t-elle la structure France Travail ?
fn est_france_travail(lignes: &[&str]) -> bool {
    lignes.iter().any(|l| normaliser(l) == "criteres de l offre")
        || lignes.iter().find(|l| !l.is_empty()).is_some_and(|l| normaliser(l).starts_with("offre n"))
}

// France Travail ----------------------------------------------------------------------------------------------

fn france_travail(f: &mut Fiche, titre: &str, lignes: &[&str]) {
    let fin = lignes.len();
    let criteres = lignes.iter().position(|l| normaliser(l) == "criteres de l offre");
    let employeur = lignes
        .iter()
        .enumerate()
        .skip(criteres.unwrap_or(0))
        .find(|(_, l)| normaliser(l) == "l employeur")
        .map(|(i, _)| i);
    let avant_profil = criteres.or(employeur).unwrap_or(fin);
    let profil = lignes[..avant_profil].iter().rposition(|l| normaliser(l) == "profil souhaite");
    let mut reperes: Vec<usize> = [profil, criteres, employeur].into_iter().flatten().collect();
    reperes.sort_unstable();
    let suite = |i: usize| reperes.iter().copied().find(|&r| r > i).unwrap_or(fin);

    // L'en-tête parasite : « Offre n° », le titre, « VILLE (57)Localiser avec Mappy », « Débutant accepté »,
    // « Actualisée le »
    let mut debutant = false;
    let mut debut = 0;
    let mut apres_numero = false;
    while debut < lignes.len() {
        let l = lignes[debut];
        let n = normaliser(l);
        if n.starts_with("offre n") {
            apres_numero = true;
            debut += 1;
            continue;
        }
        if let Some(v) = l.strip_suffix("Localiser avec Mappy") {
            if f.lieu.is_empty() {
                f.lieu = lieu_propre(v);
            }
        } else if n == "debutant accepte" {
            debutant = true;
        } else {
            let date = n.starts_with("actualisee le") || n.starts_with("publiee le") || n.starts_with("publie le");
            if !(l.is_empty() || apres_numero || date || meme_titre(l, titre)) {
                break;
            }
        }
        if !l.is_empty() {
            apres_numero = false;
        }
        debut += 1;
    }
    let fin_libre = reperes.first().copied().unwrap_or(fin).max(debut);
    texte_libre(f, &lignes[debut..fin_libre]);
    if let Some(p) = profil {
        bloc_profil(f, &lignes[p + 1..suite(p)]);
    }
    if let Some(c) = criteres {
        bloc_criteres(f, &lignes[c + 1..suite(c)]);
    }
    if let Some(e) = employeur {
        bloc_employeur(f, &lignes[e + 1..suite(e)]);
    }
    if debutant && f.experience.is_empty() {
        f.experience = "Débutant accepté".to_string();
    }
}

/// Les zones du texte libre, d'après ses sous-titres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zone {
    Intro,
    Missions,
    Profil,
    Avantages,
    Horaires,
    Reste,
}

/// Les sous-titres du texte libre reconnus tels quels (remis au propre).
const TITRES_MISSIONS: [&str; 12] = [
    "missions",
    "mission",
    "vos missions",
    "votre mission",
    "vos missions principales",
    "missions principales",
    "vos principales missions",
    "vos taches",
    "vos activites",
    "vos responsabilites",
    "votre quotidien",
    "au quotidien",
];
const TITRES_PROFIL: [&str; 16] = [
    "profil recherche",
    "profil",
    "votre profil",
    "le profil",
    "profil du candidat",
    "aptitudes recherchees",
    "aptitudes",
    "ce que nous recherchons",
    "qualites",
    "qualites requises",
    "savoir etre",
    "competences",
    "competences requises",
    "vos competences",
    "prerequis",
    "vos atouts",
];
const TITRES_AVANTAGES: [&str; 9] = [
    "ce que nous vous apportons",
    "ce que nous vous offrons",
    "ce que nous offrons",
    "nous vous offrons",
    "nous offrons",
    "avantages",
    "vos avantages",
    "les avantages",
    "pourquoi nous rejoindre",
];
const TITRES_HORAIRES: [&str; 3] = ["horaires", "horaire", "horaires de travail"];

/// Un sous-titre du texte libre : une ligne courte sans puce, connue telle quelle, ou finie par « : » et qui parle de
/// missions, de profil ou d'avantages (« Vos principales missions seront de : »).
fn titre_libre(l: &str) -> Option<Zone> {
    if puce(l).0 || mots(l) > 8 {
        return None;
    }
    let n = normaliser(l);
    for (liste, zone) in [
        (&TITRES_MISSIONS[..], Zone::Missions),
        (&TITRES_PROFIL[..], Zone::Profil),
        (&TITRES_AVANTAGES[..], Zone::Avantages),
        (&TITRES_HORAIRES[..], Zone::Horaires),
    ] {
        if liste.contains(&n.as_str()) {
            return Some(zone);
        }
    }
    if !l.ends_with(':') {
        return None;
    }
    let a = |m: &[&str]| n.split(' ').any(|x| m.contains(&x));
    if a(&["mission", "missions", "taches", "activites", "responsabilites"]) {
        Some(Zone::Missions)
    } else if a(&["apportons", "offrons", "avantages"]) {
        Some(Zone::Avantages)
    } else if a(&["profil", "recherchons", "aptitudes", "qualites", "competences", "atouts"]) {
        Some(Zone::Profil)
    } else {
        None
    }
}

/// Les phrases parasites du texte libre.
fn parasite_libre(l: &str) -> bool {
    let n = normaliser(l);
    n.starts_with("si ce poste vous interesse") || n.starts_with("si cette offre vous interesse")
}

/// Une puce sans sous-titre qui parle du candidat (« Vous êtes rigoureux », « Une première expérience... »).
fn profil_probable(t: &str) -> bool {
    let n = normaliser(t);
    [
        "vous etes",
        "vous avez",
        "vous aimez",
        "vous appreciez",
        "vous disposez",
        "vous maitrisez",
        "vous savez",
        "vous possedez",
        "titulaire",
        "une premiere experience",
        "une experience",
        "permis",
    ]
    .iter()
    .any(|d| n.starts_with(d))
}

/// Une ligne courte qui dit le rythme de travail (« Travail en 2x8 ») ou des heures (« 8h-12h »).
fn horaire_probable(t: &str) -> bool {
    let n = normaliser(t);
    let rythme = ["journee", "nuit", "2x8", "3x8", "4x8", "5x8", "equipes", "week", "decale", "decales", "astreinte"];
    (mots(t) <= 8 && n.starts_with("travail ") && n.split(' ').any(|m| rythme.contains(&m)))
        || (mots(t) <= 15 && !plages_de(t).is_empty())
}

/// Une phrase courte du texte libre qui dit le diplôme ou l'expérience demandés (« Aucun diplôme demandé, formation
/// assurée », « Débutants acceptés ») : elle va dans « Ce qu'ils demandent ».
fn demande_probable(t: &str) -> bool {
    let n = normaliser(t);
    mots(t) <= 15 && n.split(' ').any(|m| matches!(m, "diplome" | "diplomes" | "debutant" | "debutants"))
}

/// « Horaires : 6h-13h30 » : la valeur.
fn valeur_horaires(l: &str) -> Option<&str> {
    if puce(l).0 {
        return None;
    }
    let (a, v) = l.split_once(':')?;
    let v = v.trim();
    (!v.is_empty() && TITRES_HORAIRES.contains(&normaliser(a).as_str())).then_some(v)
}

/// Le texte libre de l'employeur, rangé d'après ses sous-titres.
fn texte_libre(f: &mut Fiche, lignes: &[&str]) {
    let mut zone = Zone::Intro;
    let mut faits = 0usize;
    // les lignes sans puce sous « Vos missions », avant la première puce : la présentation si des puces suivent,
    // sinon les missions elles-mêmes
    let mut attente: Vec<(String, bool)> = Vec::new();
    let mut blanc = false;
    for &l in lignes {
        if l.is_empty() {
            blanc = true;
            continue;
        }
        let nouveau = std::mem::take(&mut blanc);
        if parasite_libre(l) {
            continue;
        }
        if let Some(z) = titre_libre(l) {
            f.missions.extend(attente.drain(..).map(|(t, _)| propre(&t)));
            zone = z;
            faits = 0;
            continue;
        }
        if let Some(v) = valeur_horaires(l) {
            f.horaires.push(v.to_string());
            continue;
        }
        let (marque, t) = puce(l);
        if t.is_empty() {
            continue;
        }
        // une ligne sans puce après une ligne vide ferme la liste en cours
        if zone != Zone::Intro && zone != Zone::Reste && faits > 0 && nouveau && !marque {
            f.missions.extend(attente.drain(..).map(|(t, _)| propre(&t)));
            zone = Zone::Reste;
        }
        if marque && zone != Zone::Profil && zone != Zone::Avantages && profil_probable(t) {
            demande(f, t);
            faits += 1;
            continue;
        }
        match zone {
            Zone::Missions => {
                if marque {
                    for (i, (a, n)) in attente.drain(..).enumerate() {
                        ajouter(&mut f.intro, &a, i == 0 || n);
                    }
                    f.missions.push(propre(t));
                    faits += 1;
                } else if faits == 0 {
                    attente.push((t.to_string(), nouveau));
                } else {
                    f.missions.push(propre(t));
                    faits += 1;
                }
            }
            Zone::Profil => {
                if !marque && mots(t) > 12 && !marque_plus(t) && !marque_obligatoire(t) {
                    ajouter(&mut f.demande_texte, t, nouveau);
                } else {
                    demande(f, t);
                }
                faits += 1;
            }
            Zone::Avantages => {
                f.avantages.push(propre(t));
                faits += 1;
            }
            Zone::Horaires => {
                f.horaires.push(propre(t));
                faits += 1;
            }
            Zone::Intro | Zone::Reste => {
                if !marque && horaire_probable(t) {
                    f.horaires.push(propre(t));
                    continue;
                }
                // (0.1.5 : « Aucun diplôme demandé, formation assurée » dans le texte libre : c'est ce qu'ils
                // demandent, pas le poste)
                if !marque && demande_probable(t) {
                    ajouter(&mut f.demande_texte, t, nouveau);
                    continue;
                }
                let dest = if zone == Zone::Intro { &mut f.intro } else { &mut f.poste_texte };
                if marque {
                    ajouter(dest, &format!("- {t}"), nouveau);
                } else {
                    ajouter(dest, t, nouveau);
                }
            }
        }
    }
    f.missions.extend(attente.drain(..).map(|(t, _)| propre(&t)));
}

/// Les mots qui font d'une ligne du profil un « plus ».
fn marque_plus(t: &str) -> bool {
    let n = format!(" {} ", normaliser(t));
    n.contains(" un plus ")
        || n.contains(" un atout ")
        || [
            " recommande ",
            " recommandee ",
            " recommandes ",
            " recommandees ",
            " souhaite ",
            " souhaitee ",
            " souhaites ",
            " souhaitees ",
            " souhaitable ",
            " apprecie ",
            " appreciee ",
            " apprecies ",
            " appreciees ",
            " idealement ",
        ]
        .iter()
        .any(|m| n.contains(m))
}

/// Les mots qui font d'une ligne du profil une obligation.
fn marque_obligatoire(t: &str) -> bool {
    let n = format!(" {} ", normaliser(t));
    [
        " exige ",
        " exigee ",
        " exiges ",
        " exigees ",
        " obligatoire ",
        " obligatoires ",
        " indispensable ",
        " indispensables ",
        " imperatif ",
        " imperative ",
        " requis ",
        " requise ",
    ]
    .iter()
    .any(|m| n.contains(m))
}

/// Range une ligne de profil : « Exigé » ou obligation, « Souhaité » ou plus, savoir faire (« Compétences... »,
/// « Connaissance... », « Maîtrise... »), sinon qualité.
fn demande(f: &mut Fiche, t: &str) {
    let (reste, ex) = exigence_fin(t);
    match ex {
        Some(Exigence::Exige) => f.obligatoire.push(propre(&reste)),
        Some(Exigence::Souhaite) => f.un_plus.push(propre(&reste)),
        None if marque_plus(t) => f.un_plus.push(propre(&t.replace(" est fortement ", " fortement "))),
        None if marque_obligatoire(t) => f.obligatoire.push(propre(t)),
        None => {
            let n = normaliser(t);
            let premier = n.split(' ').next().unwrap_or("");
            let savoir = ["competence", "competences", "connaissance", "connaissances", "maitrise"];
            if savoir.contains(&premier) {
                f.savoir_faire.push(propre(t));
            } else {
                f.qualites.push(propre(t));
            }
        }
    }
}

/// Les blocs du « Profil souhaité ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlocProfil {
    Aucun,
    SavoirFaire,
    Qualites,
    Permis,
    Formation,
}

/// Le bloc « Profil souhaité » : « Expérience ... », « Qualification ... », puis les blocs à puces.
fn bloc_profil(f: &mut Fiche, lignes: &[&str]) {
    let mut bloc = BlocProfil::Aucun;
    for &l in lignes {
        if l.is_empty() {
            continue;
        }
        let titre = match normaliser(l).as_str() {
            "competences" | "savoirs et savoir faire" | "savoir faire" | "competences professionnelles" => {
                Some(BlocProfil::SavoirFaire)
            }
            "savoir etre professionnels" | "savoir etre" | "qualites professionnelles" => Some(BlocProfil::Qualites),
            "permis" | "permis de conduire" => Some(BlocProfil::Permis),
            "formation" | "formations" | "diplome" | "diplomes" | "langues" | "langue" => Some(BlocProfil::Formation),
            _ => None,
        };
        if let Some(b) = titre {
            bloc = b;
            continue;
        }
        let (marque, t) = puce(l);
        if !marque {
            if let Some(v) = apres_libelle(l, "Expérience").filter(|v| !v.is_empty()) {
                f.experience = experience_propre(v);
                continue;
            }
            if let Some(v) = apres_libelle(l, "Qualification").filter(|v| !v.is_empty()) {
                f.qualification = qualification_propre(v);
                continue;
            }
            if apres_libelle(l, "Public cible").is_some() {
                continue;
            }
        }
        let (reste, ex) = exigence_fin(t);
        match bloc {
            BlocProfil::Aucun => ajouter(&mut f.demande_texte, t, false),
            BlocProfil::SavoirFaire | BlocProfil::Qualites => match ex {
                Some(Exigence::Exige) => f.obligatoire.push(propre(&reste)),
                Some(Exigence::Souhaite) => f.un_plus.push(propre(&reste)),
                None if bloc == BlocProfil::SavoirFaire => f.savoir_faire.push(propre(t)),
                None => f.qualites.push(propre(t)),
            },
            BlocProfil::Permis | BlocProfil::Formation => {
                let v = if bloc == BlocProfil::Permis { permis_propre(&reste) } else { propre(&reste) };
                if ex == Some(Exigence::Souhaite) {
                    f.un_plus.push(v);
                } else {
                    f.obligatoire.push(v);
                }
            }
        }
    }
}

/// Les blocs des « Critères de l’offre ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlocCriteres {
    Aucun,
    Horaires,
    Avantages,
}

/// Le bloc « Critères de l’offre » : lieu, déplacements, contrat, conditions horaires, salaire, avantages.
fn bloc_criteres(f: &mut Fiche, lignes: &[&str]) {
    let mut bloc = BlocCriteres::Aucun;
    for &l in lignes {
        if l.is_empty() {
            continue;
        }
        let n = normaliser(l);
        let (marque, t) = puce(l);
        if !marque {
            if n == "conditions horaires" || n == "horaires" {
                bloc = BlocCriteres::Horaires;
                continue;
            }
            if n == "avantages" {
                bloc = BlocCriteres::Avantages;
                continue;
            }
            if apres_libelle(l, "Intitulé métier").is_some() {
                continue;
            }
            if let Some(v) = apres_libelle(l, "Lieu de travail").filter(|v| !v.is_empty()) {
                f.lieu = lieu_propre(v);
                continue;
            }
            if let Some(v) = apres_libelle(l, "Déplacements").filter(|v| !v.is_empty()) {
                ajouter(&mut f.conditions_texte, &deplacements(v), false);
                continue;
            }
            if let Some(v) = apres_libelle(l, "Contrat").filter(|v| !v.is_empty()) {
                f.contrat = contrat_propre(v);
                continue;
            }
            if let Some(v) = apres_libelle(l, "Durée du travail").filter(|v| !v.is_empty()) {
                condition_horaire(f, v);
                continue;
            }
            if let Some(v) = apres_libelle(l, "Salaire") {
                f.salaire = salaire_propre(v);
                bloc = BlocCriteres::Avantages;
                continue;
            }
        }
        match bloc {
            BlocCriteres::Horaires => condition_horaire(f, t),
            BlocCriteres::Avantages => f.avantages.push(propre(t)),
            BlocCriteres::Aucun => ajouter(&mut f.conditions_texte, t, false),
        }
    }
}

/// Le bloc « L'employeur » : le nom, le secteur, l'effectif, puis la présentation.
fn bloc_employeur(f: &mut Fiche, lignes: &[&str]) {
    let mut premier = true;
    let mut blanc = false;
    for &l in lignes {
        if l.is_empty() {
            blanc = true;
            continue;
        }
        let nouveau = std::mem::take(&mut blanc);
        let n = normaliser(l);
        if n.starts_with("voir toutes les offres") || n.starts_with("decouvrir l employeur") {
            continue;
        }
        if let Some(v) = apres_libelle(l, "Secteur d'activité") {
            f.secteur = v.to_string();
        } else if let Some(v) = apres_libelle(l, "Effectif") {
            f.effectif = if normaliser(v) == "non disponible" { String::new() } else { v.to_string() };
        } else if apres_libelle(l, "Site internet").is_some() {
            // (le site est lu sur la page et montré par la carte de l'entreprise : aucune adresse n'est envoyée à
            // l'écran dans le texte)
        } else if premier && f.entreprise.is_empty() && mots(l) <= 10 {
            f.entreprise = l.to_string();
        } else {
            ajouter(&mut f.entreprise_texte, l, nouveau);
        }
        premier = false;
    }
}

/// Une puce des conditions horaires : la durée (« 35H/semaine »), le temps plein ou partiel, sinon un horaire.
fn condition_horaire(f: &mut Fiche, t: &str) {
    let n = normaliser(t);
    let plein = n.starts_with("temps plein") || n.starts_with("temps partiel");
    match temps_propre(t).or_else(|| plein.then(|| propre(t))) {
        Some(v) if f.temps.is_empty() => f.temps = v,
        Some(v) => {
            f.temps.push_str(", ");
            f.temps.push_str(&minuscule(&v));
        }
        None => f.horaires.push(propre(t)),
    }
}

// Les autres sites : les rangs de `rubriques::decouper` ---------------------------------------------------------

fn depuis_rubriques(f: &mut Fiche, titre: &str, texte: &str) {
    for onglet in decouper(titre, texte) {
        for r in &onglet.rangs {
            let n = normaliser(&r.nom);
            let mut items: Vec<String> =
                r.valeur.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
            items.extend(r.puces.iter().cloned());
            match onglet.id {
                "poste" => rang_poste(f, &n, r, &items),
                "demande" => rang_demande(f, &n, r, &items),
                "conditions" => rang_conditions(f, &n, r, &items),
                _ => rang_entreprise(f, &n, r, &items),
            }
        }
    }
}

fn rang_poste(f: &mut Fiche, n: &str, r: &Rang, items: &[String]) {
    if n.starts_with("lieu") {
        if let Some(v) = items.first() {
            f.lieu = lieu_propre(v);
        }
        ajouter_lignes(&mut f.poste_texte, items.iter().skip(1));
        return;
    }
    if n == "informations complementaires" {
        ajouter(&mut f.poste_texte, &r.valeur, true);
        ajouter_lignes(&mut f.poste_texte, r.puces.iter().map(|p| format!("- {p}")));
        return;
    }
    let description = matches!(
        n,
        "" | "description du poste"
            | "descriptif du poste"
            | "le poste"
            | "poste"
            | "description"
            | "description de l offre"
            | "descriptif de l offre"
    );
    if description || !r.puces.is_empty() {
        ajouter(&mut f.intro, &r.valeur, true);
    } else {
        f.missions.extend(r.valeur.lines().map(str::trim).filter(|l| !l.is_empty()).map(propre));
    }
    for p in &r.puces {
        if profil_probable(p) {
            demande(f, p);
        } else {
            f.missions.push(propre(p));
        }
    }
}

fn rang_demande(f: &mut Fiche, n: &str, r: &Rang, items: &[String]) {
    let permis = n.starts_with("permis");
    let forme = |t: &String| if permis { permis_propre(t) } else { propre(t) };
    if r.exigence == Some(Exigence::Exige) {
        f.obligatoire.extend(items.iter().map(forme));
    } else if r.exigence == Some(Exigence::Souhaite) {
        f.un_plus.extend(items.iter().map(forme));
    } else if n.starts_with("experience") {
        f.experience = items.join(", ");
    } else if n == "qualification" {
        f.qualification = items.join(", ");
    } else if matches!(
        n,
        "competences" | "vos competences" | "competences requises" | "savoir faire" | "savoirs et savoir faire"
    ) {
        ranger_courtes(f, items, |f| &mut f.savoir_faire);
    } else if matches!(n, "savoir etre professionnels" | "savoir etre" | "qualites" | "qualites requises") {
        ranger_courtes(f, items, |f| &mut f.qualites);
    } else if permis || matches!(n, "formation" | "formations" | "diplome" | "langues" | "langue" | "prerequis") {
        f.obligatoire.extend(items.iter().map(forme));
    } else {
        ajouter(&mut f.demande_texte, &r.valeur, true);
        for p in &r.puces {
            demande(f, p);
        }
    }
}

/// Les lignes courtes vont dans la liste, les longues dans `demande_texte`.
fn ranger_courtes(f: &mut Fiche, items: &[String], liste: fn(&mut Fiche) -> &mut Vec<String>) {
    for t in items {
        if mots(t) <= 12 {
            liste(f).push(propre(t));
        } else {
            ajouter(&mut f.demande_texte, t, true);
        }
    }
}

fn rang_conditions(f: &mut Fiche, n: &str, r: &Rang, items: &[String]) {
    match n {
        "type de contrat" | "contrat" | "duree du contrat" | "duree de la mission" => {
            for t in items {
                let v = contrat_libre(t);
                if f.contrat.is_empty() {
                    f.contrat = v;
                } else {
                    f.contrat.push_str(", ");
                    f.contrat.push_str(&minuscule(&v));
                }
            }
        }
        "duree du travail" | "temps de travail" | "horaires" | "horaire" | "horaires de travail" => {
            for t in items {
                condition_horaire(f, t);
            }
        }
        "salaire" | "salaire brut" | "remuneration" | "remuneration et avantages" | "salaire et avantages" => {
            for t in items {
                if f.salaire.is_empty() && (parle_d_argent(t) || !n.contains("avantages")) {
                    f.salaire = salaire_propre(t);
                } else {
                    f.avantages.push(propre(t));
                }
            }
        }
        "avantages" | "nous offrons" | "nous vous offrons" | "ce que nous offrons" | "ce que nous vous offrons" => {
            f.avantages.extend(items.iter().map(|t| propre(t)));
        }
        _ => {
            for t in items {
                ajouter(&mut f.conditions_texte, &format!("{} : {}", r.nom, t), false);
            }
        }
    }
}

fn rang_entreprise(f: &mut Fiche, n: &str, r: &Rang, items: &[String]) {
    match n {
        "secteur d activite" => f.secteur = items.join(", "),
        "taille" | "taille de l entreprise" => f.effectif = items.join(", "),
        "employeur" if items.len() == 1 && mots(&items[0]) <= 8 && f.entreprise.is_empty() => {
            f.entreprise = items[0].clone();
        }
        _ => {
            ajouter(&mut f.entreprise_texte, &r.valeur, true);
            ajouter_lignes(&mut f.entreprise_texte, r.puces.iter().map(|p| format!("- {p}")));
        }
    }
}

/// Une ligne qui donne une somme (« 11,88 € brut de l'heure », « 1900 euros »).
fn parle_d_argent(t: &str) -> bool {
    let n = normaliser(t);
    t.contains('€') || n.split(' ').any(|m| m.starts_with("euro")) || n.contains(" brut")
}

// Les valeurs remises en forme --------------------------------------------------------------------------------

/// La suite d'une ligne « Libellé valeur » (sans deux-points, comme France Travail l'écrit), si elle commence par le
/// libellé (majuscules, accents et apostrophes ne comptent pas). Un « : » après le libellé est retiré.
fn apres_libelle<'a>(ligne: &'a str, libelle: &str) -> Option<&'a str> {
    let cible = normaliser(libelle);
    let mut fin = 0;
    for m in ligne.split_whitespace().take(6) {
        fin = ligne[fin..].find(m).map(|p| fin + p + m.len())?;
        let n = normaliser(&ligne[..fin]);
        if n == cible {
            return Some(ligne[fin..].trim().trim_start_matches(':').trim());
        }
        if !cible.starts_with(&format!("{n} ")) {
            return None;
        }
    }
    None
}

/// « 57 - RHODES », « RHODES (57) » : « Rhodes (57) ».
fn lieu_propre(v: &str) -> String {
    let v = v.trim();
    if let Some((a, b)) = v.split_once(" - ")
        && departement(a.trim())
    {
        return format!("{} ({})", ville(b.trim()), a.trim());
    }
    if let Some(dedans) = v.strip_suffix(')')
        && let Some(i) = dedans.rfind('(')
        && departement(dedans[i + 1..].trim())
    {
        return format!("{} ({})", ville(dedans[..i].trim()), dedans[i + 1..].trim());
    }
    ville(v)
}

fn departement(t: &str) -> bool {
    (1..=3).contains(&t.len()) && t.chars().all(|c| c.is_ascii_alphanumeric()) && t.chars().any(|c| c.is_ascii_digit())
}

/// Les petits mots d'un nom de ville, laissés en minuscules (« Saint-Jean-de-Luz »).
const PETITS_MOTS: [&str; 13] = ["de", "du", "des", "la", "le", "les", "sur", "sous", "en", "et", "aux", "au", "lès"];

/// Un nom de ville en capitales remis en minuscules (« SAINT-JEAN-DE-LUZ » : « Saint-Jean-de-Luz ») ; un nom déjà
/// écrit en minuscules reste tel quel.
fn ville(t: &str) -> String {
    if t.chars().any(char::is_lowercase) {
        return t.to_string();
    }
    let mut s = String::new();
    let mut premier = true;
    for morceau in t.split_inclusive([' ', '-', '\'', '’']) {
        let (mot, sep) = match morceau.char_indices().last() {
            Some((i, ' ' | '-' | '\'' | '’')) => (&morceau[..i], &morceau[i..]),
            _ => (morceau, ""),
        };
        let m = mot.to_lowercase();
        if !premier && PETITS_MOTS.contains(&m.as_str()) {
            s.push_str(&m);
        } else {
            let mut c = m.chars();
            if let Some(p) = c.next() {
                s.extend(p.to_uppercase());
                s.push_str(c.as_str());
            }
        }
        if !mot.is_empty() {
            premier = false;
        }
        s.push_str(sep);
    }
    s
}

/// « Quotidiens Zone départementale » : « Déplacements : quotidiens, zone départementale ».
fn deplacements(v: &str) -> String {
    let v = v.trim();
    let valeur = match v.find(" Zone ") {
        Some(i) => format!("{}, {}", minuscule(v[..i].trim()), minuscule(v[i + 1..].trim())),
        None => minuscule(v),
    };
    format!("Déplacements : {valeur}")
}

/// « 3 Mois », « 12 Jour(s) », « 2 An(s) » : « 3 mois », « 12 jours », « 2 ans ».
fn duree(t: &str) -> Option<String> {
    let mut m = t.split_whitespace();
    let x: u32 = m.next()?.parse().ok()?;
    let unite = normaliser(&m.collect::<Vec<_>>().join(" "));
    let (un, plusieurs) = match unite.as_str() {
        "mois" => ("mois", "mois"),
        "jour" | "jours" | "jour s" => ("jour", "jours"),
        "semaine" | "semaines" | "semaine s" => ("semaine", "semaines"),
        "an" | "ans" | "an s" | "annee" | "annees" | "annee s" => ("an", "ans"),
        _ => return None,
    };
    Some(format!("{x} {}", if x > 1 { plusieurs } else { un }))
}

/// « CDD - 3 Mois - Contrat travail » : « CDD de 3 mois » ; « Profession libérale - Emploi non salarié » :
/// « Indépendant (non salarié) ».
fn contrat_propre(v: &str) -> String {
    let mut parts = v.split(" - ").map(str::trim).filter(|p| !p.is_empty());
    let Some(premier) = parts.next() else { return String::new() };
    let mut s = contrat_libre(premier);
    let independant = s.starts_with("Indépendant");
    for p in parts {
        let n = normaliser(p);
        if n == "contrat travail" || (independant && n == "emploi non salarie") {
            continue;
        }
        if let Some(d) = duree(p) {
            s.push_str(" de ");
            s.push_str(&d);
        } else {
            s.push_str(", ");
            s.push_str(&minuscule(p));
        }
    }
    s
}

/// Le type de contrat en clair (« Contrat à durée indéterminée » : « CDI »), sinon tel quel.
fn contrat_libre(t: &str) -> String {
    let t = t.trim();
    if t.contains(" - ") {
        return contrat_propre(t);
    }
    let v = match normaliser(t).as_str() {
        "cdi" | "contrat a duree indeterminee" => "CDI",
        "cdd" | "contrat a duree determinee" => "CDD",
        "interim" | "mission interimaire" | "travail temporaire" | "travail interimaire" => "Intérim",
        "profession liberale" | "independant" | "emploi non salarie" => "Indépendant (non salarié)",
        "saisonnier" | "contrat saisonnier" => "Saisonnier",
        _ => return t.to_string(),
    };
    v.to_string()
}

/// « 6 Mois - Poste équivalent » : « 6 mois sur un poste équivalent » ; « 6 Mois » : « 6 mois ».
fn experience_propre(v: &str) -> String {
    let mut parts = v.split(" - ").map(str::trim).filter(|p| !p.is_empty());
    let Some(premier) = parts.next() else { return String::new() };
    let mut s = duree(premier).unwrap_or_else(|| premier.to_string());
    for p in parts {
        match normaliser(p).as_str() {
            "poste equivalent" => s.push_str(" sur un poste équivalent"),
            "exige" | "exigee" => s.push_str(", exigée"),
            "souhaite" | "souhaitee" => s.push_str(", souhaitée"),
            _ => {
                s.push_str(", ");
                s.push_str(&minuscule(p));
            }
        }
    }
    s
}

/// « Ouvrier qualifié (P1,P2) » : « Ouvrier qualifié (P1, P2) ».
fn qualification_propre(v: &str) -> String {
    let mut s = String::new();
    let mut c = v.trim().chars().peekable();
    while let Some(x) = c.next() {
        s.push(x);
        if x == ',' && c.peek().is_some_and(|y| !y.is_whitespace()) {
            s.push(' ');
        }
    }
    s
}

/// « B - Véhicule léger » : « Permis B, voiture ».
fn permis_propre(t: &str) -> String {
    let t = propre(t);
    if let Some((code, desc)) = t.split_once(" - ")
        && (1..=4).contains(&code.trim().len())
        && !code.trim().contains(' ')
    {
        let d = match normaliser(desc).as_str() {
            "vehicule leger" => "voiture".to_string(),
            "vehicule leger remorque" => "voiture avec remorque".to_string(),
            "poids lourd" => "poids lourd".to_string(),
            "transport en commun" => "bus".to_string(),
            _ => minuscule(desc.trim()),
        };
        return format!("Permis {}, {d}", code.trim());
    }
    if normaliser(&t).starts_with("permis") { t } else { format!("Permis {t}") }
}

/// « Horaire de 12.31 Euros » : « 12,31 € brut de l'heure » ; « Mensuel de 1802 Euros sur 12 mois » : « 1 802 € brut
/// par mois » ; « selon profils et expériences » : « Selon profil » ; « N.A » : vide. Une forme inconnue reste telle
/// quelle.
fn salaire_propre(v: &str) -> String {
    let mut v = v.trim();
    if let Some(r) = v.strip_prefix("brut").or_else(|| v.strip_prefix("Brut")) {
        v = r.trim().trim_start_matches(':').trim();
    }
    let n = normaliser(v);
    if matches!(n.as_str(), "" | "n a" | "na" | "non communique" | "non renseigne") {
        return String::new();
    }
    if n.starts_with("selon") {
        return "Selon profil".to_string();
    }
    let suffixe = match n.split(' ').next() {
        Some("horaire") => "brut de l'heure",
        Some("mensuel") => "brut par mois",
        Some("annuel") => "brut par an",
        _ => return v.to_string(),
    };
    let m: Vec<&str> = v.split_whitespace().collect();
    let mut sommes = Vec::new();
    let mut mois = None;
    for (i, w) in m.iter().enumerate() {
        let suivant = m.get(i + 1).map(|x| normaliser(x)).unwrap_or_default();
        if suivant.starts_with("euro")
            && let Some(x) = somme(w)
        {
            sommes.push(x);
        }
        if normaliser(w) == "sur" && m.get(i + 2).is_some_and(|x| normaliser(x) == "mois") {
            mois = m.get(i + 1).and_then(|x| x.parse::<u32>().ok());
        }
    }
    let montant = match sommes.as_slice() {
        [a] => format!("{a} €"),
        [a, b] => format!("{a} à {b} €"),
        _ => return v.to_string(),
    };
    match mois.filter(|&x| x != 12) {
        Some(x) => format!("{montant} {suffixe}, sur {x} mois"),
        None => format!("{montant} {suffixe}"),
    }
}

/// « 1802.00 » : « 1 802 » ; « 12.31 » : « 12,31 ».
fn somme(t: &str) -> Option<String> {
    let t = t.replace(',', ".");
    let (entier, dec) = t.split_once('.').unwrap_or((&t, ""));
    if entier.is_empty() || !entier.chars().all(|c| c.is_ascii_digit()) || !dec.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let entier = entier.trim_start_matches('0');
    let entier = if entier.is_empty() { "0" } else { entier };
    let mut e = String::new();
    for (i, c) in entier.chars().enumerate() {
        if i > 0 && (entier.len() - i) % 3 == 0 {
            e.push(' ');
        }
        e.push(c);
    }
    let dec = dec.trim_end_matches('0');
    Some(match dec.len() {
        0 => e,
        1 => format!("{e},{dec}0"),
        _ => format!("{e},{dec}"),
    })
}

/// « 35H/semaine » : « 35 h par semaine » ; « 35H30/semaine » : « 35 h 30 par semaine ».
fn temps_propre(t: &str) -> Option<String> {
    let s: String = t.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
    let (duree, unite) = s.split_once('/')?;
    let unite = match unite {
        "semaine" | "sem" => "semaine",
        "mois" => "mois",
        "jour" => "jour",
        _ => return None,
    };
    let (h, m) = duree.split_once('h')?;
    if h.is_empty() || !h.chars().all(|c| c.is_ascii_digit()) || !m.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(if m.is_empty() { format!("{h} h par {unite}") } else { format!("{h} h {m} par {unite}") })
}

// Les plages horaires -----------------------------------------------------------------------------------------

/// Les plages horaires écrites dans un texte : « 6h-13h30 », « de 8 h à 12 h », « 08:00 - 12:00 ». Une plage de nuit
/// (fin avant le début) est laissée de côté.
fn plages_de(t: &str) -> Vec<Plage> {
    let heures = heures_de(t);
    let mut plages = Vec::new();
    let mut i = 0;
    while i + 1 < heures.len() {
        let (_, fin_a, a) = heures[i];
        let (debut_b, _, b) = heures[i + 1];
        let lien = t[fin_a..debut_b].trim();
        let tiret = matches!(lien, "-" | "\u{2013}" | "\u{2014}");
        if (tiret || matches!(normaliser(lien).as_str(), "a" | "au" | "jusqu a")) && b > a {
            plages.push(Plage { debut: a, fin: b });
            i += 2;
        } else {
            i += 1;
        }
    }
    plages
}

/// Les heures écrites dans un texte (« 6h », « 13h30 », « 8 h », « 14 heures », « 08:00 ») : leur début et leur fin
/// dans le texte, et les minutes depuis minuit.
fn heures_de(t: &str) -> Vec<(usize, usize, u16)> {
    let c: Vec<(usize, char)> = t.char_indices().collect();
    let pos = |k: usize| c.get(k).map_or(t.len(), |x| x.0);
    let chiffre = |k: usize| c.get(k).is_some_and(|x| x.1.is_ascii_digit());
    let mut res = Vec::new();
    let mut k = 0;
    while k < c.len() {
        if !chiffre(k) || (k > 0 && c[k - 1].1.is_alphanumeric()) {
            k += 1;
            continue;
        }
        let debut = k;
        let mut j = k;
        while chiffre(j) {
            j += 1;
        }
        if j - debut > 2 {
            k = j;
            continue;
        }
        let h: u16 = t[pos(debut)..pos(j)].parse().unwrap_or(99);
        // l'espace avant « h » est permis (« 8 h »), pas avant « : »
        let mut s = j;
        if c.get(s).is_some_and(|x| x.1 == ' ') && c.get(s + 1).is_some_and(|x| matches!(x.1, 'h' | 'H')) {
            s += 1;
        }
        let sep = c.get(s).map(|x| x.1);
        if !matches!(sep, Some('h' | 'H' | ':')) {
            k = j;
            continue;
        }
        s += 1;
        if sep != Some(':') {
            let suite = t[pos(s)..].to_lowercase();
            if suite.starts_with("eures") {
                s += 5;
            } else if suite.starts_with("eure") {
                s += 4;
            }
        }
        let mut m = s;
        if sep != Some(':') && c.get(m).is_some_and(|x| x.1 == ' ') && chiffre(m + 1) && chiffre(m + 2) {
            m += 1;
        }
        let mut e = m;
        while chiffre(e) {
            e += 1;
        }
        let (minutes, fin) = match e - m {
            2 => (t[pos(m)..pos(e)].parse::<u16>().unwrap_or(99), e),
            0 if sep != Some(':') => (0, s),
            _ => {
                k = j;
                continue;
            }
        };
        if c.get(fin).is_some_and(|x| x.1.is_alphabetic()) || h > 24 || minutes > 59 || h * 60 + minutes > 24 * 60 {
            k = j;
            continue;
        }
        res.push((pos(debut), pos(fin), h * 60 + minutes));
        k = fin;
    }
    res
}

// Les outils --------------------------------------------------------------------------------------------------

/// Les listes sans vide ni doublon, les plages lues dans les horaires.
fn finir(f: &mut Fiche) {
    for liste in [
        &mut f.missions,
        &mut f.obligatoire,
        &mut f.un_plus,
        &mut f.savoir_faire,
        &mut f.qualites,
        &mut f.horaires,
        &mut f.avantages,
    ] {
        let mut vus = std::collections::HashSet::new();
        liste.retain(|x| !x.is_empty() && vus.insert(normaliser(x)));
    }
    let mut plages: Vec<Plage> = f.horaires.iter().flat_map(|h| plages_de(h)).collect();
    plages.extend(f.conditions_texte.lines().flat_map(plages_de));
    plages.sort_by_key(|p| (p.debut, p.fin));
    plages.dedup();
    f.plages = plages;
}

/// Ajoute une ligne à un texte : un retour, ou une ligne vide si c'est un nouveau paragraphe.
fn ajouter(s: &mut String, ligne: &str, paragraphe: bool) {
    let ligne = ligne.trim();
    if ligne.is_empty() {
        return;
    }
    if !s.is_empty() {
        s.push_str(if paragraphe { "\n\n" } else { "\n" });
    }
    s.push_str(ligne);
}

fn ajouter_lignes<S: AsRef<str>>(s: &mut String, lignes: impl IntoIterator<Item = S>) {
    for l in lignes {
        ajouter(s, l.as_ref(), false);
    }
}

/// Un élément de liste sans les « : », « ; », « , » ni le point final.
fn propre(t: &str) -> String {
    let t = t.trim().trim_end_matches([':', ';', ',']).trim_end();
    let t = if t.ends_with('.') && !t.ends_with("..") { &t[..t.len() - 1] } else { t };
    t.trim().to_string()
}

/// La première lettre en minuscule, sauf pour un sigle (« CDI »).
fn minuscule(t: &str) -> String {
    let mut c = t.chars();
    match (c.next(), c.next()) {
        (Some(_), Some(b)) if b.is_uppercase() => t.to_string(),
        (Some(a), _) => a.to_lowercase().chain(t[a.len_utf8()..].chars()).collect(),
        _ => String::new(),
    }
}

fn mots(t: &str) -> usize {
    t.split_whitespace().count()
}

/// La ligne est-elle le titre de l'offre (sans tenir compte de « H/F ») ?
fn meme_titre(ligne: &str, titre: &str) -> bool {
    fn sans_hf(t: &str) -> String {
        let mut m: Vec<&str> = t.split(' ').collect();
        while m.last().is_some_and(|x| matches!(*x, "h" | "f" | "hf" | "fh" | "x" | "m")) {
            m.pop();
        }
        m.join(" ")
    }
    let t = sans_hf(&normaliser(titre));
    !t.is_empty() && sans_hf(&normaliser(ligne)) == t
}

/// Les marques de puce reconnues en tête de ligne.
const PUCES: [char; 10] = ['•', '·', '▪', '◦', '►', '▸', '→', '✓', '\u{2013}', '\u{2014}'];

/// La ligne sans sa marque de puce (« - », « -Vous », « • »...), et si elle en avait une.
fn puce(ligne: &str) -> (bool, &str) {
    let l = ligne.trim();
    if let Some(r) = l.strip_prefix(PUCES) {
        return (true, r.trim());
    }
    for m in ["- ", "* ", "+ "] {
        if let Some(r) = l.strip_prefix(m) {
            return (true, r.trim());
        }
    }
    // « -Vous êtes manuel(le) » : un tiret collé à une lettre
    if let Some(r) = l.strip_prefix('-')
        && r.chars().next().is_some_and(char::is_alphabetic)
    {
        return (true, r.trim());
    }
    (false, l)
}

fn exigence_du_mot(mot: &str) -> Option<Exigence> {
    match normaliser(mot).as_str() {
        "exige" | "exigee" | "exiges" | "exigees" => Some(Exigence::Exige),
        "souhaite" | "souhaitee" | "souhaites" | "souhaitees" => Some(Exigence::Souhaite),
        _ => None,
    }
}

/// Les mots qui, juste avant « souhaitée », en font une phrase : rien n'est retiré.
const AVANT_PHRASE: [&str; 13] = [
    "est",
    "sont",
    "serait",
    "seraient",
    "sera",
    "seront",
    "etre",
    "fortement",
    "vivement",
    "tres",
    "vous",
    "nous",
    "on",
];

/// « Exigé » ou « Souhaité » en fin de ligne, retiré du texte (« B - Véhicule léger Exigé », « Anglais (souhaité) »,
/// « 2 ans - Exigé »). Une phrase (« une expérience est souhaitée ») reste telle quelle.
fn exigence_fin(t: &str) -> (String, Option<Exigence>) {
    let rien = (t.trim().to_string(), None);
    let s = t.trim_end_matches(|c: char| c == '.' || c == '!' || c.is_whitespace());
    let (avant, e) = if let Some(dedans) = s.strip_suffix(')') {
        let Some(i) = dedans.rfind('(') else { return rien };
        let Some(e) = exigence_du_mot(&dedans[i + 1..]) else { return rien };
        (&dedans[..i], e)
    } else {
        let Some(i) = s.char_indices().rev().take_while(|(_, c)| c.is_alphabetic()).last().map(|(i, _)| i) else {
            return rien;
        };
        let Some(e) = exigence_du_mot(&s[i..]) else { return rien };
        let avant = &s[..i];
        if !avant.is_empty() && !avant.ends_with(|c: char| c.is_whitespace() || matches!(c, '-' | ':' | ',')) {
            return rien;
        }
        (avant, e)
    };
    let dernier = avant.split_whitespace().last().map(normaliser).unwrap_or_default();
    if AVANT_PHRASE.contains(&dernier.as_str()) {
        return rien;
    }
    let reste = avant.trim_end().trim_end_matches(['-', ':', ',', '\u{2013}']).trim_end();
    if reste.is_empty() {
        return rien;
    }
    (reste.to_string(), Some(e))
}

#[cfg(test)]
mod tests;
