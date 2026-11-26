//! Ce que l'onglet Offres montre de « Mes offres gardées » (étape 4 du plan), dessiné par ui\jobmind.js. Depuis la
//! 0.2.1, la même disposition que les offres d'un site : à gauche la recherche gardée choisie et les filtres (le texte
//! cherché dans tout le texte des offres gardées, le lieu, le contrat, le télétravail, toutes / favoris / plus tard /
//! hors sujet / retirées, « Aucun diplôme requis », « Débutant accepté »), à droite les offres en cartes sur toute la
//! largeur, par pages de `CARTES_PAR_PAGE` (« Page 2 sur 7 »), dans la même forme que les cartes d'un site
//! (`coulisse::Coulisse::cartes`) : la page les dessine avec la même fonction. Un double clic ouvre l'offre en grand
//! (son texte gardé sur ce PC, ses sources et « Voir sur le site », favori, plus tard, « Retirer de ma liste », « J'ai
//! postulé »), avec « Retour à la liste », « Offre précédente » et « Offre suivante » ; ou « Mes recherches ». Les
//! offres gardées avant le retrait d'Adzuna (20/11/2026) restent, avec « Voir sur Adzuna ».
//!
//! Étape 6 : chaque carte a sa note (« Très proche », « Proche », « Un peu », « Hors sujet ») dans son bandeau du haut,
//! jamais dans ses étiquettes, et sa raison en une ligne ; « Déjà vue » sur une offre déjà ouverte ; l'offre ouverte
//! dit le diplôme et l'expérience demandés avec leur preuve (le bout de phrase de l'offre) et, seulement quand ce
//! n'est pas une simple lecture de Jobmind, qui les a trouvés (« déduit par Jobmind » ou « lu par Claude ») ; le tri,
//! la rubrique « Hors sujet » et ce que Claude peut lire (ou la clé qui lui manque).
//!
//! Tous les textes sont écrits ici, en français simple, sans tiret long ni demi-cadratin ; la page les échappe. Les
//! listes des départements et des régions ne sont pas répétées : la page les prend dans Mon profil (« Ce que je
//! cherche »). Aucune adresse d'offre n'est envoyée à la page : « Voir sur le site » la demande à l'appli.

use jobmind_core::Jour;
use jobmind_core::fiche::{self as rangement, Fiche};
use jobmind_core::lecture::{Confiance, Diplome, Experience, LuPar};
use jobmind_core::offre::{Offre, TeleOffre, noms_sources};
use jobmind_core::pertinence::Niveau;
use jobmind_core::recherche::{Contrat, GenreLieu, Lieu, RAYONS, Teletravail};
use jobmind_core::recherches::NOMBRE_MAX;
use jobmind_storage::offres::OffreComplete;
use serde_json::{Value, json};

use crate::ecran::{AUCUNE_OFFRE, OFFRES};
use crate::offres::{CARTES_PAR_PAGE, VueOffres, nom_source};
use jobmind_storage::lecture::TEXTE_MIN_IA;

/// Les textes de l'onglet (la page les pose tels quels), en objet.
fn textes(paires: &[(&str, &str)]) -> Value {
    Value::Object(paires.iter().map(|(k, v)| ((*k).to_string(), json!(v))).collect())
}

/// « Cariste, autour de Metz (30 km) », « Vendeur, en Moselle », « Agent de tri, partout en France ».
pub fn lieu_texte(l: &Lieu) -> String {
    match l {
        // (une ville prise dans les propositions, « Sarrebourg (57400) » : le nom seul, pour éviter deux parenthèses)
        Lieu::Ville { nom, rayon } => format!("autour de {} ({rayon} km)", jobmind_sources::communes::nom_seul(nom)),
        Lieu::Departement(d) => format!("dans le {}", d.libelle()),
        Lieu::Region(r) => format!("en {}", r.nom),
        Lieu::Partout => "partout en France".into(),
    }
}

/// Ce que cherche une recherche gardée : « agent de quai, autour de Metz (30 km) » ; sans mots, « Tous les métiers,
/// autour de Sarrebourg (20 km) ».
pub fn recherche_texte(mots: &str, l: &Lieu) -> String {
    if mots.trim().is_empty() {
        format!("Tous les métiers, {}", lieu_texte(l))
    } else {
        format!("{mots}, {}", lieu_texte(l))
    }
}

/// « Entrepôts du Val de Moselle, Ennery » (sans entreprise : le lieu seul).
fn sous(o: &Offre) -> String {
    let lieu = o.lieu.split(',').next().unwrap_or_default().trim();
    match (o.entreprise.is_empty(), lieu.is_empty()) {
        (false, false) => format!("{}, {lieu}", o.entreprise),
        (false, true) => o.entreprise.clone(),
        (true, false) => lieu.to_string(),
        (true, true) => "Entreprise non précisée".into(),
    }
}

fn noms(o: &Offre) -> String {
    noms_sources(&o.sources.iter().map(|s| nom_source(s)).collect::<Vec<_>>())
}

/// 0.2.1 : la carte d'une offre gardée, dans la même forme que la carte d'une offre d'un site (la page les dessine
/// avec la même fonction) : `i` est son numéro ; la note va dans le bandeau du haut (`proche`), jamais dans les
/// étiquettes ; `vue` : « Déjà vue », une offre déjà ouverte ; `retiree` : retirée de ma liste, montrée en gris dans
/// « Retirées » avec « Remettre ».
fn carte(o: &Offre, ouverte: Option<i64>, aujourdhui: Jour) -> Value {
    let etiquette = |nom: &str, bon: bool, genre: &str| json!({ "nom": nom, "bon": bon, "genre": genre });
    let mut etiquettes = Vec::new();
    // (ce que Jordan a fait de l'offre, d'abord)
    if o.candidature.is_some() {
        etiquettes.push(etiquette("Postulé", true, "marque"));
    }
    if o.favori {
        etiquettes.push(etiquette("Favori", false, "marque"));
    }
    if o.plus_tard {
        etiquettes.push(etiquette("Plus tard", false, "marque"));
    }
    // (puis ce que demande l'offre, comme sur la carte d'un site)
    if let Some(d) = o.diplome {
        etiquettes.push(etiquette(d.nom(), d == Diplome::Aucun, "lecture"));
    }
    if let Some(e) = o.experience {
        etiquettes.push(etiquette(&e.nom(), e == Experience::Debutant, "lecture"));
    }
    if let Some(c) = o.contrat {
        etiquettes.push(etiquette(c.nom(), false, "contrat"));
    }
    match o.teletravail {
        Some(TeleOffre::Partiel) => etiquettes.push(etiquette("Télétravail possible", false, "tele")),
        Some(TeleOffre::Complet) => etiquettes.push(etiquette("Télétravail complet", false, "tele")),
        None => {}
    }
    let date = match o.publiee_le {
        Some(j) => format!("le {}", j.court(aujourdhui)),
        None => format!("gardée le {}", o.ajoutee_le.court(aujourdhui)),
    };
    let mut carte = json!({
        "i": o.id,
        "titre": o.titre,
        "sous": o.entreprise,
        "lieu": o.lieu.split(',').next().unwrap_or_default().trim(),
        "l3": format!("{date} · sur {}", noms(o)),
        "raison": o.raison,
        "sel": Some(o.id) == ouverte,
        "etiquettes": etiquettes,
        "etat": "lue",
        "retiree": o.cachee,
        "vue": o.vue && !o.cachee,
    });
    // (le bandeau du haut : la note en mots ; le champ n'existe que sur une carte qui a sa note)
    if let Some(n) = o.niveau().filter(|_| !o.cachee) {
        let niveau = match n {
            Niveau::TresProche => "tres",
            Niveau::Proche => "proche",
            Niveau::UnPeu => "peu",
            Niveau::HorsSujet => "hors",
        };
        carte["proche"] = json!({ "niveau": niveau, "nom": n.nom() });
    }
    carte
}

