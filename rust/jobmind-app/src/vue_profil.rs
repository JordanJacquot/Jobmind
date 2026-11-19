//! Ce que l'onglet Mon profil montre (étape 3 du plan) : ses rubriques Mon CV, Ce que je cherche, Mon style et Mes
//! clés, chacune avec sa ligne de la liste (à gauche) et son dossier (à droite), dessinés par ui\jobmind.js. La
//! rubrique Réglages reste dans ecran.rs.
//!
//! Tous les textes sont écrits ici, en français simple, sans tiret long ni demi-cadratin ; la page les échappe. Une clé
//! n'y est jamais : seulement « enregistrée » et ses 4 derniers caractères.

use jobmind_core::Jour;
use jobmind_core::cle::{Cle, Service};
use jobmind_core::lieux::{DEPARTEMENTS, REGIONS};
use jobmind_core::profil::{CV_NOMBRE_MAX, CV_TEXTE_MAX, STYLE_NOMBRE_MAX, STYLE_TEXTE_MAX, debut, mots};
use jobmind_core::recherche::{Contrat, GenreLieu, Lieu, RAYON_DEFAUT, RAYONS, Recherche, Teletravail};
use jobmind_storage::cles::EtatCle;
use jobmind_storage::profil::{Cv, GenreCv, TexteStyle};
use serde_json::{Value, json};

/// Ce que le profil montre, lu dans la base et le fichier des clés (crate::profil, `Profil::vue`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VueProfil {
    pub cvs: Vec<Cv>,
    pub cv_ouvert: Option<i64>,
    /// Un import en cours : rien (la boîte « Ouvrir » est ouverte) ou le nom du fichier en lecture
    pub import: Option<Option<String>>,
    pub style: Vec<TexteStyle>,
    pub style_ouvert: Option<i64>,
    pub recherche: Option<Recherche>,
    /// Étape 9 : « Mes règles » de la rubrique Mon style (crate::style, `vue`)
    pub regles: Value,
    /// Chaque partie de clé, et ce que l'écran en sait (rien : pas encore donnée)
    pub cles: Vec<(Cle, Option<EtatCle>)>,
}

/// Rien encore (les tests, une base qui ne s'ouvre pas).
pub static VIDE: VueProfil = VueProfil {
    cvs: Vec::new(),
    cv_ouvert: None,
    import: None,
    style: Vec::new(),
    style_ouvert: None,
    recherche: None,
    regles: Value::Null,
    cles: Vec::new(),
};

/// Les textes d'une rubrique (la page les pose tels quels), en objet.
fn textes(paires: &[(&str, &str)]) -> Value {
    Value::Object(paires.iter().map(|(k, v)| ((*k).to_string(), json!(v))).collect())
}

/// « 182 Ko », « 1,4 Mo ».
pub fn taille_texte(octets: i64) -> String {
    if octets < 1024 * 1024 {
        format!("{} Ko", (octets + 1023) / 1024)
    } else {
        let dixiemes = (octets * 10 + 512 * 1024) / (1024 * 1024);
        if dixiemes % 10 == 0 {
            format!("{} Mo", dixiemes / 10)
        } else {
            format!("{},{} Mo", dixiemes / 10, dixiemes % 10)
        }
    }
}

/// « PDF, importé le 3 nov. », « Collé le 18 nov. ».
fn sous_cv(c: &Cv, aujourdhui: Jour) -> String {
    let quand = c.importe_le.court(aujourdhui);
    match c.genre {
        GenreCv::Pdf => format!("PDF, importé le {quand}"),
        GenreCv::Docx => format!("Word, importé le {quand}"),
        GenreCv::Texte => format!("Collé le {quand}"),
    }
}

/// « importé le 3 nov. », « collé le 18 nov. ».
fn quand_cv(c: &Cv, aujourdhui: Jour) -> String {
    let quand = c.importe_le.court(aujourdhui);
    if c.genre == GenreCv::Texte { format!("collé le {quand}") } else { format!("importé le {quand}") }
}

/// Le CV ouvert : celui choisi, sinon le principal.
fn cv_ouvert(v: &VueProfil) -> Option<&Cv> {
    v.cv_ouvert
        .and_then(|id| v.cvs.iter().find(|c| c.id == id))
        .or_else(|| v.cvs.iter().find(|c| c.principal))
        .or(v.cvs.first())
}

