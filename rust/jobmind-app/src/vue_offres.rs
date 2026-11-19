//! Ce que l'onglet Offres montre (étape 4 du plan), dessiné par ui\jobmind.js : la barre des filtres (le texte cherché
//! dans tout le texte des offres gardées, le lieu, le contrat, le télétravail, toutes / favoris / plus tard / cachées),
//! à gauche la recherche gardée choisie, « Chercher maintenant » et la liste des offres, à droite l'offre ouverte (son
//! texte gardé sur ce PC, ses sources et « Voir sur le site », la mention « Adzuna » liée à adzuna.fr pour chaque offre
//! d'Adzuna, favori, plus tard, « Pas pour moi », « J'ai postulé »), ou « Mes recherches ».
//!
//! Étape 6 : chaque offre a sa pastille (« Très proche », « Proche », « Un peu », « Hors sujet ») et sa raison en une
//! ligne ; l'offre ouverte dit le diplôme et l'expérience demandés avec leur preuve (le bout de phrase de l'offre) et
//! qui les a lus (« lu par Jobmind » ou « lu par Claude ») ; la barre a « Aucun diplôme requis », « Débutant accepté »
//! et le tri ; la liste, la rubrique « Hors sujet » et ce que Claude peut lire (ou la clé qui lui manque).
//!
//! Tous les textes sont écrits ici, en français simple, sans tiret long ni demi-cadratin ; la page les échappe. Les
//! listes des départements et des régions ne sont pas répétées : la page les prend dans Mon profil (« Ce que je
//! cherche »). Aucune adresse d'offre n'est envoyée à la page : « Voir sur le site » la demande à l'appli.

use jobmind_core::Heure;
use jobmind_core::Jour;
use jobmind_core::lecture::{Confiance, LuPar};
use jobmind_core::offre::{Offre, TeleOffre, noms_sources};
use jobmind_core::recherche::{Contrat, GenreLieu, Lieu, RAYONS, Teletravail};
use jobmind_core::recherches::NOMBRE_MAX;
use jobmind_storage::Moment;
use jobmind_storage::offres::{OffreComplete, RechercheGardee};
use serde_json::{Value, json};

use crate::ecran::{AUCUNE_OFFRE, OFFRES};
use crate::offres::{LISTE_MAX, VueOffres, n_offres, nom_source};
use jobmind_storage::lecture::TEXTE_MIN_IA;

/// Les textes de l'onglet (la page les pose tels quels), en objet.
fn textes(paires: &[(&str, &str)]) -> Value {
    Value::Object(paires.iter().map(|(k, v)| ((*k).to_string(), json!(v))).collect())
}

/// « Cariste, autour de Metz (30 km) », « Vendeur, en Moselle », « Agent de tri, partout en France ».
pub fn lieu_texte(l: &Lieu) -> String {
    match l {
        Lieu::Ville { nom, rayon } => format!("autour de {nom} ({rayon} km)"),
        Lieu::Departement(d) => format!("dans le {}", d.libelle()),
        Lieu::Region(r) => format!("en {}", r.nom),
        Lieu::Partout => "partout en France".into(),
    }
}

/// « aujourd'hui à 14 h 05 », « le 17 nov. à 9 h 30 ».
fn moment_texte(m: Moment, aujourdhui: Jour) -> String {
    let h = Heure::nouvelle(m.heure, m.minute).map(Heure::texte).unwrap_or_default();
    if m.jour() == aujourdhui {
        format!("aujourd'hui à {h}")
    } else {
        format!("le {} à {h}", m.jour().court(aujourdhui))
    }
}