/// Ce qui a été lu, pour l'offre ouverte : la valeur, la preuve et qui l'a trouvé (0.2.1 : plus de « lu par Jobmind »,
/// Jordan n'en veut plus : rien quand c'est une simple lecture de Jobmind ; « déduit par Jobmind » et « lu par
/// Claude » restent).
fn lu_vue(nom: &str, lu: Option<(String, &str, LuPar, Confiance)>, absent: &str) -> Value {
    match lu {
        Some((valeur, preuve, par, confiance)) => json!({
            "nom": nom,
            "valeur": valeur,
            "preuve": preuve,
            "par": match (par, confiance) {
                (LuPar::Regles, Confiance::Moyenne) => "déduit par Jobmind",
                (LuPar::Regles, _) => "",
                (LuPar::Ia, _) => "lu par Claude",
            },
            "dit": true,
        }),
        None => json!({
            "nom": nom,
            "valeur": absent,
            "preuve": "",
            "par": "",
            "dit": false,
        }),
    }
}

/// 0.1.5 : le texte d'une offre rangé pour la fiche « En bref d'abord », pour une offre gardée comme pour une offre
/// lue sur un site : `(bref, rangee)`. `bref` : 0 à 4 cases `{ nom, valeur, sous }` (salaire, temps, horaires,
/// contrat), seulement celles que l'offre dit ; le salaire et le contrat de l'offre servent si le texte n'en dit rien.
/// `rangee` : les champs de jobmind_core::fiche::Fiche, et la carte de l'entreprise (crate::entreprise::carte) dans
/// `"carte"` s'il y en a une. Aucune adresse n'y est mise.
pub fn fiche_b_vue(f: &Fiche, salaire: &str, contrat: Option<Contrat>, carte: Option<Value>) -> (Value, Value) {
    let case = |nom: &str, (valeur, sous): (String, String)| json!({ "nom": nom, "valeur": valeur, "sous": sous });
    let mut bref = Vec::new();
    let salaire = if f.salaire.is_empty() { salaire.trim() } else { f.salaire.as_str() };
    if !salaire.is_empty() {
        bref.push(case("Salaire", couper_apres(salaire, "€")));
    }
    if !f.temps.is_empty() {
        bref.push(case("Temps", couper_avant(&f.temps, " par ")));
    }
    if let Some(h) = horaires_bref(f) {
        bref.push(case("Horaires", h));
    }
    if !f.contrat.is_empty() {
        bref.push(case("Contrat", couper_avant(&f.contrat, " de ")));
    } else if let Some(c) = contrat {
        bref.push(case("Contrat", (c.nom().to_string(), String::new())));
    }
    let plages: Vec<Value> = f.plages.iter().map(|p| json!({ "debut": p.debut, "fin": p.fin })).collect();
    let mut rangee = json!({
        "intro": f.intro, "missions": f.missions, "lieu": f.lieu, "poste_texte": f.poste_texte,
        "obligatoire": f.obligatoire, "un_plus": f.un_plus, "experience": f.experience,
        "qualification": f.qualification, "savoir_faire": f.savoir_faire, "qualites": f.qualites,
        "demande_texte": f.demande_texte,
        "salaire": f.salaire, "temps": f.temps, "horaires": f.horaires, "plages": plages, "contrat": f.contrat,
        "avantages": f.avantages, "conditions_texte": f.conditions_texte,
        "entreprise": f.entreprise, "secteur": f.secteur, "effectif": f.effectif,
        "entreprise_texte": f.entreprise_texte,
    });
    if let Some(c) = carte {
        rangee["carte"] = c;
    }
    (Value::Array(bref), rangee)
}

/// 0.1.5 : les infos de l'en-tête sans celles qu'une case « En bref » montre déjà (le contrat, le salaire).
pub fn sans_doublon(infos: Vec<Value>, bref: &Value) -> Vec<Value> {
    let cases = bref.as_array().map_or(&[][..], Vec::as_slice);
    infos.into_iter().filter(|i| !cases.iter().any(|c| c["nom"] == i["nom"])).collect()
}

/// « 12,31 € brut de l'heure » : (« 12,31 € », « brut de l'heure ») ; sans la marque, tout dans la valeur.
fn couper_apres(t: &str, marque: &str) -> (String, String) {
    match t.find(marque) {
        Some(i) => {
            let (a, b) = t.split_at(i + marque.len());
            (a.trim().to_string(), b.trim().to_string())
        }
        None => (t.trim().to_string(), String::new()),
    }
}

/// « CDD de 3 mois » : (« CDD », « 3 mois ») ; « 35 h par semaine » : (« 35 h », « par semaine »).
fn couper_avant(t: &str, marque: &str) -> (String, String) {
    match t.find(marque) {
        Some(i) => {
            let sous = t[i..].trim();
            let sous = sous.strip_prefix("de ").unwrap_or(sous);
            (t[..i].trim().to_string(), sous.to_string())
        }
        None => (t.trim().to_string(), String::new()),
    }
}

/// La case des horaires. Quand des heures sont lues (les plages), toujours elles : la première en valeur (« 7 h à
/// 12 h »), les suivantes dessous (« et 13 h à 16 h »), sinon le premier horaire écrit sans heure (« en journée »).
/// Sans heure : le premier horaire en court (« Travail en journée » : « En journée »), les suivants dessous ; le
/// dessous est coupé vers 40 caractères.
fn horaires_bref(f: &Fiche) -> Option<(String, String)> {
    fn court(h: &str) -> String {
        let mut h = h.trim();
        for p in ["Travail ", "travail ", "Horaires : ", "Horaires: ", "Horaire : ", "Horaire: "] {
            h = h.strip_prefix(p).unwrap_or(h);
        }
        let mut c = h.chars();
        c.next().map_or_else(String::new, |p| p.to_uppercase().chain(c).collect())
    }
    fn a_des_heures(h: &str) -> bool {
        let b = h.to_lowercase().into_bytes();
        b.windows(2).any(|w| w[0].is_ascii_digit() && (w[1] == b'h' || w[1] == b':'))
            || b.windows(3).any(|w| w[0].is_ascii_digit() && w[1] == b' ' && w[2] == b'h')
    }
    fn heure(m: u16) -> String {
        if m.is_multiple_of(60) { format!("{} h", m / 60) } else { format!("{} h {:02}", m / 60, m % 60) }
    }
    let coupe = |s: String| {
        if s.chars().count() > 40 { format!("{}…", s.chars().take(39).collect::<String>().trim_end()) } else { s }
    };
    let plages: Vec<String> = f.plages.iter().map(|p| format!("{} à {}", heure(p.debut), heure(p.fin))).collect();
    if let Some((premiere, autres)) = plages.split_first() {
        let sous = if autres.is_empty() {
            f.horaires.iter().find(|h| !a_des_heures(h)).map(|h| court(h).to_lowercase()).unwrap_or_default()
        } else {
            format!("et {}", autres.join(", "))
        };
        return Some((premiere.clone(), coupe(sous)));
    }
    let lignes: Vec<String> = f.horaires.iter().map(|h| court(h)).collect();
    let (premier, reste) = lignes.split_first()?;
    let sous: Vec<String> = reste.iter().map(|h| h.to_lowercase()).collect();
    Some((premier.clone(), coupe(sous.join(", "))))
}