fn rubrique_cv(v: &VueProfil, aujourdhui: Jour) -> Value {
    let principal = v.cvs.iter().find(|c| c.principal);
    let sous = match (v.cvs.len(), principal) {
        (0, _) => "Pas encore donné".to_string(),
        (1, Some(p)) => format!("« {} », {}", p.nom, quand_cv(p, aujourdhui)),
        (n, Some(p)) => format!("{n} CV, le principal : « {} »", p.nom),
        (n, None) => format!("{n} CV"),
    };
    let ouvert = cv_ouvert(v).map(|c| {
        let fichier = match (c.genre, c.taille) {
            (GenreCv::Texte, _) => "Collé à la main : aucun fichier gardé.".to_string(),
            (g, t) => format!(
                "Le fichier {}{} est gardé sur ce PC : il sera joint à tes candidatures.",
                if g == GenreCv::Pdf { "PDF" } else { "Word" },
                t.map(|t| format!(" ({})", taille_texte(t))).unwrap_or_default()
            ),
        };
        json!({
            "id": c.id,
            "nom": c.nom,
            "texte": c.texte,
            "principal": c.principal,
            "sous": sous_cv(c, aujourdhui),
            "fichier": fichier,
            "sans_texte": c.texte.trim().is_empty(),
            "mots": if c.texte.trim().is_empty() { String::new() } else { format!("{} mots", mots(&c.texte)) },
            "question": format!("Supprimer « {} » ? Son texte et son fichier partent avec lui.", c.nom),
        })
    });
    let import = match &v.import {
        None => Value::Null,
        Some(None) => json!({ "texte": "Choisis ton CV dans la fenêtre qui vient de s'ouvrir." }),
        Some(Some(nom)) => json!({ "texte": format!("Lecture de « {nom} »...") }),
    };
    json!({
        "id": "cv",
        "nom": "Mon CV",
        "sous": sous,
        "icone": "cv",
        "dossier": {
            "vue": "cv",
            "icone": "cv",
            "titre": "Mon CV",
            "intro": "Jobmind prépare chaque candidature avec ce qui est écrit dans ton CV, et rien d'autre.",
            "cvs": v.cvs.iter().map(|c| json!({ "id": c.id, "nom": c.nom, "sous": sous_cv(c, aujourdhui), "principal": c.principal })).collect::<Vec<_>>(),
            "ouvert": ouvert,
            "import": import,
            "plein": v.cvs.len() >= CV_NOMBRE_MAX,
            "longueur": CV_TEXTE_MAX,
            "vide": {
                "titre": "Aucun CV pour l'instant.",
                "texte": "Donne ton CV à Jobmind : elle s'en servira pour préparer chaque candidature. Choisis le \
                          fichier de ton CV, ou colle son texte.",
                "note": "Le fichier reste sur ce PC. Il sera joint à tes candidatures.",
            },
            "textes": textes(&[
                ("importer", "Importer un fichier"),
                ("formats", "PDF ou Word (.docx), 15 Mo au plus."),
                ("coller", "Coller le texte"),
                ("autre", "Ajouter un autre CV"),
                ("mes_cv", "Mes CV"),
                ("nom", "Nom du CV"),
                ("i_nom", "Par exemple : CV 2026"),
                ("texte", "Le texte de ton CV"),
                ("aide", "Relis ce texte et corrige ce qui a été mal lu : c'est lui qui servira à tes candidatures."),
                ("i_texte", "Colle ici le texte de ton CV."),
                ("sans_texte", "Le texte de ce fichier n'a pas pu être lu. Colle ici le texte de ton CV : le fichier, \
                               lui, est gardé."),
                ("enregistrer", "Enregistrer"),
                ("annuler", "Annuler"),
                ("principal", "CV principal"),
                ("rendre_principal", "En faire mon CV principal"),
                ("supprimer", "Supprimer ce CV"),
                ("oui", "Oui, supprimer"),
                ("non", "Non, garder"),
                ("coller_titre", "Coller le texte d'un CV"),
                ("coller_intro", "Ouvre ton CV, sélectionne tout son texte (Ctrl + A), copie-le (Ctrl + C), puis \
                                 colle-le ici (Ctrl + V)."),
                ("plein", &format!("Tu as déjà {CV_NOMBRE_MAX} CV : supprime-en un pour en ajouter un autre.")),
            ]),
        },
    })
}