/// Ce qu'a donné la dernière fois d'une recherche.
fn derniere_fois(r: &RechercheGardee, aujourdhui: Jour) -> String {
    match (r.derniere_fois, r.dernier_nombre) {
        (Some(m), Some(n)) => {
            format!("Dernière fois {}, {} trouvées", moment_texte(m, aujourdhui), n_offres(n.max(0) as usize))
                .replace("1 offre trouvées", "1 offre trouvée")
        }
        (Some(m), None) => format!("Dernière fois {}", moment_texte(m, aujourdhui)),
        _ => "Pas encore lancée".into(),
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

/// La petite ligne d'une offre : « CDI, le 17 nov., sur Adzuna et Jooble ».
fn ligne3(o: &Offre, aujourdhui: Jour) -> String {
    let mut l: Vec<String> = Vec::new();
    if let Some(c) = o.contrat {
        l.push(c.nom().to_string());
    }
    match o.publiee_le {
        Some(j) => l.push(format!("le {}", j.court(aujourdhui))),
        None => l.push(format!("gardée le {}", o.ajoutee_le.court(aujourdhui))),
    }
    l.push(format!("sur {}", noms(o)));
    l.join(", ")
}

fn ligne(o: &Offre, ouverte: Option<i64>, aujourdhui: Jour) -> Value {
    json!({
        "id": o.id,
        "titre": o.titre,
        "sous": sous(o),
        "l3": ligne3(o, aujourdhui),
        "sel": Some(o.id) == ouverte,
        "nouvelle": !o.vue && !o.cachee,
        "favori": o.favori,
        "plus_tard": o.plus_tard,
        "cachee": o.cachee,
        "postule": o.candidature.is_some(),
        // (étape 6 : la pastille et la raison)
        "note": o.niveau().map(|n| json!({ "code": n.code(), "nom": n.nom() })),
        "raison": o.raison,
    })
}

/// Ce qui a été lu, pour l'offre ouverte : la valeur, la preuve et qui l'a lu.
fn lu_vue(nom: &str, lu: Option<(String, &str, LuPar, Confiance)>, lue: bool) -> Value {
    match lu {
        Some((valeur, preuve, par, confiance)) => json!({
            "nom": nom,
            "valeur": valeur,
            "preuve": preuve,
            "par": if par == LuPar::Regles && confiance == Confiance::Moyenne {
                "déduit par Jobmind".to_string()
            } else {
                par.nom().to_string()
            },
            "dit": true,
        }),
        None => json!({
            "nom": nom,
            "valeur": "Non dit dans l'offre",
            "preuve": "",
            "par": if lue { "lu par Jobmind" } else { "" },
            "dit": false,
        }),
    }
}

/// L'offre ouverte.
fn fiche(x: &OffreComplete, aujourdhui: Jour, claude: bool) -> Value {
    let o = &x.offre;
    let l = &x.lecture;
    let mut lecture = vec![
        lu_vue(
            "Diplôme",
            l.diplome.as_ref().map(|d| (d.valeur.nom().to_string(), d.preuve.as_str(), d.par, d.confiance)),
            l.lue,
        ),
        lu_vue(
            "Expérience",
            l.experience.as_ref().map(|e| (e.valeur.nom(), e.preuve.as_str(), e.par, e.confiance)),
            l.lue,
        ),
    ];
    if !l.autres.is_empty() {
        lecture.push(
            json!({ "nom": "Permis et CACES", "valeur": l.autres, "preuve": "", "par": "lu par Jobmind", "dit": true }),
        );
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
    let infos = vec![
        json!({ "nom": "Contrat", "valeur": o.contrat.map_or(pas_dit, Contrat::nom) }),
        json!({ "nom": "Salaire", "valeur": if o.salaire.is_empty() { pas_dit.to_string() } else { o.salaire.clone() } }),
        json!({ "nom": "Télétravail", "valeur": tele }),
        json!({ "nom": "Publiée", "valeur": o.publiee_le.map_or("Non précisée".to_string(), |j| format!("le {}", j.court(aujourdhui))) }),
    ];
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
        "texte_vide": match (x.venue_par_mail, x.liens.iter().find(|l| !l.url.is_empty())) {
            (Some(_), Some(l)) => format!(
                "Le mail d'alerte ne donne que le titre. Clique « Voir sur {} », puis « Ajouter à Jobmind » dans la \
                 petite fenêtre : Jobmind gardera le texte entier de l'offre.",
                nom_source(&l.source)
            ),
            _ => "Le site n'a donné aucun texte pour cette offre. Lis-la sur le site.".to_string(),
        },
        "note": if matches!(x.description_de.as_str(), "adzuna" | "jooble") {
            format!("{texte_de} ne donne que le début de l'annonce : le texte entier est sur le site.")
        } else if x.description_de == "site" {
            "Le texte entier de l'annonce, lu sur la page du site.".to_string()
        } else {
            format!("Le texte entier de l'annonce, lu sur la page de {texte_de}.")
        },
        "gardee": gardee,
        "liens": liens,
        "adzuna": o.sources.iter().any(|s| s == "adzuna"),
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
    let (icone, titre, texte, bouton, note): (&str, String, &str, &str, &str) = if !v.cles.0 && !v.cles.1 {
        (
            "cle",
            "Il manque une clé".into(),
            "Pour chercher des offres, Jobmind a besoin de ta clé Adzuna ou Jooble : un code gratuit, créé une seule \
             fois sur leur site.",
            "cles",
            "Une seule des deux suffit pour commencer.",
        )
    } else if v.recherches.is_empty() {
        (
            "cible",
            "Aucune recherche pour l'instant".into(),
            "Une recherche, c'est ce que tu cherches et où. Crée-les à partir de « Ce que je cherche », ou écris-en une.",
            if v.profil { "profil" } else { "nouvelle" },
            "",
        )
    } else if v.gardees == 0 && v.cachees == 0 {
        (
            "loupe",
            AUCUNE_OFFRE.into(),
            "Choisis une recherche à gauche, puis « Chercher sur Adzuna et Jooble ». Jobmind leur demande les \
             offres, et garde chacune sur ce PC.",
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

/// Mes recherches (le panneau de droite « Gérer ») : chaque recherche avec ce qu'elle cherche et sa dernière fois.
fn gestion(v: &VueOffres, aujourdhui: Jour) -> Value {
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
                "sous": format!("{}, {}", r.mots, lieu_texte(&r.lieu)),
                "derniere": derniere_fois(r, aujourdhui),
                "message": r.dernier_message.clone().unwrap_or_default(),
                "valeurs": { "nom": r.nom, "mots": r.mots, "lieu": genre.code(), "ville": ville, "rayon": rayon,
                             "departement": dep, "region": reg },
                "question": format!("Supprimer la recherche « {} » ? Les offres qu'elle a trouvées restent gardées.", r.nom),
            })
        })
        .collect();
    json!({
        "titre": "Mes recherches",
        "intro": "Chaque recherche dit ce que tu cherches et où. « Chercher maintenant » demande ses offres à Adzuna et \
                  à Jooble.",
        "liste": l,
        "plein": v.recherches.len() >= NOMBRE_MAX,
        "profil": v.profil,
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
    // (la liste vide : une phrase seulement si des offres gardées sont cachées par les filtres)
    let vide_liste = if v.gardees + v.cachees > 0 { "Aucune offre avec ces filtres." } else { "" };
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
        { "id": "cachees", "nom": "Cachées", "nombre": v.cachees },
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
    let annonce = v.annonce.as_ref().map(|a| json!({ "texte": a.texte, "genre": a.genre, "cles": a.cles }));
    json!({
        "id": OFFRES,
        "nom": "Offres",
        "nombre": v.nouvelles,
        "liste": "Les offres",
        "vide": "Aucune offre gardée.",
        "rubriques": [],
        "dossier": {
            "icone": "loupe",
            "titre": AUCUNE_OFFRE,
            "texte": "Jobmind rassemble ici les offres d'Adzuna et de Jooble, et celles que tu ajoutes depuis les sites, \
                      sans doublons.",
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
            "vide_liste": vide_liste,
            "recherches": v.recherches.iter().map(|r| json!({ "id": r.id, "nom": r.nom })).collect::<Vec<_>>(),
            "recherche": choisie.map(|r| r.id),
            "recherche_sous": choisie.map(|r| format!("{}, {}", r.mots, lieu_texte(&r.lieu))),
            "recherche_fois": choisie.map(|r| derniere_fois(r, aujourdhui)),
            "en_cours": v.en_cours,
            "annonce": annonce,
            "cles": v.cles.0 || v.cles.1,
            "lignes": v.liste.iter().map(|o| ligne(o, ouverte, aujourdhui)).collect::<Vec<_>>(),
            "plus": if v.total > LISTE_MAX {
                format!("Seules les {LISTE_MAX} premières sont montrées : précise les filtres pour voir les autres.")
            } else {
                String::new()
            },
            "fiche": v.ouverte.as_ref().map(|o| fiche(o, aujourdhui, v.claude)),
            "vide": vide(v),
            "gestion": gestion(v, aujourdhui),
            "textes": textes(&[
                ("i_texte", "Chercher dans les offres gardées : quai, nuit, magasin..."),
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
                ("chercher", "Chercher maintenant"),
                ("chercher_aj", "Chercher sur Adzuna et Jooble"),
                ("liste_t", "Les plus récentes en haut"),
                ("vide_l", "Aucune offre avec ces filtres."),
                ("sans_recherche", "Crée une recherche pour commencer."),
                ("ouvrir_cles", "Ouvrir Mes clés"),
                ("depuis_profil", "Créer depuis « Ce que je cherche »"),
                ("nouvelle", "Nouvelle recherche"),
                ("retirer", "Retirer les filtres"),
                ("favori", "Favori"),
                ("plus_tard", "Plus tard"),
                ("cacher", "Pas pour moi"),
                ("decacher", "Remettre dans la liste"),
                ("postule", "J'ai postulé"),
                ("voir_candidature", "Voir ma candidature"),
                ("postule_deja", "Tu as noté une candidature pour cette offre."),
                ("par", "Offre trouvée par"),
                ("adzuna", "Adzuna"),
                ("adzuna_aide", "Ouvrir le site d'Adzuna"),
                ("texte_t", "L'offre"),
                ("doublon", "La même offre était sur plusieurs sites : elle n'est montrée qu'une fois."),
                ("nouvelle_l", "Nouvelle"),
                ("form_nouvelle", "Nouvelle recherche"),
                ("form_modifier", "Modifier la recherche"),
                ("nom", "Nom"),
                ("facultatif", "facultatif"),
                ("i_nom", "Par exemple : Quai autour de Metz"),
                ("mots", "Ce que je cherche"),
                ("i_mots", "Par exemple : agent de quai"),
                ("mots_aide", "Un métier ou quelques mots, comme sur un site d'emploi."),
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
            ]),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offres::Annonce;
    use jobmind_core::lecture::{Diplome, Experience};
    use jobmind_core::pertinence::Niveau;
    use jobmind_storage::lecture::{LectureGardee, Lu};
    use jobmind_storage::offres::{Lien, VueCandidats};

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
        let mut a = o(1, "Agent de quai H/F", "Entrepôts du Val de Moselle", "Ennery, Moselle", &["adzuna", "jooble"]);
        a.favori = true;
        a.vue = true;
        a.teletravail = Some(TeleOffre::Partiel);
        a.salaire = "21 800 à 23 500 € par an".into();
        a.pertinence = Some(92);
        a.raison = "Aucun diplôme demandé, à 12 km, débutant accepté".into();
        a.diplome = Some(Diplome::Aucun);
        a.experience = Some(Experience::Debutant);
        let b = o(2, "Agent de tri", "", "Forbach (57)", &["jooble"]);
        let r = RechercheGardee {
            id: 4,
            nom: "Agent de quai".into(),
            mots: "agent de quai".into(),
            lieu: Lieu::Ville { nom: "Metz".into(), rayon: 30 },
            derniere_fois: Moment::nouveau(2026, 11, 18, 14, 5, 0),
            dernier_nombre: Some(19),
            dernier_message: Some("19 offres trouvées".into()),
        };
        VueOffres {
            liste: vec![a.clone(), b],
            total: 2,
            gardees: 2,
            nouvelles: 1,
            favoris: 1,
            ouverte: Some(OffreComplete {
                offre: a,
                description: "Chargement et déchargement des camions.\nDébutant accepté.".into(),
                description_de: "adzuna".into(),
                liens: vec![
                    Lien { source: "adzuna".into(), url: "https://www.adzuna.fr/land/ad/1".into() },
                    Lien { source: "jooble".into(), url: "https://jooble.org/desc/1".into() },
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
            }),
            recherches: vec![r],
            recherche: Some(4),
            annonce: Some(Annonce {
                texte: "19 offres trouvées sur Adzuna et Jooble.".into(),
                genre: "ok",
                cles: false,
            }),
            saisie: crate::offres::VIDE.saisie.clone(),
            cles: (true, true),
            profil: true,
            ..VueOffres::default()
        }
    }

    #[test]
    fn l_onglet_rempli() {
        let v = exemple();
        let e = onglet(&v, j(18));
        let o = &e["offres"];
        assert_eq!(e["nombre"], 1);
        assert_eq!(o["compte"], "2 offres gardées");
        assert_eq!(o["recherche_sous"], "agent de quai, autour de Metz (30 km)");
        assert_eq!(o["recherche_fois"], "Dernière fois aujourd'hui à 14 h 05, 19 offres trouvées");
        let l = o["lignes"].as_array().unwrap();
        assert_eq!(l[0]["l3"], "CDI, le 17 nov., sur Adzuna et Jooble");
        assert_eq!(l[0]["sel"], true);
        assert_eq!(l[1]["sous"], "Forbach (57)");
        assert_eq!(l[1]["nouvelle"], true);
        let f = &o["fiche"];
        assert_eq!(f["sous"], "Entrepôts du Val de Moselle, Ennery");
        assert_eq!(f["sources"], "Trouvée sur Adzuna et Jooble");
        assert_eq!((f["double"].as_bool(), f["adzuna"].as_bool()), (Some(true), Some(true)));
        assert_eq!(f["liens"][1]["nom"], "Voir sur Jooble");
        assert_eq!(f["candidats"], "Plus de 100 candidats sur LinkedIn, vu le 17 nov.");
        assert_eq!(f["infos"][1]["valeur"], "21 800 à 23 500 € par an");
        assert_eq!(f["infos"][2]["valeur"], "Possible");
        assert_eq!(f["gardee"], "Gardée sur ce PC depuis le 18 nov. : elle reste ici même si elle disparaît du site.");
        // Aucune adresse d'offre dans l'état
        assert!(!e.to_string().contains("https://"), "{e}");
        assert_eq!(o["gestion"]["liste"][0]["valeurs"]["lieu"], "ville");
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
        cas.push(VueOffres { cles: (true, false), ..crate::offres::VIDE.clone() });
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
        assert_eq!(o["lignes"][0]["note"]["nom"], "Très proche");
        assert_eq!(o["lignes"][0]["raison"], "Aucun diplôme demandé, à 12 km, débutant accepté");
        assert!(o["lignes"][1]["note"].is_null());
        let f = &o["fiche"];
        assert_eq!(f["note"]["code"], "tres_proche");
        assert_eq!(f["lecture"][0]["valeur"], "Aucun diplôme demandé");
        assert_eq!(f["lecture"][0]["preuve"], "Sans diplôme");
        assert_eq!(f["lecture"][0]["par"], "lu par Jobmind");
        assert_eq!(f["lecture"][1]["valeur"], "Non dit dans l'offre");
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

    #[test]
    fn les_panneaux_vides() {
        let rien = crate::offres::VIDE.clone();
        assert_eq!(onglet(&rien, j(18))["offres"]["vide"]["bouton"], "cles");
        let avec_cle = VueOffres { cles: (false, true), profil: true, ..rien.clone() };
        assert_eq!(onglet(&avec_cle, j(18))["offres"]["vide"]["bouton"], "profil");
        let mut x = exemple();
        x.liste.clear();
        x.ouverte = None;
        x.gardees = 0;
        assert_eq!(onglet(&x, j(18))["offres"]["vide"]["bouton"], "chercher");
        x.gardees = 3;
        x.filtres_actifs = true;
        let e = onglet(&x, j(18));
        assert_eq!(e["offres"]["vide"]["bouton"], "filtres");
        assert!(e["offres"]["fiche"].is_null());
    }
}