/// L'offre ouverte (`annuaire` : ce que l'annuaire officiel dit de son entreprise).
fn fiche(x: &OffreComplete, aujourdhui: Jour, claude: bool, annuaire: Option<&crate::entreprise::Connu>) -> Value {
    let o = &x.offre;
    let l = &x.lecture;
    // (le texte d'une offre gardée d'Adzuna n'est qu'un résumé de l'annonce : ce qu'il ne dit pas, l'annonce complète
    // peut le dire)
    let absent = if x.description_de == "adzuna" { "Pas dans le résumé d'Adzuna" } else { "Non dit dans l'offre" };
    let mut lecture = vec![
        lu_vue(
            "Diplôme",
            l.diplome.as_ref().map(|d| (d.valeur.nom().to_string(), d.preuve.as_str(), d.par, d.confiance)),
            absent,
        ),
        lu_vue(
            "Expérience",
            l.experience.as_ref().map(|e| (e.valeur.nom(), e.preuve.as_str(), e.par, e.confiance)),
            absent,
        ),
    ];
    if !l.autres.is_empty() {
        lecture.push(json!({ "nom": "Permis et CACES", "valeur": l.autres, "preuve": "", "par": "", "dit": true }));
    }
    // (Claude peut lire cette offre : quelque chose n'est pas dit, jamais demandé, et assez de texte)
    let floue = l.lue
        && (l.diplome.is_none() || l.experience.is_none())
        && !l.par_ia
        && x.description.chars().count() >= TEXTE_MIN_IA
        && !o.rangee_hors_sujet();
    let ia = match (floue, claude) {
        (false, _) => None,
        (true, true) => Some("lire"),
        (true, false) => Some("cle"),
    };
    let pas_dit = "Non précisé";
    let tele = match o.teletravail {
        Some(TeleOffre::Partiel) => "Possible",
        Some(TeleOffre::Complet) => "Complet",
        None => pas_dit,
    };
    // (0.1.5 : le texte rangé pour la fiche « En bref d'abord » ; les horaires sont dans ses cases)
    let rangee = rangement::ranger(&o.titre, &x.description);
    let nom = crate::entreprise::nom_offre(&o.entreprise, &o.titre, &x.description);
    let (bref, rangee) = fiche_b_vue(
        &rangee,
        &o.salaire,
        o.contrat,
        crate::entreprise::carte(&nom, &o.lieu, &x.site_entreprise, annuaire),
    );
    let infos = sans_doublon(
        vec![
            json!({ "nom": "Contrat", "valeur": o.contrat.map_or(pas_dit, Contrat::nom) }),
            json!({ "nom": "Salaire", "valeur": if o.salaire.is_empty() { pas_dit.to_string() } else { o.salaire.clone() } }),
            json!({ "nom": "Télétravail", "valeur": tele }),
            json!({ "nom": "Publiée", "valeur": o.publiee_le.map_or("Non précisée".to_string(), |j| format!("le {}", j.court(aujourdhui))) }),
        ],
        &bref,
    );
    let liens: Vec<Value> = x
        .liens
        .iter()
        .filter(|l| !l.url.is_empty())
        .map(|l| {
            let nom = if l.source == "site" {
                "Voir sur le site".into()
            } else {
                format!("Voir sur {}", nom_source(&l.source))
            };
            json!({ "source": l.source, "nom": nom })
        })
        .collect();
    let gardee = match x.retrouvee_le {
        Some(r) if r != o.ajoutee_le => format!(
            "Gardée sur ce PC depuis le {} (revue sur le site le {}) : elle reste ici même si elle disparaît du site.",
            o.ajoutee_le.court(aujourdhui),
            r.court(aujourdhui)
        ),
        _ => format!(
            "Gardée sur ce PC depuis le {} : elle reste ici même si elle disparaît du site.",
            o.ajoutee_le.court(aujourdhui)
        ),
    };
    let texte_de = nom_source(&x.description_de);
    json!({
        "id": o.id,
        "titre": o.titre,
        "sous": sous(o),
        "sources": match x.venue_par_mail {
            // (étape 7 : venue d'une alerte mail)
            Some(j) => format!("Trouvée sur {}, venue d'une alerte mail le {}", noms(o), j.court(aujourdhui)),
            None => format!("Trouvée sur {}", noms(o)),
        },
        "alerte_mail": x.venue_par_mail.is_some(),
        "double": o.sources.len() > 1,
        "infos": infos,
        "texte": x.description,
        "bref": bref,
        "rangee": rangee,
        "texte_vide": match (x.venue_par_mail, x.liens.iter().find(|l| !l.url.is_empty())) {
            (Some(_), Some(l)) => format!(
                "Le mail d'alerte ne donne que le titre. Clique « Voir sur {} », puis « Ajouter à Jobmind » dans la \
                 petite fenêtre : Jobmind gardera le texte entier de l'offre.",
                nom_source(&l.source)
            ),
            _ => "Le site n'a donné aucun texte pour cette offre. Lis-la sur le site.".to_string(),
        },
        // (une source retirée, Adzuna ou Jooble : seulement le début ; une page lue : le texte entier)
        "note": match x.description_de.as_str() {
            "site" => "Le texte entier de l'annonce, trouvé sur la page du site.".to_string(),
            "linkedin" | "indeed" | "wttj" | "francetravail" => {
                format!("Le texte entier de l'annonce, trouvé sur la page de {texte_de}.")
            }
            "adzuna" => format!("{texte_de} ne donne que le début de l'annonce : le texte entier est sur le site."),
            _ => "La source ne donne que le début de l'annonce : le texte entier est sur le site.".to_string(),
        },
        "gardee": gardee,
        "liens": liens,
        "favori": o.favori,
        "plus_tard": o.plus_tard,
        "cachee": o.cachee,
        "candidature": o.candidature,
        // (étape 5 : « 34 candidats sur LinkedIn, vu le 18 nov. », ce que la page montrait au dernier « Ajouter à
        // Jobmind »)
        "candidats": x.candidats.as_ref().map(|c| {
            format!("{} sur {}, vu le {}", c.texte, nom_source(&c.site), c.vu_le.court(aujourdhui))
        }),
        // (étape 6)
        "note": o.niveau().map(|n| json!({ "code": n.code(), "nom": n.nom(), "raison": o.raison })),
        "lecture": lecture,
        "hors_sujet": o.hors_sujet,
        "pas_hors_sujet": o.pas_hors_sujet,
        "ia": ia,
    })
}

/// Le panneau de droite quand il n'y a pas d'offre à montrer : ce qui manque, et le bouton pour avancer.
fn vide(v: &VueOffres) -> Value {
    let (icone, titre, texte, bouton, note): (&str, String, &str, &str, &str) = if v.recherches.is_empty() {
        (
            "cible",
            "Aucune recherche pour l'instant".into(),
            "Une recherche, c'est ce que tu cherches et où. Ajoute la première ici, ou dans « Ce que je cherche ».",
            "nouvelle",
            "",
        )
    } else if v.gardees == 0 && v.cachees == 0 {
        (
            "loupe",
            AUCUNE_OFFRE.into(),
            "Choisis France Travail dans « Où chercher », en haut à gauche, puis « Chercher sur France Travail ». \
             Chaque offre que tu ajoutes avec « Ajouter à Jobmind » est gardée ici.",
            "chercher",
            "Chaque offre gardée reste lisible ici, même si elle disparaît du site.",
        )
    } else {
        (
            "loupe",
            "Aucune offre ne correspond".into(),
            "Essaie un rayon plus large, ou retire un filtre.",
            if v.filtres_actifs || !matches!(v.saisie.liste.as_str(), "" | "toutes") { "filtres" } else { "" },
            "",
        )
    };
    json!({ "icone": icone, "titre": titre, "texte": texte, "bouton": bouton, "note": note })
}