fn rubrique_recherche(v: &VueProfil) -> Value {
    let r = v.recherche.as_ref();
    let sous = r.map_or_else(|| "Rien de noté pour l'instant".to_string(), Recherche::resume);
    let (lieu, ville, rayon, departement, region) = match r.map(|r| &r.lieu) {
        Some(Lieu::Ville { nom, rayon }) => ("ville", nom.as_str(), *rayon, "", ""),
        Some(Lieu::Departement(d)) => ("departement", "", RAYON_DEFAUT, d.code, ""),
        Some(Lieu::Region(x)) => ("region", "", RAYON_DEFAUT, "", x.code),
        Some(Lieu::Partout) => ("partout", "", RAYON_DEFAUT, "", ""),
        None => ("ville", "", RAYON_DEFAUT, "", ""),
    };
    let valeurs = json!({
        "postes": r.map(|r| r.postes.join(", ")).unwrap_or_default(),
        "lieu": lieu,
        "ville": ville,
        "rayon": rayon,
        "departement": departement,
        "region": region,
        "contrats": r.map(|r| r.contrats.iter().map(|c| c.code()).collect::<Vec<_>>()).unwrap_or_default(),
        "teletravail": r.map_or(Teletravail::PeuImporte, |r| r.teletravail).code(),
        "sans_diplome": r.is_some_and(|r| r.sans_diplome),
        "debutant": r.is_some_and(|r| r.debutant),
        "salaire": r.and_then(|r| r.salaire_min).map(|n| n.to_string()).unwrap_or_default(),
    });
    let bilan: Vec<String> = match r {
        Some(r) => {
            let mut l = vec![r.lieu.texte(), r.contrats_texte()];
            if let Some(s) = r.salaire_texte() {
                l.push(s);
            }
            l
        }
        None => Vec::new(),
    };
    json!({
        "id": "recherche",
        "nom": "Ce que je cherche",
        "sous": sous,
        "icone": "cible",
        "dossier": {
            "vue": "recherche",
            "icone": "cible",
            "titre": "Ce que je cherche",
            "intro": "Jobmind s'en servira pour chercher les offres et les trier. Les autres offres resteront \
                      visibles, rangées à part.",
            "valeurs": valeurs,
            "enregistree": r.is_some(),
            "bilan": bilan,
            "lieux": GenreLieu::TOUS.iter().map(|g| json!({ "id": g.code(), "nom": g.nom() })).collect::<Vec<_>>(),
            "rayons": RAYONS,
            "departements": DEPARTEMENTS.iter().map(|d| json!({ "id": d.code, "nom": d.libelle() })).collect::<Vec<_>>(),
            "regions": REGIONS.iter().map(|x| json!({ "id": x.code, "nom": x.nom })).collect::<Vec<_>>(),
            "liste_contrats": Contrat::TOUS.iter().map(|c| json!({ "id": c.code(), "nom": c.nom() })).collect::<Vec<_>>(),
            "liste_teletravail": Teletravail::TOUS.iter().map(|t| json!({ "id": t.code(), "nom": t.nom() })).collect::<Vec<_>>(),
            "textes": textes(&[
                ("postes", "Les postes ou les mots que je cherche"),
                ("i_postes", "Par exemple : agent de quai, cariste, vendeur"),
                ("postes_aide", "N'importe quel métier. Sépare-les par des virgules. Vide : tous les métiers du lieu."),
                ("lieu", "Où"),
                ("ville", "Ville"),
                ("i_ville", "Par exemple : Metz"),
                ("rayon", "Jusqu'à"),
                ("km", "km"),
                ("departement", "Département"),
                ("region", "Région"),
                ("choisir", "Choisis dans la liste"),
                ("contrats", "Contrats"),
                ("contrats_aide", "Aucun choisi : tous les contrats."),
                ("teletravail", "Télétravail"),
                ("avant", "Mettre en avant les offres"),
                ("sans_diplome", "Aucun diplôme requis"),
                ("debutant", "Débutant accepté"),
                ("salaire", "Salaire minimum"),
                ("facultatif", "facultatif"),
                ("i_salaire", "Par exemple : 1 800"),
                ("salaire_aide", "En euros, brut par mois."),
                ("enregistrer", "Enregistrer"),
                ("deja", "Enregistré. Change ce que tu veux, puis « Enregistrer »."),
            ]),
        },
    })
}

