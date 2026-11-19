//! Ce que la page de Jobmind montre : l'état du kit d'écrans de la suite (format en tête de ui\kit.js).
//!
//! L'accueil est celui de la maquette C (maquettes\proposition-c.html, « les dossiers ») : trois onglets,
//! Candidatures, Offres et Mon profil, chacun avec une liste fine à gauche et le dossier ouvert à droite. Il est
//! dessiné par ui\jobmind.js à la place des tuiles du kit (bloc `accueil.perso`, voir « perso » en tête de kit.js) ;
//! le kit garde l'en-tête, le bas, le clavier et la manette, et les sons.
//!
//! Étape 2 : l'onglet Candidatures montre le suivi tapé à la main (jobmind-core, `suivi`) : la liste groupée « À faire
//! maintenant », « En attente », « Terminées », chaque ligne avec sa pastille de statut et sa ligne d'action, le petit
//! bilan du mois en bas ; à droite, le dossier ouvert (statut à changer, « où on en est », la prochaine action et ses
//! boutons, la frise, « Ce que j'ai envoyé », « L'offre », « Notes »). Le formulaire d'ajout est dessiné par la page
//! avec les textes envoyés ici. Mon profil > Réglages : le délai avant relance. Offres reste vide (étape 4).
//!
//! Étape 3 : Mon profil montre ses rubriques Mon CV, Ce que je cherche, Mon style et Mes clés (vue_profil.rs), puis
//! Réglages.
//!
//! Étape 4 : l'onglet Offres (vue_offres.rs) : les filtres, les recherches gardées et « Chercher maintenant », la liste
//! des offres gardées, l'offre ouverte.
//!
//! Tous les textes vont à la page tels quels (elle les échappe) : en français simple, sans tiret long ni demi-cadratin.

use jobmind_core::candidature::{self, Candidature, Evenement, Moyen};
use jobmind_core::suivi::{
    self, Action, DELAI_MAX, DELAI_MIN, Groupe, cadre_action, ligne_action, message_relance, ou_on_en_est,
    phrase_calme, situation,
};
use jobmind_core::{Jour, Statut};
use serde_json::{Value, json};

use crate::offres::VueOffres;
use crate::vue_profil::{self, VIDE, VueProfil};

/// Les onglets, dans l'ordre.
pub const CANDIDATURES: &str = "candidatures";
pub const OFFRES: &str = "offres";
pub const PROFIL: &str = "profil";
pub const ONGLETS: [&str; 3] = [CANDIDATURES, OFFRES, PROFIL];

/// La phrase de l'onglet Candidatures, tant qu'il est vide.
pub const AUCUNE_CANDIDATURE: &str = "Aucune candidature pour l'instant.";
/// La phrase de l'onglet Offres, tant qu'il est vide.
pub const AUCUNE_OFFRE: &str = "Aucune offre pour l'instant.";

/// Ce que l'écran montre : les candidatures, celle dont le dossier est ouvert (et sa frise), le délai de relance et le
/// jour qu'il est.
#[derive(Debug, Clone, Copy)]
pub struct Vue<'a> {
    pub candidatures: &'a [Candidature],
    pub ouverte: Option<i64>,
    pub frise: &'a [Evenement],
    pub delai: u32,
    pub aujourdhui: Jour,
    /// Mon profil (étape 3) : les CV, ce que je cherche, les textes pour le style, les clés
    pub profil: &'a VueProfil,
    /// L'onglet Offres (étape 4) : la liste filtrée, l'offre ouverte, les recherches gardées
    pub offres: &'a VueOffres,
}

impl Vue<'_> {
    /// Rien encore (les tests, une base qui ne s'ouvre pas).
    pub fn vide(aujourdhui: Jour) -> Vue<'static> {
        Vue {
            candidatures: &[],
            ouverte: None,
            frise: &[],
            delai: suivi::DELAI_DEFAUT,
            aujourdhui,
            profil: &VIDE,
            offres: &crate::offres::VIDE,
        }
    }
}

/// Un dossier vide (le grand panneau de droite) : son icône, sa phrase, ce qui viendra, et une note.
fn dossier_vide(icone: &str, titre: &str, texte: &str, note: &str) -> Value {
    json!({ "icone": icone, "titre": titre, "texte": texte, "note": note })
}