/// Mes recherches (le panneau de droite « Gérer ») : chaque recherche avec ce qu'elle cherche.
fn gestion(v: &VueOffres) -> Value {
    let l: Vec<Value> = v
        .recherches
        .iter()
        .map(|r| {
            let (genre, ville, rayon, dep, reg) = match &r.lieu {
                Lieu::Ville { nom, rayon } => (GenreLieu::Ville, nom.as_str(), *rayon, "", ""),
                Lieu::Departement(d) => (GenreLieu::Departement, "", 30, d.code, ""),
                Lieu::Region(x) => (GenreLieu::Region, "", 30, "", x.code),
                Lieu::Partout => (GenreLieu::Partout, "", 30, "", ""),
            };
            json!({
                "id": r.id,
                "nom": r.nom,
                "sous": recherche_texte(&r.mots, &r.lieu),
                "valeurs": { "nom": r.nom, "mots": r.mots, "lieu": genre.code(), "ville": ville, "rayon": rayon,
                             "departement": dep, "region": reg },
                "question": format!("Supprimer la recherche « {} » ? Les offres qu'elle a trouvées restent gardées.", r.nom),
            })
        })
        .collect();
    json!({
        "titre": "Mes recherches",
        "intro": "Chaque recherche dit ce que tu cherches et où. Choisis-en une, puis « Chercher sur France Travail » \
                  (ou un autre site) dans « Où chercher ».",
        "liste": l,
        "plein": v.recherches.len() >= NOMBRE_MAX,
        "profil": false,
    })
}