fn rubrique_style(v: &VueProfil, aujourdhui: Jour) -> Value {
    let n = v.style.len();
    let sous = match n {
        0 => "Aucun texte pour l'instant".to_string(),
        1 => "1 texte".to_string(),
        n => format!("{n} textes"),
    };
    let ouvert = v.style_ouvert.and_then(|id| v.style.iter().find(|t| t.id == id)).map(|t| {
        json!({
            "id": t.id,
            "titre": t.titre,
            "texte": t.texte,
            "question": format!("Supprimer « {} » ?", t.titre),
        })
    });
    json!({
        "id": "style",
        "nom": "Mon style",
        "sous": sous,
        "icone": "plume",
        "dossier": {
            "vue": "style",
            "icone": "plume",
            "titre": "Mon style",
            "intro": "Colle ici des textes que tu as écrits toi-même : des lettres, des mails, des messages. Jobmind y \
                      apprendra ta façon d'écrire, tes mots et la longueur de tes phrases. Tes candidatures resteront \
                      toujours professionnelles.",
            "conseil": "Trois à cinq textes de quelques lignes suffisent.",
            "note": "Claude s'en sert pour réécrire tes lettres dans ton style (Ma candidature, « Dans mon style »). Il y prend ta façon d'écrire, jamais des faits.",
            "liste": v.style.iter().map(|t| json!({
                "id": t.id,
                "titre": t.titre,
                "sous": format!("{} mots, ajouté le {}", mots(&t.texte), t.ajoute_le.court(aujourdhui)),
                "extrait": debut(&t.texte, 150),
            })).collect::<Vec<_>>(),
            "ouvert": ouvert,
            "regles": v.regles,
            "plein": n >= STYLE_NOMBRE_MAX,
            "longueur": STYLE_TEXTE_MAX,
            "vide": {
                "titre": "Aucun texte de toi pour l'instant.",
                "texte": "Quelques textes écrits par toi apprendront à Jobmind ta façon d'écrire.",
            },
            "textes": textes(&[
                ("ajouter", "Ajouter un texte"),
                ("nouveau", "Un texte écrit par toi"),
                ("titre", "Titre"),
                ("facultatif", "facultatif"),
                ("i_titre", "Par exemple : Lettre au magasin de sport"),
                ("texte", "Le texte"),
                ("i_texte", "Colle ici un texte que tu as écrit toi-même."),
                ("enregistrer", "Enregistrer"),
                ("annuler", "Annuler"),
                ("lire", "Lire"),
                ("fermer", "Fermer"),
                ("supprimer", "Supprimer"),
                ("oui", "Oui, supprimer"),
                ("non", "Non, garder"),
                ("plein", &format!("Tu as déjà {STYLE_NOMBRE_MAX} textes : supprime-en un pour en ajouter un autre.")),
            ]),
        },
    })
}

/// Ce qu'un service demande, et où trouver sa clé.
fn service_json(s: Service, v: &VueProfil) -> Value {
    let (nom, role, texte, adresse) = match s {
        Service::Claude => (
            "Claude",
            "L'IA qui prépare tes candidatures. Payante : quelques centimes par lettre.",
            "Crée un compte sur la console de Claude, ajoute un moyen de paiement, puis crée une clé dans la page \
             « API Keys ». Copie-la ici tout de suite : elle n'est montrée qu'une fois.",
            "https://console.anthropic.com/settings/keys",
        ),
        // (jamais dans Mes clés : la rubrique « Ma boîte mail », crate::mail)
        Service::Gmail => ("Gmail", "", "", ""),
        // (jamais dans Mes clés : la carte de connexion de chaque site, crate::coulisse::garde)
        Service::Sites => ("Sites", "", "", ""),
    };
    let etat_de = |c: Cle| v.cles.iter().find(|(k, _)| *k == c).and_then(|(_, e)| e.clone());
    let parties: Vec<Value> = s
        .parties()
        .iter()
        .map(|c| {
            let (apercu, illisible) = match etat_de(*c) {
                Some(EtatCle::Gardee(a)) => (a, false),
                Some(EtatCle::Illisible) => (String::new(), true),
                None => (String::new(), false),
            };
            json!({
                "id": c.code(),
                "nom": c.nom(),
                "apercu": apercu,
                "illisible": illisible,
                "indice": match c {
                    Cle::Claude => "Commence par sk-ant-",
                    _ => "",
                },
            })
        })
        .collect();
    let etats: Vec<Option<EtatCle>> = s.parties().iter().map(|c| etat_de(*c)).collect();
    let (etat, etat_nom) = if etats.contains(&Some(EtatCle::Illisible)) {
        ("illisible", "Illisible sur ce PC : remplace-la")
    } else if etats.iter().all(|e| matches!(e, Some(EtatCle::Gardee(_)))) {
        ("gardee", "Enregistrée")
    } else if etats.iter().any(Option::is_some) {
        ("partielle", "Incomplète : donne-la de nouveau")
    } else {
        ("absente", "Pas encore donnée")
    };
    json!({
        "id": s.code(),
        "nom": nom,
        "role": role,
        "texte": texte,
        "adresse": adresse,
        "etat": etat,
        "etat_nom": etat_nom,
        "parties": parties,
        "question": format!("Effacer la clé {nom} ? Jobmind ne pourra plus s'en servir."),
    })
}