/// La rubrique Réglages : le délai avant relance (le seul réglage de l'étape 2).
fn rubrique_reglages(delai: u32) -> Value {
    json!({
        "id": "reglages",
        "nom": "Réglages",
        "sous": format!("Relance après {delai} jours"),
        "icone": "reglages",
        "dossier": {
            "icone": "reglages",
            "titre": "Réglages",
            "reglage": {
                "id": "relance",
                "titre": "Relancer après",
                "texte": "Une candidature restée sans réponse passe dans « À faire maintenant » après ce nombre de \
                          jours, avec un message de relance prêt à copier. Après une relance restée sans réponse aussi \
                          longtemps, Jobmind propose de la classer en « Sans réponse ».",
                "valeur": delai,
                "min": DELAI_MIN,
                "max": DELAI_MAX,
                "unite": "jours",
                "choix": [7, 10, 14, 21, 30],
                "moins": "Un jour de moins",
                "plus": "Un jour de plus",
            },
            // (étapes 5 et 5 bis : la session des sites lus par Jobmind)
            "sites": {
                "titre": "Tes connexions aux sites",
                "texte": "Jobmind garde tes connexions à LinkedIn, Indeed, Welcome to the Jungle et France Travail sur \
                          ce PC, seulement pour lui. Ton mot de passe, lui, n'est jamais gardé.",
                "bouton": "Me déconnecter de tous les sites",
                "question": "Te déconnecter de tous les sites ? Tu devras te reconnecter à tes comptes la prochaine \
                             fois.",
                "oui": "Oui, me déconnecter",
                "non": "Annuler",
            },
            "note": "Ta boîte Gmail, la sauvegarde et les exports viendront ici.",
        },
    })
}

/// Les textes du formulaire et du dossier (la page les pose tels quels).
fn textes(aujourdhui: Jour) -> Value {
    let paires: [(&str, &str); 46] = [
        ("ajouter", "Ajouter une candidature"),
        ("ajouter_court", "Ajouter"),
        ("nouvelle", "Nouvelle candidature"),
        ("modifier_titre", "Modifier la candidature"),
        ("intro", "Note une candidature que tu as déjà envoyée. Seuls l'entreprise et le poste sont obligatoires."),
        ("entreprise", "Entreprise"),
        ("poste", "Poste"),
        ("ville", "Ville"),
        ("date", "Date d'envoi"),
        ("moyen", "Par où"),
        ("source", "Offre vue sur"),
        ("lien", "Lien de l'offre"),
        ("envoye", "Ce que j'ai envoyé"),
        ("cv", "CV utilisé"),
        ("notes", "Notes"),
        ("facultatif", "facultatif"),
        ("i_entreprise", "Par exemple : Transports Moselle Fret"),
        ("i_poste", "Par exemple : Agent de quai"),
        ("i_ville", "Par exemple : Metz"),
        ("i_date", "JJ/MM/AAAA"),
        ("i_source", "LinkedIn, Indeed, France Travail..."),
        ("i_lien", "https://..."),
        ("i_envoye", "Colle ici ta lettre ou ton message."),
        ("i_cv", "Par exemple : CV 2026.pdf"),
        ("i_notes", "Le nom du contact, ce qu'on t'a dit..."),
        ("enregistrer", "Enregistrer"),
        ("annuler", "Annuler"),
        ("modifier", "Modifier"),
        ("supprimer", "Supprimer"),
        ("oui_supprimer", "Oui, supprimer"),
        ("non", "Non, garder"),
        ("p_envoye", "Ce que j'ai envoyé"),
        ("p_offre", "L'offre"),
        ("p_notes", "Notes"),
        ("notes_aide", "Gardé tout seul quand tu quittes le champ."),
        ("copier", "Copier"),
        ("copie", "Copié"),
        ("copier_lien", "Copier le lien"),
        ("statut", "Changer le statut"),
        ("entretien_titre", "L'entretien"),
        ("entretien_date", "Date"),
        ("entretien_heure", "Heure"),
        ("entretien_lieu", "Lieu"),
        ("i_heure", "9 h 30"),
        ("i_lieu", "L'adresse, ou « au téléphone »"),
        ("choisir", "Choisis une candidature à gauche."),
    ];
    let mut m: serde_json::Map<String, Value> = paires.iter().map(|(k, v)| ((*k).to_string(), json!(v))).collect();
    m.insert("aujourdhui".into(), json!(aujourdhui.saisie()));
    m.insert("calendrier".into(), calendrier(aujourdhui));
    Value::Object(m)
}