/// L'onglet Offres.
pub fn onglet(v: &VueOffres, aujourdhui: Jour) -> Value {
    let ouverte = v.ouverte.as_ref().map(|o| o.offre.id);
    let choisie = v.recherche.and_then(|id| v.recherches.iter().find(|r| r.id == id));
    let toutes = matches!(v.saisie.liste.as_str(), "" | "toutes");
    let compte = if v.filtres_actifs || !toutes {
        format!("{} sur {}", v.total, v.gardees + if v.saisie.liste == "cachees" { v.cachees } else { 0 })
    } else {
        match v.total {
            0 => "Aucune offre gardée".to_string(),
            1 => "1 offre gardée".to_string(),
            n => format!("{n} offres gardées"),
        }
    };
    // (0.2.1 : « Page 2 sur 7 · offres 25 à 48 », comme pour un site ; rien sans carte)
    let page_texte = if v.liste.is_empty() {
        String::new()
    } else {
        format!("Page {} sur {} · offres {} à {}", v.page, v.pages.max(1), v.debut + 1, v.debut + v.liste.len())
    };
    let lieux: Vec<Value> = GenreLieu::TOUS
        .iter()
        .map(|g| {
            let nom = match g {
                GenreLieu::Partout => "Tous les lieux",
                GenreLieu::Ville => "Autour d'une ville",
                GenreLieu::Departement => "Un département",
                GenreLieu::Region => "Une région",
            };
            json!({ "id": g.code(), "nom": nom })
        })
        .collect();
    let mut contrats = vec![json!({ "id": "", "nom": "Tous les contrats" })];
    contrats.extend(Contrat::TOUS.iter().map(|c| json!({ "id": c.code(), "nom": c.nom() })));
    let teles: Vec<Value> = Teletravail::TOUS
        .iter()
        .map(|t| {
            let nom = match t {
                Teletravail::PeuImporte => "Télétravail : peu importe",
                Teletravail::Possible => "Télétravail possible",
                Teletravail::Complet => "Seulement en télétravail",
            };
            json!({ "id": t.code(), "nom": nom })
        })
        .collect();
    let listes = json!([
        { "id": "toutes", "nom": "Toutes" },
        { "id": "favoris", "nom": "Favoris", "nombre": v.favoris },
        { "id": "plus_tard", "nom": "Plus tard", "nombre": v.plus_tard },
        { "id": "hors_sujet", "nom": "Hors sujet", "nombre": v.hors_sujet },
        { "id": "cachees", "nom": "Retirées", "nombre": v.cachees },
    ]);
    let tri = if v.saisie.tri == "recentes" { "recentes" } else { "pertinence" };
    let ia = json!({
        "floues": v.floues,
        "claude": v.claude,
        "en_cours": v.ia_en_cours,
        "annonce": v.ia_annonce.as_ref().map(|a| json!({ "texte": a.texte, "genre": a.genre, "cles": a.cles })),
        "texte": if v.floues == 0 {
            String::new()
        } else if v.claude {
            format!("{} que Jobmind n'a pas su lire en entier.", if v.floues == 1 { "1 offre floue".to_string() } else { format!("{} offres floues", v.floues) })
        } else {
            "Ajoute ta clé Claude pour que Jobmind lise aussi les offres floues.".to_string()
        },
    });
    json!({
        "id": OFFRES,
        "nom": "Offres",
        // (0.1.7 : aucun chiffre ici ; seul le nombre d'offres de la liste du site à l'écran en met un,
        // coulisse::chiffre_onglet. Le premier état de la page montrait encore les offres gardées jamais ouvertes.)
        "nombre": Value::Null,
        "liste": "Les offres",
        "vide": "Aucune offre gardée.",
        "rubriques": [],
        "dossier": {
            "icone": "loupe",
            "titre": AUCUNE_OFFRE,
            "texte": "Jobmind rassemble ici les offres que tu ajoutes depuis les sites, France Travail d'abord, sans \
                      doublons.",
            "note": "Chaque offre gardée reste lisible sur ce PC, même si elle disparaît du site.",
        },
        "offres": {
            "filtres": {
                "texte": v.saisie.texte,
                "genre": v.saisie.genre,
                "ville": v.saisie.ville,
                "rayon": v.saisie.rayon,
                "departement": v.saisie.departement,
                "region": v.saisie.region,
                "contrat": v.saisie.contrat,
                "teletravail": if v.saisie.teletravail.is_empty() { "peu_importe" } else { v.saisie.teletravail.as_str() },
                "liste": if v.saisie.liste.is_empty() { "toutes" } else { v.saisie.liste.as_str() },
                "sans_diplome": v.saisie.sans_diplome,
                "debutant": v.saisie.debutant,
                "tri": tri,
            },
            "tris": [
                { "id": "pertinence", "nom": "Les plus pertinentes en haut" },
                { "id": "recentes", "nom": "Les plus récentes en haut" },
            ],
            "liste_t": if tri == "pertinence" { "Les plus pertinentes en haut" } else { "Les plus récentes en haut" },
            "sans_note": if v.profil || v.gardees == 0 { "" } else { "Remplis « Ce que je cherche » dans Mon profil : Jobmind triera les offres pour toi." },
            "ia": ia,
            "actifs": v.filtres_actifs,
            "lieux": lieux,
            "rayons": RAYONS,
            "contrats": contrats,
            "teles": teles,
            "listes": listes,
            "compte": compte,
            "recherches": v.recherches.iter().map(|r| json!({ "id": r.id, "nom": r.nom })).collect::<Vec<_>>(),
            "recherche": choisie.map(|r| r.id),
            "recherche_sous": choisie.map(|r| recherche_texte(&r.mots, &r.lieu)),
            // (0.2.1 : les cartes de la page montrée, dans la forme des cartes d'un site, et leurs pages)
            "titre": "Mes offres gardées",
            "lignes": v.liste.iter().map(|o| carte(o, ouverte, aujourdhui)).collect::<Vec<_>>(),
            "par_page": CARTES_PAR_PAGE,
            "page": v.page,
            "pages": v.pages,
            "page_texte": page_texte,
            "precedente": v.page > 1,
            "suivante": v.page < v.pages,
            // (l'offre ouverte en grand, et ses voisines : « Offre précédente », « Offre suivante »)
            "fiche": v.ouverte.as_ref().map(|o| fiche(o, aujourdhui, v.claude, v.annuaire.as_ref())),
            "fiche_avant": v.avant,
            "fiche_apres": v.apres,
            "vide": vide(v),
            "gestion": gestion(v),
            "textes": textes(&[
                ("i_texte", "Chercher dans mes offres : quai, nuit, magasin..."),
                ("ville", "Ville"),
                ("i_ville", "Ville"),
                ("km", "km"),
                ("departement", "Département"),
                ("region", "Région"),
                ("choisir", "Choisis"),
                ("contrat", "Contrat"),
                ("teletravail", "Télétravail"),
                ("mes_recherches", "Ma recherche"),
                ("gerer", "Gérer mes recherches"),
                ("chercher_ft", "Chercher sur France Travail"),
                ("liste_t", "Les plus récentes en haut"),
                ("vide_l", "Aucune offre avec ces filtres."),
                ("sans_recherche", "Crée une recherche pour commencer."),
                ("ouvrir_cles", "Ouvrir Mes clés"),
                ("depuis_profil", "Créer depuis « Ce que je cherche »"),
                ("nouvelle", "Nouvelle recherche"),
                ("ajouter", "Ajouter une recherche"),
                ("retirer", "Retirer les filtres"),
                ("favori", "Favori"),
                ("plus_tard", "Plus tard"),
                ("cacher", "Retirer de ma liste"),
                ("retirer_liste", "Retirer de ma liste"),
                ("remettre_liste", "Remettre dans ma liste"),
                ("decacher", "Remettre dans ma liste"),
                ("postule", "J'ai postulé"),
                ("voir_candidature", "Voir ma candidature"),
                ("postule_deja", "Tu as noté une candidature pour cette offre."),
                ("texte_t", "L'offre"),
                ("doublon", "La même offre était sur plusieurs sites : elle n'est montrée qu'une fois."),
                ("nouvelle_l", "Nouvelle"),
                ("form_nouvelle", "Nouvelle recherche"),
                ("form_modifier", "Modifier la recherche"),
                ("nom", "Nom"),
                ("facultatif", "facultatif"),
                ("i_nom", "Par exemple : Quai autour de Metz"),
                ("mots", "Poste"),
                ("i_mots", "Par exemple : assistance informatique, cariste, vendeur"),
                ("mots_aide", "Écris un métier : Jobmind te propose les noms exacts et les métiers voisins. Vide : tous les métiers du lieu."),
                ("lieu", "Où"),
                ("rayon", "Jusqu'à"),
                ("enregistrer", "Enregistrer"),
                ("annuler", "Annuler"),
                ("modifier", "Modifier"),
                ("supprimer", "Supprimer"),
                ("oui", "Oui, supprimer"),
                ("non", "Non, garder"),
                ("fermer", "Revenir aux offres"),
                ("sans_diplome", "Aucun diplôme requis"),
                ("debutant", "Débutant accepté"),
                ("tri", "Trier les offres"),
                ("pourquoi", "Pourquoi cette note"),
                ("lu_t", "Ce que demande l'offre"),
                ("pas_hors_sujet", "Pas hors sujet"),
                ("remettre_hors_sujet", "Remettre dans Hors sujet"),
                ("hors_sujet_t", "Jobmind a rangé cette offre dans « Hors sujet ». Elle reste gardée ici."),
                ("pas_hors_sujet_t", "Tu as dit que cette offre n'est pas hors sujet."),
                ("lire_ia", "Demander à Claude"),
                ("lire_ia_t", "Jobmind n'a pas tout trouvé dans cette offre. Claude peut la lire."),
                ("lire_floues", "Lire avec Claude"),
                ("cle_ia_t", "Ajoute ta clé Claude pour que Jobmind lise aussi les offres floues."),
                ("plein", "Tu as déjà 20 recherches : supprime-en une pour en ajouter une autre."),
                // (0.2.1 : les cartes, leurs pages et l'offre ouverte en grand, avec les mots des offres d'un site)
                ("i_texte_t", "Chercher dans mes offres"),
                ("lieu_t", "Lieu"),
                ("listes_t", "Montrer"),
                ("l_retirer", "Retirer de ma liste"),
                ("l_remettre", "Remettre"),
                ("deja_vue", "Déjà vue"),
                ("precedente", "Page précédente"),
                ("suivante", "Page suivante"),
                ("double_clic", "Double clic sur une offre pour l'ouvrir en grand."),
                ("retour_liste", "Retour à la liste"),
                ("offre_avant", "Offre précédente"),
                ("offre_apres", "Offre suivante"),
            ]),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jobmind_storage::lecture::{LectureGardee, Lu};
    use jobmind_storage::offres::{Lien, RechercheGardee, VueCandidats};

    fn j(d: u32) -> Jour {
        Jour::nouveau(2026, 11, d).unwrap()
    }

    pub(crate) fn exemple() -> VueOffres {
        let o = |id, titre: &str, entreprise: &str, lieu: &str, sources: &[&str]| Offre {
            id,
            titre: titre.into(),
            entreprise: entreprise.into(),
            lieu: lieu.into(),
            contrat: Some(Contrat::Cdi),
            publiee_le: Some(j(17)),
            ajoutee_le: j(18),
            sources: sources.iter().map(|s| s.to_string()).collect(),
            ..Offre::default()
        };
        let mut a =
            o(1, "Agent de quai H/F", "Entrepôts du Val de Moselle", "Ennery, Moselle", &["adzuna", "linkedin"]);
        a.favori = true;
        a.vue = true;
        a.teletravail = Some(TeleOffre::Partiel);
        a.salaire = "21 800 à 23 500 € par an".into();
        a.pertinence = Some(92);
        a.raison = "Aucun diplôme demandé, à 12 km, débutant accepté".into();
        a.diplome = Some(Diplome::Aucun);
        a.experience = Some(Experience::Debutant);
        let b = o(2, "Agent de tri", "", "Forbach (57)", &["indeed"]);
        let r = RechercheGardee {
            id: 4,
            nom: "Agent de quai".into(),
            mots: "agent de quai".into(),
            lieu: Lieu::Ville { nom: "Metz".into(), rayon: 30 },
        };
        VueOffres {
            liste: vec![a.clone(), b],
            total: 2,
            page: 1,
            pages: 1,
            gardees: 2,
            nouvelles: 1,
            favoris: 1,
            ouverte: Some(OffreComplete {
                offre: a,
                description: "Chargement et déchargement des camions.\nDébutant accepté.".into(),
                description_de: "adzuna".into(),
                liens: vec![
                    Lien { source: "adzuna".into(), url: "https://www.adzuna.fr/land/ad/1".into() },
                    Lien { source: "linkedin".into(), url: "https://fr.linkedin.com/jobs/view/1".into() },
                ],
                retrouvee_le: Some(j(18)),
                candidats: Some(VueCandidats {
                    site: "linkedin".into(),
                    texte: "Plus de 100 candidats".into(),
                    nombre: Some(100),
                    vu_le: j(17),
                }),
                lecture: LectureGardee {
                    diplome: Some(Lu {
                        valeur: Diplome::Aucun,
                        preuve: "Sans diplôme".into(),
                        confiance: Confiance::Forte,
                        par: LuPar::Regles,
                    }),
                    experience: None,
                    autres: "Permis B".into(),
                    lue: true,
                    par_ia: false,
                },
                venue_par_mail: None,
                site_entreprise: String::new(),
            }),
            recherches: vec![r],
            recherche: Some(4),
            saisie: crate::offres::VIDE.saisie.clone(),
            profil: true,
            ..VueOffres::default()
        }
    }

    #[test]
    fn l_onglet_rempli() {
        let v = exemple();
        let e = onglet(&v, j(18));
        let o = &e["offres"];
        // (aucun chiffre sur l'onglet tant qu'aucune liste d'un site n'est à l'écran, même au premier état)
        assert!(e["nombre"].is_null());
        assert_eq!(o["compte"], "2 offres gardées");
        assert_eq!(o["recherche_sous"], "agent de quai, autour de Metz (30 km)");
        let l = o["lignes"].as_array().unwrap();
        // (0.2.1 : une carte, dans la forme des cartes d'un site : son numéro dans `i`, l'entreprise, le lieu, la date
        // et les sites ; le contrat est une étiquette)
        assert_eq!(l[0]["i"], 1);
        assert_eq!((&l[0]["sous"], &l[0]["lieu"]), (&json!("Entrepôts du Val de Moselle"), &json!("Ennery")));
        assert_eq!(l[0]["l3"], "le 17 nov. · sur Adzuna et LinkedIn");
        assert_eq!(l[0]["sel"], true);
        assert_eq!((&l[1]["sous"], &l[1]["lieu"], &l[1]["sel"]), (&json!(""), &json!("Forbach (57)"), &json!(false)));
        assert_eq!(o["titre"], "Mes offres gardées");
        let f = &o["fiche"];
        assert_eq!(f["sous"], "Entrepôts du Val de Moselle, Ennery");
        assert_eq!(f["sources"], "Trouvée sur Adzuna et LinkedIn");
        assert_eq!(f["double"].as_bool(), Some(true));
        // (une offre gardée avant le retrait d'Adzuna : son lien vers l'annonce reste)
        assert_eq!(f["liens"][0]["nom"], "Voir sur Adzuna");
        assert!(f.get("adzuna").is_none());
        assert_eq!(f["liens"][1]["nom"], "Voir sur LinkedIn");
        assert_eq!(f["candidats"], "Plus de 100 candidats sur LinkedIn, vu le 17 nov.");
        // (0.1.5 : le salaire dans sa case « En bref », plus dans les infos)
        assert_eq!(f["bref"][0], json!({ "nom": "Salaire", "valeur": "21 800 à 23 500 €", "sous": "par an" }));
        let tele = f["infos"].as_array().unwrap().iter().find(|i| i["nom"] == "Télétravail").unwrap();
        assert_eq!(tele["valeur"], "Possible");
        assert_eq!(f["gardee"], "Gardée sur ce PC depuis le 18 nov. : elle reste ici même si elle disparaît du site.");
        // Aucune adresse d'offre dans l'état
        assert!(!e.to_string().contains("https://"), "{e}");
        assert_eq!(o["gestion"]["liste"][0]["valeurs"]["lieu"], "ville");
        // Une ville prise dans les propositions : une seule parenthèse
        assert_eq!(
            lieu_texte(&Lieu::Ville { nom: "Sarrebourg (57400)".into(), rayon: 30 }),
            "autour de Sarrebourg (30 km)"
        );
    }

    /// Règle de la suite : aucun tiret long ni demi-cadratin, aucun HTML, pas de double espace ; les noms que le kit
    /// remet en liste (`LISTES` de kit.js) n'y sont jamais des textes.
    #[test]
    fn textes_simples() {
        fn textes_de(v: &Value, l: &mut Vec<String>) {
            match v {
                Value::String(s) => l.push(s.clone()),
                Value::Array(a) => a.iter().for_each(|x| textes_de(x, l)),
                Value::Object(o) => o.values().for_each(|x| textes_de(x, l)),
                _ => {}
            }
        }
        let mut cas = vec![exemple(), crate::offres::VIDE.clone()];
        cas.push(VueOffres { profil: true, ..crate::offres::VIDE.clone() });
        for v in &cas {
            let e = onglet(v, j(18));
            let mut t = Vec::new();
            textes_de(&e, &mut t);
            for t in t {
                assert!(!t.contains(['\u{2013}', '\u{2014}', '<', '>']), "{t}");
                assert!(!t.contains("  "), "{t}");
            }
            for nom in ["options", "choix", "actions", "boutons", "resume", "quand", "chemin", "points", "sections"] {
                assert!(!e.to_string().contains(&format!("\"{nom}\":")), "{nom}");
            }
        }
    }

    /// Étape 6 : la pastille, la raison, ce que demande l'offre avec sa preuve, Claude (clé ou non), le tri.
    #[test]
    fn la_lecture_et_la_note_montrees() {
        let mut v = exemple();
        let e = onglet(&v, j(18));
        let o = &e["offres"];
        // (0.2.1 : la note est dans le bandeau du haut de la carte, jamais dans ses étiquettes du bas)
        assert_eq!(o["lignes"][0]["proche"], json!({ "niveau": "tres", "nom": "Très proche" }));
        assert!(o["lignes"][0].get("note").is_none());
        assert_eq!(o["lignes"][0]["raison"], "Aucun diplôme demandé, à 12 km, débutant accepté");
        assert!(o["lignes"][1].get("proche").is_none());
        let noms = |c: &Value| -> Vec<String> {
            c["etiquettes"].as_array().unwrap().iter().map(|e| e["nom"].as_str().unwrap().to_string()).collect()
        };
        assert_eq!(
            noms(&o["lignes"][0]),
            ["Favori", "Aucun diplôme demandé", "Débutant accepté", "CDI", "Télétravail possible"]
        );
        assert_eq!(noms(&o["lignes"][1]), ["CDI"]);
        for c in o["lignes"].as_array().unwrap() {
            for n in noms(c) {
                assert!(!["Très proche", "Proche", "Un peu", "Hors sujet"].contains(&n.as_str()), "{n}");
            }
        }
        let f = &o["fiche"];
        assert_eq!(f["note"]["code"], "tres_proche");
        assert_eq!(f["lecture"][0]["valeur"], "Aucun diplôme demandé");
        assert_eq!(f["lecture"][0]["preuve"], "Sans diplôme");
        // (0.2.1 : plus de « lu par Jobmind » : rien pour une simple lecture de Jobmind, ni pour ce qui n'est pas dit,
        // ni pour les permis)
        assert_eq!(f["lecture"][0]["par"], "");
        assert_eq!(f["lecture"][1]["par"], "");
        assert_eq!(f["lecture"][2]["par"], "");
        assert!(!e.to_string().contains("lu par Jobmind"));
        // (le texte vient d'Adzuna : un résumé, ce qu'il ne dit pas est peut-être dans l'annonce complète)
        assert_eq!(f["lecture"][1]["valeur"], "Pas dans le résumé d'Adzuna");
        assert_eq!(f["lecture"][2]["valeur"], "Permis B");
        // Le texte est trop court pour Claude
        assert!(f["ia"].is_null());
        assert_eq!(o["liste_t"], "Les plus pertinentes en haut");
        assert_eq!(o["filtres"]["tri"], "pertinence");
        // Un texte assez long, sans clé Claude : la clé est demandée ; avec : « Demander à Claude »
        let x = v.ouverte.as_mut().unwrap();
        x.description = "Un long texte d'offre. ".repeat(20);
        assert_eq!(onglet(&v, j(18))["offres"]["fiche"]["ia"], "cle");
        v.claude = true;
        v.floues = 3;
        let e = onglet(&v, j(18));
        assert_eq!(e["offres"]["fiche"]["ia"], "lire");
        assert_eq!(e["offres"]["ia"]["texte"], "3 offres floues que Jobmind n'a pas su lire en entier.");
        v.claude = false;
        assert_eq!(
            onglet(&v, j(18))["offres"]["ia"]["texte"],
            "Ajoute ta clé Claude pour que Jobmind lise aussi les offres floues."
        );
        // Hors sujet
        let x = v.ouverte.as_mut().unwrap();
        x.offre.hors_sujet = true;
        x.offre.pertinence = Some(0);
        let f = onglet(&v, j(18))["offres"]["fiche"].clone();
        assert_eq!((f["note"]["nom"].as_str(), f["hors_sujet"].as_bool()), (Some("Hors sujet"), Some(true)));
        assert!(f["ia"].is_null());
        assert_eq!(v.ouverte.as_ref().unwrap().offre.niveau(), Some(Niveau::HorsSujet));
        // Sans « Ce que je cherche » : la phrase qui le demande
        v.profil = false;
        assert!(onglet(&v, j(18))["offres"]["sans_note"].as_str().unwrap().contains("Ce que je cherche"));
    }

    /// 0.2.1 : « déduit par Jobmind » et « lu par Claude » restent sur ce que demande l'offre.
    #[test]
    fn qui_a_lu_sans_lu_par_jobmind() {
        let mut v = exemple();
        let x = v.ouverte.as_mut().unwrap();
        x.lecture.diplome.as_mut().unwrap().confiance = Confiance::Moyenne;
        x.lecture.experience = Some(Lu {
            valeur: Experience::Debutant,
            preuve: "Débutant accepté".into(),
            confiance: Confiance::Forte,
            par: LuPar::Ia,
        });
        let e = onglet(&v, j(18));
        let l = &e["offres"]["fiche"]["lecture"];
        assert_eq!(l[0]["par"], "déduit par Jobmind");
        assert_eq!(l[1]["par"], "lu par Claude");
        assert_eq!(l[2]["par"], "");
        assert!(!e.to_string().contains("lu par Jobmind"));
    }

    /// 0.2.1 : les cartes des offres gardées : « Déjà vue » d'après le champ `vue`, le bandeau de la note (les quatre
    /// niveaux), une offre retirée en gris sans bandeau ni « Déjà vue », les marques en étiquettes.
    #[test]
    fn les_cartes_deja_vue_et_bandeau() {
        let mut v = exemple();
        v.ouverte = None;
        let e = onglet(&v, j(18));
        let l = e["offres"]["lignes"].as_array().unwrap().clone();
        // (l'offre 1 a été ouverte, l'offre 2 jamais ; aucune n'est ouverte en grand)
        assert_eq!((&l[0]["vue"], &l[1]["vue"]), (&json!(true), &json!(false)));
        assert_eq!((&l[0]["sel"], &l[0]["etat"], &l[0]["retiree"]), (&json!(false), &json!("lue"), &json!(false)));
        assert!(e["offres"]["fiche"].is_null());
        assert_eq!(e["offres"]["textes"]["deja_vue"], "Déjà vue");
        // Les quatre niveaux de la note, en mots, dans le bandeau
        for (note, hors, niveau) in [(92, false, "tres"), (65, false, "proche"), (40, false, "peu"), (0, true, "hors")]
        {
            v.liste[1].pertinence = Some(note);
            v.liste[1].hors_sujet = hors;
            let nom = v.liste[1].niveau().unwrap().nom();
            let c = onglet(&v, j(18))["offres"]["lignes"][1].clone();
            assert_eq!(c["proche"], json!({ "niveau": niveau, "nom": nom }), "{note}");
        }
        assert_eq!(v.liste[1].niveau(), Some(Niveau::HorsSujet));
        // Une offre retirée de ma liste : en gris, ni bandeau ni « Déjà vue » ; postulée et gardée pour plus tard
        v.liste[0].cachee = true;
        v.liste[0].plus_tard = true;
        v.liste[0].candidature = Some(3);
        let c = onglet(&v, j(18))["offres"]["lignes"][0].clone();
        assert_eq!((&c["retiree"], &c["vue"]), (&json!(true), &json!(false)));
        assert!(c.get("proche").is_none());
        let noms: Vec<&str> = c["etiquettes"].as_array().unwrap().iter().map(|e| e["nom"].as_str().unwrap()).collect();
        assert_eq!(noms[..3], ["Postulé", "Favori", "Plus tard"]);
        // Sans date de publication : le jour où elle a été gardée
        v.liste[1].publiee_le = None;
        assert_eq!(onglet(&v, j(18))["offres"]["lignes"][1]["l3"], "gardée le 18 nov. · sur Indeed");
    }

    /// 0.2.1 : les pages de cartes (« Page 2 sur 7 · offres 25 à 48 »), leurs deux boutons, et les voisines de
    /// l'offre ouverte en grand.
    #[test]
    fn les_pages_de_cartes_et_les_voisines() {
        let mut v = exemple();
        let modele = v.liste[1].clone();
        v.liste = (0..CARTES_PAR_PAGE as i64).map(|k| Offre { id: 100 + k, ..modele.clone() }).collect();
        (v.total, v.gardees, v.page, v.pages, v.debut) = (150, 150, 2, 7, CARTES_PAR_PAGE);
        v.ouverte = None;
        let e = onglet(&v, j(18));
        let o = &e["offres"];
        assert_eq!(o["lignes"].as_array().unwrap().len(), 24);
        assert_eq!(o["page_texte"], "Page 2 sur 7 · offres 25 à 48");
        assert_eq!((&o["page"], &o["pages"], &o["par_page"]), (&json!(2), &json!(7), &json!(24)));
        assert_eq!((&o["precedente"], &o["suivante"]), (&json!(true), &json!(true)));
        assert_eq!(o["compte"], "150 offres gardées");
        assert_eq!((&o["fiche_avant"], &o["fiche_apres"]), (&json!(false), &json!(false)));
        // La dernière page, pas pleine
        v.liste.truncate(6);
        (v.page, v.debut) = (7, 6 * CARTES_PAR_PAGE);
        let e = onglet(&v, j(18));
        assert_eq!(e["offres"]["page_texte"], "Page 7 sur 7 · offres 145 à 150");
        assert_eq!((&e["offres"]["precedente"], &e["offres"]["suivante"]), (&json!(true), &json!(false)));
        // Une seule page : aucun bouton ; aucune carte : aucune phrase de page
        (v.page, v.pages, v.debut, v.total) = (1, 1, 0, 6);
        let e = onglet(&v, j(18));
        assert_eq!(e["offres"]["page_texte"], "Page 1 sur 1 · offres 1 à 6");
        assert_eq!((&e["offres"]["precedente"], &e["offres"]["suivante"]), (&json!(false), &json!(false)));
        v.liste.clear();
        assert_eq!(onglet(&v, j(18))["offres"]["page_texte"], "");
        // L'offre ouverte en grand : ses voisines, et les mots de ses boutons
        let mut v = exemple();
        (v.avant, v.apres) = (false, true);
        let e = onglet(&v, j(18));
        let o = &e["offres"];
        assert_eq!((&o["fiche_avant"], &o["fiche_apres"]), (&json!(false), &json!(true)));
        assert_eq!(o["fiche"]["id"], 1);
        let t = &o["textes"];
        assert_eq!(
            (&t["retour_liste"], &t["offre_avant"], &t["offre_apres"], &t["precedente"], &t["suivante"]),
            (
                &json!("Retour à la liste"),
                &json!("Offre précédente"),
                &json!("Offre suivante"),
                &json!("Page précédente"),
                &json!("Page suivante")
            )
        );
    }

    #[test]
    fn les_panneaux_vides() {
        let rien = crate::offres::VIDE.clone();
        assert_eq!(onglet(&rien, j(18))["offres"]["vide"]["bouton"], "nouvelle");
        // (0.1.8 : plus de « à partir de Ce que je cherche », le tableau des recherches est le même partout)
        let avec_profil = VueOffres { profil: true, ..rien.clone() };
        assert_eq!(onglet(&avec_profil, j(18))["offres"]["vide"]["bouton"], "nouvelle");
        let mut x = exemple();
        x.liste.clear();
        x.ouverte = None;
        x.gardees = 0;
        let e = onglet(&x, j(18));
        assert_eq!(e["offres"]["vide"]["bouton"], "chercher");
        assert!(e["offres"]["vide"]["texte"].as_str().unwrap().contains("France Travail"));
        x.gardees = 3;
        x.filtres_actifs = true;
        let e = onglet(&x, j(18));
        assert_eq!(e["offres"]["vide"]["bouton"], "filtres");
        assert!(e["offres"]["fiche"].is_null());
    }

    /// 0.1.5 : la fiche « En bref d'abord » d'une vraie offre de France Travail (gardée en 0.1.4, sans nom
    /// d'entreprise en base) : les cases, le texte rangé, la carte au nom lu après « L'employeur », sans tiret long.
    #[test]
    fn la_fiche_en_bref() {
        let mut v = exemple();
        let x = v.ouverte.as_mut().unwrap();
        x.description = include_str!("../../jobmind-core/src/fiche/exemples/brut2.txt").into();
        x.offre.titre = "Agent de Production (h/f) (H/F)".into();
        x.offre.entreprise = String::new();
        let f = &onglet(&v, j(18))["offres"]["fiche"];
        assert_eq!(f["bref"][0], json!({ "nom": "Salaire", "valeur": "12,31 €", "sous": "brut de l'heure" }));
        assert_eq!(f["bref"][1], json!({ "nom": "Temps", "valeur": "35 h", "sous": "par semaine" }));
        assert_eq!(f["bref"][2]["nom"], "Horaires");
        assert_eq!(f["bref"][3], json!({ "nom": "Contrat", "valeur": "Intérim", "sous": "6 mois" }));
        let r = &f["rangee"];
        assert!(r["missions"].as_array().unwrap().iter().any(|m| m == "Contrôler la qualité des pièces produites"));
        assert_eq!(r["entreprise"], "ADECCO FRANCE");
        assert_eq!(r["carte"]["nom"], "ADECCO FRANCE");
        // (les infos de l'en-tête ne répètent pas les cases : ni horaires, ni salaire, ni contrat ; les anciens onglets
        // ne sont plus envoyés)
        let noms: Vec<&str> = f["infos"].as_array().unwrap().iter().map(|i| i["nom"].as_str().unwrap()).collect();
        assert_eq!(noms, ["Télétravail", "Publiée"]);
        assert!(f["onglets"].is_null());
        assert!(!f.to_string().contains(['\u{2013}', '\u{2014}']));
        // L'exemple sans rubrique : tout dans la présentation, la carte de l'entreprise
        let f = &onglet(&exemple(), j(18))["offres"]["fiche"];
        assert!(!f["rangee"]["intro"].as_str().unwrap().is_empty());
        assert_eq!(f["rangee"]["carte"]["nom"], "Entrepôts du Val de Moselle");
    }

    /// 0.1.5 : les cases « En bref » se coupent en valeur et dessous ; une case inconnue n'est pas montrée.
    #[test]
    fn les_cases_en_bref() {
        let f = Fiche {
            salaire: "1 802 € brut par mois".into(),
            horaires: vec!["Travail en journée".into(), "Travail les week-ends et jours fériés".into()],
            contrat: "Indépendant (non salarié)".into(),
            ..Fiche::default()
        };
        let (bref, rangee) = fiche_b_vue(&f, "", None, None);
        assert_eq!(
            bref,
            json!([
                { "nom": "Salaire", "valeur": "1 802 €", "sous": "brut par mois" },
                { "nom": "Horaires", "valeur": "En journée", "sous": "les week-ends et jours fériés" },
                { "nom": "Contrat", "valeur": "Indépendant (non salarié)", "sous": "" },
            ])
        );
        assert!(rangee.get("carte").is_none());
        // (des heures lues : la ligne qui les écrit ne revient pas en texte, la plage sert de dessous)
        let f = Fiche {
            horaires: vec!["Travail en journée".into(), "Horaires : 7h-12h et 13h-16h du lundi au vendredi".into()],
            plages: vec![rangement::Plage { debut: 420, fin: 720 }, rangement::Plage { debut: 780, fin: 960 }],
            ..Fiche::default()
        };
        assert_eq!(
            fiche_b_vue(&f, "", None, None).0[0],
            json!({ "nom": "Horaires", "valeur": "7 h à 12 h", "sous": "et 13 h à 16 h" })
        );
        // (une seule plage : dessous, l'horaire écrit sans heure, jamais « le samedi » à la place des heures)
        let f = Fiche {
            horaires: vec!["Horaires : 6h-9h".into(), "Travail en journée".into()],
            plages: vec![rangement::Plage { debut: 360, fin: 540 }],
            ..Fiche::default()
        };
        assert_eq!(
            fiche_b_vue(&f, "", None, None).0[0],
            json!({ "nom": "Horaires", "valeur": "6 h à 9 h", "sous": "en journée" })
        );
        // (rien dans le texte : le salaire et le contrat de l'offre ; une plage d'heures sans horaire écrit)
        let f = Fiche { plages: vec![rangement::Plage { debut: 360, fin: 810 }], ..Fiche::default() };
        let (bref, _) = fiche_b_vue(&f, "Selon profil", Some(Contrat::Cdi), None);
        assert_eq!(
            bref,
            json!([
                { "nom": "Salaire", "valeur": "Selon profil", "sous": "" },
                { "nom": "Horaires", "valeur": "6 h à 13 h 30", "sous": "" },
                { "nom": "Contrat", "valeur": "CDI", "sous": "" },
            ])
        );
        assert_eq!(fiche_b_vue(&Fiche::default(), "", None, None).0, json!([]));
    }

    /// 0.1.4 : la carte de l'entreprise d'une offre gardée, avec et sans site dans l'annonce, avec l'annuaire.
    #[test]
    fn la_carte_de_l_entreprise() {
        use crate::entreprise::Connu;
        use jobmind_sources::annuaire::Fiche;
        let mut v = exemple();
        // (« Ennery, Moselle » : deux communes s'appellent Ennery, la carte ne cherche pas dans l'annuaire)
        let c = onglet(&v, j(18))["offres"]["fiche"]["rangee"]["carte"].clone();
        assert_eq!(c["attente"], false);
        assert!(c["note"].as_str().unwrap().starts_with("Le nom vient de l'annonce."), "{c}");
        v.ouverte.as_mut().unwrap().offre.lieu = "Ennery (57)".into();
        // (sans site, l'annuaire pas encore répondu)
        let c = onglet(&v, j(18))["offres"]["fiche"]["rangee"]["carte"].clone();
        assert_eq!(
            c,
            json!({
                "site": "", "origine": "", "nom": "Entrepôts du Val de Moselle", "secteur": "", "taille": "",
                "adresse": "", "note": "Jobmind cherche cette entreprise dans l'annuaire officiel des entreprises.",
                "attente": true
            })
        );
        // (avec le site de l'annonce et la réponse de l'annuaire)
        v.ouverte.as_mut().unwrap().site_entreprise = "https://www.entrepots-val-moselle.fr/".into();
        v.annuaire = Some(Connu::Trouve(Fiche {
            nom: "ENTREPOTS DU VAL DE MOSELLE".into(),
            secteur: "Transports et entreposage".into(),
            taille: "100 à 199 salariés (chiffre de 2023)".into(),
            adresse: "ZONE ACTIPOLE 57365 ENNERY".into(),
            commune: "ENNERY".into(),
        }));
        let f = onglet(&v, j(18));
        let c = &f["offres"]["fiche"]["rangee"]["carte"];
        assert_eq!(c["site"], "entrepots-val-moselle.fr");
        assert_eq!(c["origine"], "Donné dans l'annonce");
        assert_eq!(c["nom"], "ENTREPOTS DU VAL DE MOSELLE");
        assert_eq!(c["taille"], "100 à 199 salariés (chiffre de 2023)");
        assert_eq!(c["attente"], false);
        // (aucune adresse de site envoyée à la page)
        assert!(!f.to_string().contains("https://www.entrepots"), "{f}");
    }

    /// Audit du 27/11/2026 : l'exemple du poste est en mots simples (« assistance informatique », pas « helpdesk »).
    #[test]
    fn exemple_du_poste_en_mots_simples() {
        let e = onglet(&exemple(), j(18));
        let t = e["offres"]["textes"]["i_mots"].as_str().unwrap();
        assert_eq!(t, "Par exemple : assistance informatique, cariste, vendeur");
        assert!(!e.to_string().contains("helpdesk"));
    }
}