/// Le nombre de services dont la clé est entière et lisible.
fn services_prets(v: &VueProfil) -> usize {
    Service::TOUS
        .iter()
        .filter(|s| {
            s.parties().iter().all(|c| v.cles.iter().any(|(k, e)| k == c && matches!(e, Some(EtatCle::Gardee(_)))))
        })
        .count()
}

fn rubrique_cles(v: &VueProfil) -> Value {
    let n = services_prets(v);
    let total = Service::TOUS.len();
    let sous = match (n, total) {
        (0, _) => "Aucune clé pour l'instant".to_string(),
        (1, 1) => "Ta clé Claude est là".to_string(),
        (n, t) if n == t => format!("Les {t} clés sont là"),
        (1, t) => format!("1 clé sur {t}"),
        (n, t) => format!("{n} clés sur {t}"),
    };
    json!({
        "id": "cles",
        "nom": "Mes clés",
        "sous": sous,
        "icone": "cle",
        "dossier": {
            "vue": "cles",
            "icone": "cle",
            "titre": "Mes clés",
            "intro": "L'IA Claude demande une clé : un code personnel, créé une seule fois sur son site. Jobmind la \
                      garde chiffrée sur ce PC : personne d'autre ne peut la lire.",
            "services": Service::TOUS.iter().map(|s| service_json(*s, v)).collect::<Vec<_>>(),
            "note": "La clé de Claude sert à préparer tes candidatures et à lire les offres que Jobmind n'a pas su lire \
                     en entier. Les sites d'offres, eux, ne demandent pas de clé.",
            "textes": textes(&[
                ("donner", "Donner la clé"),
                ("remplacer", "Remplacer"),
                ("effacer", "Effacer"),
                ("enregistrer", "Enregistrer"),
                ("annuler", "Annuler"),
                ("ou", "Où la trouver"),
                ("copier", "Copier l'adresse"),
                ("copie", "Copié"),
                ("adresse_aide", "Copie cette adresse, puis ouvre-la dans ton navigateur."),
                ("oui", "Oui, effacer"),
                ("non", "Non, garder"),
            ]),
        },
    })
}