/// Le petit calendrier à cliquer des dates (la date d'envoi, celle de l'entretien) : ses textes, aujourd'hui, et les
/// jours qu'il laisse choisir pour chacune (les mêmes que la vérification, qui reste la seule qui compte).
fn calendrier(aujourdhui: Jour) -> Value {
    let bornes = |(min, max): (Jour, Jour)| json!({ "min": min.iso(), "max": max.iso() });
    json!({
        "ouvrir": "Choisir dans le calendrier",
        "fermer": "Fermer",
        "avant": "Mois d'avant",
        "apres": "Mois d'après",
        "mois": (1..=12).filter_map(|m| Jour::nouveau(2000, m, 1)).map(Jour::nom_du_mois).collect::<Vec<_>>(),
        // (du lundi au dimanche)
        "jours": ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."],
        "aujourdhui": aujourdhui.iso(),
        "envoi": bornes(candidature::bornes_envoi(aujourdhui)),
        "entretien": bornes(candidature::bornes_entretien(aujourdhui)),
        "titre_envoi": "Choisis le jour de l'envoi",
        "titre_entretien": "Choisis le jour de l'entretien",
    })
}

/// Le petit texte « Agent de quai, Woippy ».
fn poste_ville(c: &Candidature) -> String {
    if c.ville.is_empty() { c.poste.clone() } else { format!("{}, {}", c.poste, c.ville) }
}

/// L'icône du cadre « prochaine action ».
fn icone_action(a: &Action) -> &'static str {
    match a {
        Action::Relancer { .. } => "relance",
        Action::ClasserSansReponse { .. } => "horloge",
        Action::Entretien { .. } => "agenda",
    }
}

/// Les boutons du cadre « prochaine action » : (id, texte, principal).
fn boutons_action(a: &Action) -> Value {
    match a {
        Action::Relancer { .. } => json!([
            { "id": "copier", "texte": "Copier un message", "icone": "copie" },
            { "id": "relancee", "texte": "C'est relancé", "icone": "coche", "principal": true },
        ]),
        Action::ClasserSansReponse { .. } => json!([
            { "id": "sans_reponse", "texte": "Classer en Sans réponse", "principal": true },
        ]),
        Action::Entretien { .. } => json!([
            { "id": "entretien", "texte": "Changer l'entretien", "icone": "crayon" },
        ]),
    }
}

/// « Par mail, le 2 nov., avec « CV 2026.pdf » ».
fn meta_envoi(c: &Candidature, aujourdhui: Jour) -> String {
    let mut t = match c.moyen {
        Moyen::Autre => String::new(),
        m => format!("{}, ", m.nom()),
    };
    t.push_str(&format!("le {}", c.envoyee_le.court(aujourdhui)));
    if !c.cv.is_empty() {
        t.push_str(&format!(", avec « {} »", c.cv));
    }
    let mut c = t.chars();
    match c.next() {
        Some(p) => p.to_uppercase().chain(c).collect(),
        None => t,
    }
}

/// Le dossier d'une candidature (le grand panneau de droite).
pub fn fiche(c: &Candidature, frise: &[Evenement], delai: u32, aujourdhui: Jour) -> Value {
    let s = situation(c, aujourdhui, delai);
    let (ou, ou_sous) = ou_on_en_est(c, aujourdhui);
    let (action, calme, relance) = match &s.action {
        Some(a) => {
            let (titre, texte) = cadre_action(c, a, aujourdhui);
            let relance = matches!(a, Action::Relancer { .. }).then(|| message_relance(c, aujourdhui));
            (
                json!({ "icone": icone_action(a), "titre": titre, "texte": texte, "boutons": boutons_action(a) }),
                Value::Null,
                relance,
            )
        }
        None => (
            Value::Null,
            json!({
                "icone": if s.groupe == Groupe::Terminee { "coche" } else { "horloge" },
                "texte": phrase_calme(c, &s, aujourdhui),
            }),
            None,
        ),
    };
    let frise: Vec<Value> =
        frise.iter().map(|e| json!({ "texte": e.statut.nom(), "date": e.jour.court(aujourdhui) })).collect();
    let entretien = match &c.entretien {
        Some(e) => json!({
            "date": e.jour.saisie(),
            "heure": e.heure.map(|h| h.texte()).unwrap_or_default(),
            "lieu": e.lieu,
        }),
        None => json!({ "date": "", "heure": "", "lieu": "" }),
    };
    json!({
        "id": c.id,
        "titre": c.entreprise,
        "sous": poste_ville(c),
        "statut": c.statut.code(),
        "statut_nom": c.statut.nom(),
        "ou": ou,
        "ou_sous": ou_sous,
        "action": action,
        "calme": calme,
        "relance": relance,
        "frise": frise,
        "envoye": {
            "meta": meta_envoi(c, aujourdhui),
            "texte": c.envoye,
            "vide": "Rien de noté. Choisis « Modifier » pour coller ce que tu as envoyé.",
        },
        "offre": {
            "lien": c.lien,
            "texte": "Le lien de l'offre, à copier pour la revoir dans ton navigateur.",
            "vide": "Aucun lien noté. L'offre elle-même sera gardée ici quand elle viendra de l'onglet Offres.",
        },
        "notes": c.notes,
        "champs": {
            "entreprise": c.entreprise,
            "poste": c.poste,
            "ville": c.ville,
            "date": c.envoyee_le.saisie(),
            "moyen": c.moyen.code(),
            "source": c.source,
            "lien": c.lien,
            "envoye": c.envoye,
            "cv": c.cv,
            "notes": c.notes,
        },
        "entretien": entretien,
        "supprimer": format!("Supprimer la candidature chez {} ? Sa frise et ses notes partent avec elle.", c.entreprise),
    })
}

/// L'onglet Candidatures.
fn onglet_candidatures(v: &Vue) -> Value {
    let ranges = suivi::ranger(v.candidatures, v.aujourdhui, v.delai);
    let a_faire = ranges.iter().find(|(g, _)| *g == Groupe::AFaire).map_or(0, |(_, l)| l.len());
    let groupes: Vec<Value> = ranges
        .iter()
        .map(|(g, l)| {
            let lignes: Vec<Value> = l
                .iter()
                .map(|(c, s)| {
                    json!({
                        "id": c.id,
                        "nom": c.entreprise,
                        "sous": poste_ville(c),
                        "statut": c.statut.code(),
                        "statut_nom": c.statut.nom(),
                        "action": ligne_action(s, v.aujourdhui).unwrap_or_default(),
                        "fini": *g == Groupe::Terminee,
                    })
                })
                .collect();
            json!({ "id": g.code(), "nom": g.nom(), "lignes": lignes })
        })
        .collect();
    let ouverte = v.ouverte.and_then(|id| v.candidatures.iter().find(|c| c.id == id));
    let bilan =
        if v.candidatures.is_empty() { Vec::new() } else { suivi::bilan(v.candidatures, v.aujourdhui).lignes() };
    json!({
        "id": CANDIDATURES,
        "nom": "Candidatures",
        "nombre": a_faire,
        "liste": "Mes candidatures",
        "vide": "Rien à suivre pour l'instant.",
        "rubriques": [],
        "dossier": dossier_vide(
            "dossier",
            AUCUNE_CANDIDATURE,
            "Note ici chaque candidature envoyée : Jobmind te dira quand relancer, et gardera ce que tu as envoyé, \
             les réponses et la prochaine chose à faire.",
            "Tout reste sur ce PC.",
        ),
        "suivi": {
            "groupes": groupes,
            "bilan": bilan,
            "ouverte": ouverte.map(|c| c.id),
            "fiche": ouverte.map(|c| fiche(c, v.frise, v.delai, v.aujourdhui)),
            "statuts": Statut::TOUS.iter().map(|s| json!({ "id": s.code(), "nom": s.nom() })).collect::<Vec<_>>(),
            "moyens": Moyen::TOUS.iter().map(|m| json!({ "id": m.code(), "nom": m.nom() })).collect::<Vec<_>>(),
            "textes": textes(v.aujourdhui),
        },
    })
}

/// Les rubriques de Mon profil : Mon CV, Ce que je cherche, Mon style, Mes clés (vue_profil.rs), puis Réglages.
fn rubriques_profil(v: &Vue) -> Vec<Value> {
    let mut l = vue_profil::rubriques(v.profil, v.aujourdhui);
    l.push(rubrique_reglages(v.delai));
    l
}

/// L'accueil de Jobmind (bloc `accueil.perso`) : les trois onglets et ce que chacun montre.
pub fn accueil(v: &Vue) -> Value {
    json!({
        "onglets": [
            onglet_candidatures(v),
            crate::vue_offres::onglet(v.offres, v.aujourdhui),
            {
                "id": PROFIL,
                "nom": "Mon profil",
                "liste": "Mon profil",
                "vide": "",
                "rubriques": rubriques_profil(v),
                "dossier": dossier_vide("profil", "Mon profil", "", ""),
            },
        ],
    })
}