/// Les rubriques du profil, dans l'ordre de l'écran (Réglages vient après, d'ecran.rs).
pub fn rubriques(v: &VueProfil, aujourdhui: Jour) -> Vec<Value> {
    vec![rubrique_cv(v, aujourdhui), rubrique_recherche(v), rubrique_style(v, aujourdhui), rubrique_cles(v)]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use jobmind_core::recherche::{SaisieRecherche, verifier_recherche};

    fn j(m: u32, d: u32) -> Jour {
        Jour::nouveau(2026, m, d).unwrap()
    }

    /// Un profil rempli de tout : trois CV (PDF principal, Word sans texte, collé), deux textes (l'un ouvert), ce que
    /// je cherche, une clé illisible, une partielle.
    pub(crate) fn exemple() -> VueProfil {
        let cv = |id, nom: &str, genre, texte: &str, principal| Cv {
            id,
            nom: nom.into(),
            fichier: (genre != GenreCv::Texte).then(|| format!(r"cv\cv-2026111{id}-100000.pdf")),
            genre,
            texte: texte.into(),
            importe_le: j(11, id as u32),
            change_le: None,
            principal,
            taille: (genre != GenreCv::Texte).then_some(182_000),
        };
        let r = verifier_recherche(&SaisieRecherche {
            postes: "Agent de quai, cariste, vendeur".into(),
            lieu: "ville".into(),
            ville: "Metz".into(),
            rayon: 30,
            contrats: vec!["cdi".into()],
            teletravail: "possible".into(),
            salaire: "1800".into(),
            ..SaisieRecherche::default()
        })
        .unwrap();
        VueProfil {
            cvs: vec![
                cv(1, "CV 2026", GenreCv::Pdf, "Jordan Exemple\nCariste", true),
                cv(2, "CV scanné", GenreCv::Docx, "", false),
                cv(3, "CV vente", GenreCv::Texte, "Jordan Exemple, vendeur", false),
            ],
            cv_ouvert: Some(2),
            import: Some(Some("CV.pdf".into())),
            style: vec![
                TexteStyle {
                    id: 1,
                    titre: "Lettre".into(),
                    texte: "Madame, Monsieur, je vous écris.".into(),
                    ajoute_le: j(11, 3),
                },
                TexteStyle {
                    id: 2,
                    titre: "Mail".into(),
                    texte: "Bonjour, je reviens vers vous.".into(),
                    ajoute_le: j(11, 4),
                },
            ],
            style_ouvert: Some(2),
            recherche: Some(r),
            regles: Value::Null,
            cles: vec![(Cle::Claude, Some(EtatCle::Illisible))],
        }
    }

    #[test]
    fn un_profil_rempli() {
        let v = exemple();
        let r = rubriques(&v, j(11, 18));
        let sous: Vec<&str> = r.iter().map(|x| x["sous"].as_str().unwrap()).collect();
        assert_eq!(
            sous,
            [
                "3 CV, le principal : « CV 2026 »",
                "Agent de quai, cariste et 1 autre",
                "2 textes",
                "Aucune clé pour l'instant"
            ]
        );
        let cv = &r[0]["dossier"];
        assert_eq!(cv["ouvert"]["id"], 2);
        assert_eq!(cv["ouvert"]["sans_texte"], true);
        assert_eq!(
            cv["ouvert"]["fichier"],
            "Le fichier Word (178 Ko) est gardé sur ce PC : il sera joint à tes candidatures."
        );
        assert_eq!(cv["import"]["texte"], "Lecture de « CV.pdf »...");
        assert_eq!(cv["cvs"][2]["sous"], "Collé le 3 nov.");
        let rc = &r[1]["dossier"];
        assert_eq!(rc["valeurs"]["contrats"], json!(["cdi"]));
        assert_eq!(rc["valeurs"]["salaire"], "1800");
        assert_eq!(rc["bilan"], json!(["Metz, 30 km autour", "CDI", "1 800 € brut par mois au moins"]));
        assert_eq!(r[2]["dossier"]["ouvert"]["texte"], "Bonjour, je reviens vers vous.");
        let s = &r[3]["dossier"]["services"];
        let etats: Vec<&str> = s.as_array().unwrap().iter().map(|x| x["etat"].as_str().unwrap()).collect();
        assert_eq!(etats, ["illisible"]);
        assert_eq!(s[0]["parties"][0]["apercu"], "");
        assert!(s[0].get("essai").is_none());
        // La clé Claude lisible : la seule clé de Mes clés (Adzuna retiré)
        let mut une = v.clone();
        une.cles[0].1 = Some(EtatCle::Gardee("se termine par AAAA".into()));
        assert_eq!(rubriques(&une, j(11, 18))[3]["sous"], "Ta clé Claude est là");
        assert_eq!(
            rubriques(&une, j(11, 18))[3]["dossier"]["services"][0]["parties"][0]["apercu"],
            "se termine par AAAA"
        );
        // Un seul CV, collé : sa ligne dit quand
        let un = VueProfil { cvs: vec![Cv { principal: true, ..v.cvs[2].clone() }], ..VueProfil::default() };
        assert_eq!(rubriques(&un, j(11, 18))[0]["sous"], "« CV vente », collé le 3 nov.");
        let pdf = VueProfil { cvs: vec![v.cvs[0].clone()], ..VueProfil::default() };
        assert_eq!(rubriques(&pdf, j(11, 18))[0]["sous"], "« CV 2026 », importé le 1 nov.");
    }
    #[test]
    fn tailles_en_mots() {
        assert_eq!(taille_texte(500), "1 Ko");
        assert_eq!(taille_texte(182_000), "178 Ko");
        assert_eq!(taille_texte(1024 * 1024), "1 Mo");
        assert_eq!(taille_texte(1_468_006), "1,4 Mo");
    }
}