/// L'état de la page : le nom, la version et le logo de l'appli, et l'accueil (les trois onglets). Le thème, les
/// couleurs d'un pack et ses images s'ajoutent par l'habillage ([`crate::Page::habiller`]).
pub fn etat(version: &str, v: &Vue) -> Value {
    json!({
        "app": { "nom": crate::NOM_APPLI, "version": version, "logo": "logo.svg" },
        "accueil": {
            // (aucune tuile : les onglets de la maquette C prennent toute la place)
            "tuiles": [],
            "ronds": [],
            "perso": accueil(v),
        },
        "pages": {},
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offres::VIDE as OVIDE;
    use jobmind_core::candidature::Entretien;
    use jobmind_core::{Heure, Moyen};

    fn auj() -> Jour {
        Jour::nouveau(2026, 11, 18).unwrap()
    }

    /// Tous les textes d'une valeur (récursivement).
    fn textes_de(v: &Value, l: &mut Vec<String>) {
        match v {
            Value::String(s) => l.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| textes_de(x, l)),
            Value::Object(o) => o.values().for_each(|x| textes_de(x, l)),
            _ => {}
        }
    }

    pub(crate) fn exemples() -> Vec<Candidature> {
        let j = |m, d| Jour::nouveau(2026, m, d).unwrap();
        let c = |id, e: &str, p: &str, v: &str, st, env| Candidature {
            id,
            entreprise: e.into(),
            poste: p.into(),
            ville: v.into(),
            statut: st,
            statut_le: env,
            envoyee_le: env,
            moyen: Moyen::Site,
            source: "Indeed".into(),
            lien: String::new(),
            envoye: "Madame, Monsieur,\n\nJe vous adresse ma candidature.".into(),
            cv: "CV 2026.pdf".into(),
            notes: String::new(),
            relance_le: None,
            entretien: None,
        };
        let mut l = vec![
            c(1, "Transports Moselle Fret", "Agent de quai", "Woippy", Statut::Envoyee, j(11, 2)),
            c(2, "Netéa Services", "Agent d'entretien", "Metz", Statut::Envoyee, j(11, 16)),
            c(
                3,
                "Logistique des Trois Frontières",
                "Préparateur de commandes",
                "Thionville",
                Statut::Entretien,
                j(11, 5),
            ),
            c(4, "Réseaux Moselle Télécom", "Technicien réseaux", "", Statut::Negative, j(10, 20)),
        ];
        l[2].entretien =
            Some(Entretien { jour: j(11, 20), heure: Heure::nouvelle(9, 30), lieu: "Zone Actipôle".into() });
        l
    }

    #[test]
    fn trois_onglets_vides() {
        let e = etat("0.1.0", &Vue::vide(auj()));
        assert_eq!(e["app"], json!({ "nom": "Jobmind", "version": "0.1.0", "logo": "logo.svg" }));
        assert_eq!(e["accueil"]["tuiles"], json!([]));
        let o = e["accueil"]["perso"]["onglets"].as_array().unwrap();
        let ids: Vec<&str> = o.iter().map(|x| x["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ONGLETS);
        let noms: Vec<&str> = o.iter().map(|x| x["nom"].as_str().unwrap()).collect();
        assert_eq!(noms, ["Candidatures", "Offres", "Mon profil"]);
        assert_eq!(o[0]["dossier"]["titre"], AUCUNE_CANDIDATURE);
        assert_eq!(o[1]["dossier"]["titre"], AUCUNE_OFFRE);
        assert_eq!(o[0]["nombre"], 0);
        assert!(o[0]["suivi"]["groupes"].as_array().unwrap().is_empty());
        assert!(o[0]["suivi"]["fiche"].is_null());
        assert!(o[0]["suivi"]["bilan"].as_array().unwrap().is_empty());
        // Mon profil : ses cinq rubriques, vides ; Réglages a son délai de relance
        let r = o[2]["rubriques"].as_array().unwrap();
        let noms: Vec<&str> = r.iter().map(|x| x["nom"].as_str().unwrap()).collect();
        assert_eq!(noms, ["Mon CV", "Ce que je cherche", "Mon style", "Mes clés", "Réglages"]);
        let vues: Vec<&str> = r[..4].iter().map(|x| x["dossier"]["vue"].as_str().unwrap()).collect();
        assert_eq!(vues, ["cv", "recherche", "style", "cles"]);
        let sous: Vec<&str> = r.iter().map(|x| x["sous"].as_str().unwrap()).collect();
        assert_eq!(
            sous,
            [
                "Pas encore donné",
                "Rien de noté pour l'instant",
                "Aucun texte pour l'instant",
                "Aucune clé pour l'instant",
                "Relance après 14 jours"
            ]
        );
        assert!(r[0]["dossier"]["ouvert"].is_null() && r[0]["dossier"]["import"].is_null());
        assert_eq!(r[1]["dossier"]["enregistree"], false);
        assert_eq!(r[1]["dossier"]["departements"].as_array().unwrap().len(), 101);
        assert_eq!(r[3]["dossier"]["services"][2]["etat"], "absente");
        assert_eq!(r[4]["dossier"]["reglage"]["valeur"], 14);
        assert_eq!(
            (r[4]["dossier"]["reglage"]["min"].as_u64(), r[4]["dossier"]["reglage"]["max"].as_u64()),
            (Some(3), Some(60))
        );
        assert!(e["pages"].as_object().unwrap().is_empty());
    }

    #[test]
    fn la_liste_groupee_et_le_dossier_ouvert() {
        let l = exemples();
        let frise =
            [Evenement { jour: Jour::nouveau(2026, 11, 2).unwrap(), statut: Statut::Envoyee, texte: String::new() }];
        let v = Vue {
            candidatures: &l,
            ouverte: Some(1),
            frise: &frise,
            delai: 14,
            aujourdhui: auj(),
            profil: &VIDE,
            offres: &OVIDE,
        };
        let e = etat("0.1.0", &v);
        let o = &e["accueil"]["perso"]["onglets"][0];
        // Deux choses à faire : l'entretien du 20, puis la relance de Transports Moselle Fret
        assert_eq!(o["nombre"], 2);
        let g = o["suivi"]["groupes"].as_array().unwrap();
        let resume: Vec<(String, Vec<i64>)> = g
            .iter()
            .map(|x| {
                (
                    x["nom"].as_str().unwrap().to_string(),
                    x["lignes"].as_array().unwrap().iter().map(|l| l["id"].as_i64().unwrap()).collect(),
                )
            })
            .collect();
        assert_eq!(
            resume,
            [
                ("À faire maintenant".to_string(), vec![3, 1]),
                ("En attente".into(), vec![2]),
                ("Terminées".into(), vec![4])
            ]
        );
        let l1 = &g[0]["lignes"][1];
        assert_eq!(l1["nom"], "Transports Moselle Fret");
        assert_eq!(l1["sous"], "Agent de quai, Woippy");
        assert_eq!((l1["statut"].as_str(), l1["statut_nom"].as_str()), (Some("envoyee"), Some("Envoyée")));
        assert_eq!(l1["action"], "Relance à faire, 16 jours sans réponse");
        assert_eq!(g[0]["lignes"][0]["action"], "Entretien le 20 nov. à 9 h 30");
        assert_eq!(g[2]["lignes"][0]["sous"], "Technicien réseaux");
        assert_eq!(g[2]["lignes"][0]["fini"], true);
        assert_eq!(o["suivi"]["bilan"], json!(["3 envois en novembre", "2 réponses sur 4 candidatures, soit 50 %"]));
        // Le dossier ouvert
        let f = &o["suivi"]["fiche"];
        assert_eq!(o["suivi"]["ouverte"], 1);
        assert_eq!(f["titre"], "Transports Moselle Fret");
        assert_eq!(f["ou"], "Envoyée il y a 16 jours, pas encore de réponse.");
        assert_eq!(f["ou_sous"], "Envoyée le lundi 2 novembre, sur le site, offre vue sur Indeed.");
        assert_eq!(f["action"]["titre"], "Relancer Transports Moselle Fret");
        let boutons: Vec<&str> =
            f["action"]["boutons"].as_array().unwrap().iter().map(|b| b["id"].as_str().unwrap()).collect();
        assert_eq!(boutons, ["copier", "relancee"]);
        assert!(f["relance"].as_str().unwrap().starts_with("Madame, Monsieur,"));
        assert!(f["calme"].is_null());
        assert_eq!(f["frise"], json!([{ "texte": "Envoyée", "date": "2 nov." }]));
        assert_eq!(f["envoye"]["meta"], "Sur le site, le 2 nov., avec « CV 2026.pdf »");
        assert_eq!(f["champs"]["date"], "02/11/2026");
        assert_eq!(f["champs"]["moyen"], "site");
        // Une candidature en attente : une phrase calme, pas de message de relance
        let v = Vue { ouverte: Some(2), ..v };
        let f = &etat("0.1.0", &v)["accueil"]["perso"]["onglets"][0]["suivi"]["fiche"];
        assert!(f["action"].is_null() && f["relance"].is_null());
        assert_eq!(
            f["calme"]["texte"],
            "Rien à faire pour l'instant. Si rien n'arrive, Jobmind te préviendra le lundi 30 novembre."
        );
        // L'entretien : la date à reprendre dans le petit formulaire
        let v = Vue { ouverte: Some(3), ..v };
        let f = &etat("0.1.0", &v)["accueil"]["perso"]["onglets"][0]["suivi"]["fiche"];
        assert_eq!(f["entretien"], json!({ "date": "20/11/2026", "heure": "9 h 30", "lieu": "Zone Actipôle" }));
        assert_eq!(f["action"]["icone"], "agenda");
        // Une candidature qui n'existe plus : aucun dossier
        let v = Vue { ouverte: Some(99), ..v };
        assert!(etat("0.1.0", &v)["accueil"]["perso"]["onglets"][0]["suivi"]["fiche"].is_null());
    }

    /// Le calendrier des dates : les mois en français, la semaine du lundi au dimanche, aujourd'hui, et les jours à
    /// choisir de la date d'envoi (jusqu'à aujourd'hui) et de l'entretien (comme la vérification).
    #[test]
    fn le_calendrier_des_dates() {
        let e = etat("0.1.0", &Vue::vide(auj()));
        let k = &e["accueil"]["perso"]["onglets"][0]["suivi"]["textes"]["calendrier"];
        let mois: Vec<&str> = k["mois"].as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect();
        assert_eq!(mois.len(), 12);
        assert_eq!((mois[0], mois[1], mois[7], mois[11]), ("janvier", "février", "août", "décembre"));
        assert_eq!(k["jours"][0], "lun.");
        assert_eq!(k["jours"][6], "dim.");
        assert_eq!(k["aujourdhui"], "2026-11-18");
        assert_eq!(k["envoi"], json!({ "min": "2023-11-16", "max": "2026-11-18" }));
        assert_eq!(k["entretien"], json!({ "min": "2025-11-17", "max": "2029-11-20" }));
        // (le champ garde son indice JJ/MM/AAAA : taper reste possible)
        assert_eq!(e["accueil"]["perso"]["onglets"][0]["suivi"]["textes"]["i_date"], "JJ/MM/AAAA");
        let js =
            std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join("jobmind.js"))
                .unwrap();
        assert!(js.contains("function calendrierHTML("), "jobmind.js ne dessine pas le calendrier");
    }

    /// Règle de la suite : aucun tiret long ni demi-cadratin, aucun HTML (la page écrit les textes tels quels), des
    /// ids que la page accepte.
    #[test]
    fn textes_simples() {
        let l = exemples();
        let p = crate::vue_profil::tests::exemple();
        for (ouverte, profil) in [(None, &VIDE), (Some(1), &p), (Some(2), &VIDE), (Some(3), &p), (Some(4), &VIDE)] {
            let v = Vue { candidatures: &l, ouverte, frise: &[], delai: 14, aujourdhui: auj(), profil, offres: &OVIDE };
            let mut t = Vec::new();
            textes_de(&etat("0.1.0", &v), &mut t);
            for t in t {
                assert!(!t.contains(['\u{2013}', '\u{2014}', '<', '>']), "{t}");
                assert!(!t.contains("  "), "{t}");
            }
        }
        let e = etat("0.1.0", &Vue::vide(auj()));
        for o in e["accueil"]["perso"]["onglets"].as_array().unwrap() {
            for x in std::iter::once(o).chain(o["rubriques"].as_array().unwrap()) {
                let id = x["id"].as_str().unwrap();
                assert!(!id.is_empty() && id.bytes().all(|c| c.is_ascii_lowercase()), "{id}");
            }
        }
        for s in e["accueil"]["perso"]["onglets"][0]["suivi"]["statuts"].as_array().unwrap() {
            let id = s["id"].as_str().unwrap();
            assert!(id.bytes().all(|c| c.is_ascii_lowercase() || c == b'_'), "{id}");
        }
    }

    /// Le kit remet en liste certains champs dès l'arrivée de l'état (`LISTES` de kit.js : « quand », « choix »,
    /// « boutons »...) : un de ces noms qui porterait un texte arriverait vide à la page (vu à l'étape 2 : la date de la
    /// frise s'appelait « quand »). Chacun de ces noms, partout dans l'état, porte donc une liste (ou rien).
    #[test]
    fn les_listes_du_kit_sont_des_listes() {
        let kit = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join("kit.js"))
            .unwrap();
        let debut = kit.find("const LISTES = new Set([").unwrap();
        let fin = debut + kit[debut..].find("]);").unwrap();
        let noms: Vec<&str> = kit[debut..fin].split('\'').skip(1).step_by(2).collect();
        assert!(noms.contains(&"quand") && noms.contains(&"boutons") && noms.len() > 10, "{noms:?}");
        fn verifier(v: &Value, noms: &[&str], chemin: &str) {
            match v {
                Value::Object(o) => {
                    for (k, x) in o {
                        if noms.contains(&k.as_str()) {
                            assert!(x.is_array() || x.is_null(), "{chemin}.{k} n'est pas une liste : {x}");
                        }
                        verifier(x, noms, &format!("{chemin}.{k}"));
                    }
                }
                Value::Array(a) => a.iter().for_each(|x| verifier(x, noms, chemin)),
                _ => {}
            }
        }
        let l = exemples();
        let frise =
            [Evenement { jour: Jour::nouveau(2026, 11, 2).unwrap(), statut: Statut::Envoyee, texte: String::new() }];
        let p = crate::vue_profil::tests::exemple();
        for (ouverte, profil) in [(None, &VIDE), (Some(1), &p), (Some(2), &VIDE), (Some(3), &p), (Some(4), &VIDE)] {
            let v =
                Vue { candidatures: &l, ouverte, frise: &frise, delai: 14, aujourdhui: auj(), profil, offres: &OVIDE };
            verifier(&etat("0.1.0", &v), &noms, "etat");
        }
    }

    /// Le logo de l'état est bien livré avec les pages (ui\), et la page charge l'accueil de Jobmind ; chaque icône
    /// envoyée est connue de la page.
    #[test]
    fn le_logo_et_l_accueil_existent() {
        let ui = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
        let l = exemples();
        let p = crate::vue_profil::tests::exemple();
        let e = etat("0.1.0", &Vue { profil: &p, ..Vue::vide(auj()) });
        assert!(ui.join(e["app"]["logo"].as_str().unwrap()).is_file());
        let index = std::fs::read_to_string(ui.join("index.html")).unwrap();
        for f in ["jobmind.css", "jobmind.js"] {
            assert!(ui.join(f).is_file(), "{f}");
            assert!(index.contains(f), "index.html ne charge pas {f}");
        }
        let js = std::fs::read_to_string(ui.join("jobmind.js")).unwrap();
        let mut icones = Vec::new();
        for o in e["accueil"]["perso"]["onglets"].as_array().unwrap() {
            icones.push(o["dossier"]["icone"].as_str().unwrap().to_string());
            for r in o["rubriques"].as_array().unwrap() {
                icones.push(r["icone"].as_str().unwrap().to_string());
            }
        }
        for id in 1..=4 {
            let v = Vue {
                candidatures: &l,
                ouverte: Some(id),
                frise: &[],
                delai: 14,
                aujourdhui: auj(),
                profil: &VIDE,
                offres: &OVIDE,
            };
            let f = &etat("0.1.0", &v)["accueil"]["perso"]["onglets"][0]["suivi"]["fiche"];
            for x in [&f["action"]["icone"], &f["calme"]["icone"]] {
                if let Some(i) = x.as_str() {
                    icones.push(i.to_string());
                }
            }
            for b in f["action"]["boutons"].as_array().into_iter().flatten() {
                if let Some(i) = b["icone"].as_str() {
                    icones.push(i.to_string());
                }
            }
        }
        for i in [
            "plus",
            "crayon",
            "poubelle",
            "copie",
            "lien",
            "importer",
            "coller",
            "etoile",
            "attention",
            "cadenas",
            "cle",
        ] {
            icones.push(i.to_string());
        }
        for i in icones {
            assert!(js.contains(&format!("  {i}: ")), "icône {i} inconnue de jobmind.js");
        }
        // L'état entier reste léger (envoyé à chaque changement ; la liste des départements compte pour 6 Ko)
        assert!(etat("0.1.0", &Vue::vide(auj())).to_string().len() < 40_000);
        assert!(e.to_string().len() < 60_000);
        let v = Vue {
            candidatures: &l,
            ouverte: Some(1),
            frise: &[],
            delai: 14,
            aujourdhui: auj(),
            profil: &VIDE,
            offres: &OVIDE,
        };
        assert!(etat("0.1.0", &v).to_string().len() < 40_000);
    }
}
