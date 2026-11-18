//! Le suivi des candidatures (étape 2 du plan) : où en est chacune, ce qu'il faut faire maintenant, le petit bilan du
//! mois, le résumé de la carte de Mindset, les bulles près de l'horloge et le message de relance. Tout est calculé ici,
//! d'après les candidatures, le jour qu'il est (donné par l'appelant) et le délai de relance choisi dans les réglages.
//!
//! - Envoyée depuis au moins X jours sans réponse : « À faire maintenant », relance à faire ; avant : « En attente ».
//! - Relancée depuis au moins X jours : « À faire maintenant », proposer de la classer en « Sans réponse ».
//! - Entretien aujourd'hui ou plus tard : « À faire maintenant » ; passé : « En attente » de la réponse.
//! - Réponse positive ou négative, Sans réponse, Abandonnée : « Terminées ».
//!
//! Les textes sont en français simple, sans tiret long ; le message de relance est professionnel (vouvoiement,
//! formule d'appel et de politesse, rien d'inventé).

use crate::candidature::{Candidature, Entretien, Moyen};
use crate::jour::Jour;
use crate::statut::Statut;

/// Le délai de relance par défaut, et ses bornes (réglage « Relancer après »).
pub const DELAI_DEFAUT: u32 = 14;
pub const DELAI_MIN: u32 = 3;
pub const DELAI_MAX: u32 = 60;

/// Un délai lu (dans la base, venu de la page) ramené entre les bornes.
pub fn delai_borne(n: i64) -> u32 {
    n.clamp(i64::from(DELAI_MIN), i64::from(DELAI_MAX)) as u32
}

/// Les trois groupes de la liste, dans l'ordre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Groupe {
    AFaire,
    EnAttente,
    Terminee,
}

impl Groupe {
    pub const TOUS: [Groupe; 3] = [Groupe::AFaire, Groupe::EnAttente, Groupe::Terminee];

    pub fn code(self) -> &'static str {
        match self {
            Groupe::AFaire => "afaire",
            Groupe::EnAttente => "attente",
            Groupe::Terminee => "terminees",
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            Groupe::AFaire => "À faire maintenant",
            Groupe::EnAttente => "En attente",
            Groupe::Terminee => "Terminées",
        }
    }
}

/// Ce qu'il faut faire maintenant pour une candidature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Envoyée, sans réponse depuis `jours` jours
    Relancer { jours: i64 },
    /// Relancée, toujours sans réponse depuis `jours` jours
    ClasserSansReponse { jours: i64 },
    /// Un entretien aujourd'hui ou plus tard
    Entretien { entretien: Entretien, dans: i64 },
}

/// Où en est une candidature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Situation {
    pub groupe: Groupe,
    pub action: Option<Action>,
    /// Le jour où une relance (ou le classement) sera proposée si rien n'arrive
    pub prochaine: Option<Jour>,
}

/// Le jour de la relance (ou, à défaut, du dernier changement de statut).
fn relancee_le(c: &Candidature) -> Jour {
    c.relance_le.unwrap_or(c.statut_le)
}

/// Où en est une candidature aujourd'hui, avec ce délai de relance (en jours).
pub fn situation(c: &Candidature, aujourdhui: Jour, delai: u32) -> Situation {
    let delai = i64::from(delai);
    let attente = |prochaine| Situation { groupe: Groupe::EnAttente, action: None, prochaine };
    match c.statut {
        Statut::Envoyee => {
            let jours = c.envoyee_le.jours_jusqua(aujourdhui);
            if jours >= delai {
                Situation { groupe: Groupe::AFaire, action: Some(Action::Relancer { jours }), prochaine: None }
            } else {
                attente(Some(c.envoyee_le.plus(delai)))
            }
        }
        Statut::Relancee => {
            let depuis = relancee_le(c);
            let jours = depuis.jours_jusqua(aujourdhui);
            if jours >= delai {
                Situation {
                    groupe: Groupe::AFaire,
                    action: Some(Action::ClasserSansReponse { jours }),
                    prochaine: None,
                }
            } else {
                attente(Some(depuis.plus(delai)))
            }
        }
        Statut::Entretien => match &c.entretien {
            Some(e) if e.jour >= aujourdhui => Situation {
                groupe: Groupe::AFaire,
                action: Some(Action::Entretien { entretien: e.clone(), dans: aujourdhui.jours_jusqua(e.jour) }),
                prochaine: None,
            },
            _ => attente(None),
        },
        _ => Situation { groupe: Groupe::Terminee, action: None, prochaine: None },
    }
}

fn pluriel(n: i64, un: &str, plusieurs: &str) -> String {
    if n.abs() <= 1 { format!("{n} {un}") } else { format!("{n} {plusieurs}") }
}

/// La petite ligne d'action sous une candidature dans la liste (rien si rien n'est à faire).
pub fn ligne_action(s: &Situation, aujourdhui: Jour) -> Option<String> {
    Some(match s.action.as_ref()? {
        Action::Relancer { jours } => format!("Relance à faire, {} sans réponse", pluriel(*jours, "jour", "jours")),
        Action::ClasserSansReponse { jours } => {
            format!("Rien depuis la relance, {}", pluriel(*jours, "jour", "jours"))
        }
        Action::Entretien { entretien, dans } => {
            let jour = match dans {
                0 => "aujourd'hui".to_string(),
                1 => "demain".to_string(),
                _ => format!("le {}", entretien.jour.court(aujourdhui)),
            };
            match entretien.heure {
                Some(h) => format!("Entretien {jour} à {}", h.texte()),
                None => format!("Entretien {jour}"),
            }
        }
    })
}

/// Une phrase finie par un point (sans le doubler après « nov. »).
fn phrase(mut t: String) -> String {
    if !t.ends_with('.') {
        t.push('.');
    }
    t
}

/// « il y a 16 jours », « hier », « aujourd'hui ».
fn il_y_a(jours: i64) -> String {
    match jours {
        i64::MIN..=0 => "aujourd'hui".into(),
        1 => "hier".into(),
        n => format!("il y a {n} jours"),
    }
}

/// La grande phrase « où on en est » du dossier, et la ligne pâle dessous (envoyée quand, par où, vue où).
pub fn ou_on_en_est(c: &Candidature, aujourdhui: Jour) -> (String, String) {
    let depuis = |j: Jour| j.jours_jusqua(aujourdhui);
    let phrase = match c.statut {
        Statut::Envoyee => match depuis(c.envoyee_le) {
            n if n <= 0 => "Envoyée aujourd'hui.".to_string(),
            n => format!("Envoyée {}, pas encore de réponse.", il_y_a(n)),
        },
        Statut::Relancee => format!("Relancée {}, en attente de réponse.", il_y_a(depuis(relancee_le(c)))),
        Statut::Entretien => match &c.entretien {
            // (le jour et l'heure exacts sont dans le cadre « prochaine action », juste dessous)
            Some(e) if e.jour >= aujourdhui => match aujourdhui.jours_jusqua(e.jour) {
                0 => "L'entretien est aujourd'hui.".to_string(),
                1 => "L'entretien est demain.".to_string(),
                n => format!("Un entretien est prévu dans {n} jours."),
            },
            Some(e) => format!("Entretien passé le {}, en attente de la réponse.", e.jour.court(aujourdhui)),
            None => "Un entretien est prévu.".to_string(),
        },
        Statut::Positive => phrase(format!("Réponse positive le {}", c.statut_le.court(aujourdhui))),
        Statut::Negative => phrase(format!("Réponse négative le {}", c.statut_le.court(aujourdhui))),
        Statut::SansReponse => phrase(format!("Pas de réponse, classée le {}", c.statut_le.court(aujourdhui))),
        Statut::Abandonnee => phrase(format!("Abandonnée le {}", c.statut_le.court(aujourdhui))),
    };
    let mut sous = format!("Envoyée le {}", c.envoyee_le.long(aujourdhui));
    if let Some(m) = c.moyen.dans_phrase() {
        sous.push_str(", ");
        sous.push_str(m);
    }
    if !c.source.is_empty() {
        sous.push_str(", offre vue sur ");
        sous.push_str(&c.source);
    }
    sous.push('.');
    (phrase, sous)
}

/// Le cadre « prochaine action » du dossier : son titre et sa phrase.
pub fn cadre_action(c: &Candidature, a: &Action, aujourdhui: Jour) -> (String, String) {
    match a {
        Action::Relancer { jours } => (
            format!("Relancer {}", c.entreprise),
            format!("Pas de réponse depuis {}. Un message court suffit.", pluriel(*jours, "jour", "jours")),
        ),
        Action::ClasserSansReponse { jours } => (
            "Toujours rien après la relance".to_string(),
            format!(
                "{} depuis la relance. Tu peux la classer en « Sans réponse ».",
                majuscule(&pluriel(*jours, "jour", "jours"))
            ),
        ),
        Action::Entretien { entretien, .. } => {
            let titre = majuscule(&format!("Entretien {}", entretien.quand(aujourdhui)));
            let texte = if entretien.lieu.is_empty() {
                "Pense à prendre ton CV.".to_string()
            } else {
                format!("{}. Pense à prendre ton CV.", entretien.lieu)
            };
            (titre, texte)
        }
    }
}

/// La phrase calme quand rien n'est à faire.
pub fn phrase_calme(c: &Candidature, s: &Situation, aujourdhui: Jour) -> String {
    match (s.groupe, s.prochaine) {
        (Groupe::Terminee, _) if c.statut == Statut::Positive => {
            "Bonne nouvelle. Note la suite dans les notes : un rendez-vous, un appel à passer.".to_string()
        }
        (Groupe::Terminee, _) => "Dossier fermé. Il reste ici pour garder une trace.".to_string(),
        (_, Some(j)) if c.statut == Statut::Relancee => format!(
            "Rien à faire pour l'instant. Si rien n'arrive, Jobmind te proposera de la classer le {}.",
            j.long(aujourdhui)
        ),
        (_, Some(j)) => {
            format!("Rien à faire pour l'instant. Si rien n'arrive, Jobmind te préviendra le {}.", j.long(aujourdhui))
        }
        _ => "Rien à faire pour l'instant. Note la réponse dès que tu l'as.".to_string(),
    }
}

fn majuscule(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(p) => p.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Le poste dans une phrase : « agent de quai » (la première lettre en minuscule, sauf un sigle).
fn poste_en_phrase(poste: &str) -> String {
    let mut c = poste.chars();
    match (c.next(), c.next()) {
        (Some(p), Some(d)) if p.is_uppercase() && d.is_lowercase() => {
            p.to_lowercase().chain(poste.chars().skip(1)).collect()
        }
        _ => poste.to_string(),
    }
}

/// « de » ou « d' » devant un mot.
fn de(mot: &str) -> String {
    let voyelle = mot.chars().next().is_some_and(|c| "aeiouyhàâäéèêëîïôöûùüAEIOUYHÀÂÉÈÊÎÔÛ".contains(c));
    if voyelle { format!("d'{mot}") } else { format!("de {mot}") }
}

/// Le message de relance à copier : court et professionnel (vouvoiement, formule d'appel et de politesse, rien
/// d'inventé : seulement le poste, la ville et la date d'envoi). Par mail, avec une ligne d'objet.
pub fn message_relance(c: &Candidature, aujourdhui: Jour) -> String {
    let poste = poste_en_phrase(&c.poste);
    let ville = if c.ville.is_empty() { String::new() } else { format!(" à {}", c.ville) };
    let e = c.envoyee_le;
    let jour = if e.jour == 1 { "1er".to_string() } else { e.jour.to_string() };
    let quand = if e.annee == aujourdhui.annee {
        format!("{jour} {}", e.nom_du_mois())
    } else {
        format!("{jour} {} {}", e.nom_du_mois(), e.annee)
    };
    let mut m = String::new();
    if c.moyen == Moyen::Mail {
        m.push_str(&format!("Objet : Ma candidature au poste {}\n\n", de(&poste)));
    }
    m.push_str("Madame, Monsieur,\n\n");
    m.push_str(&format!(
        "Le {quand}, je vous ai adressé ma candidature au poste {}{ville}. Je souhaiterais savoir si elle a pu être \
         étudiée.\n\n",
        de(&poste)
    ));
    m.push_str(
        "Ce poste m'intéresse toujours et je reste disponible pour un entretien, à la date qui vous conviendra.\n\n",
    );
    m.push_str("Je vous prie d'agréer, Madame, Monsieur, mes salutations distinguées.");
    m
}

/// Le petit bilan du mois, en bas de la liste : les envois du mois, puis les réponses sur l'ensemble.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bilan {
    pub envois_du_mois: usize,
    /// Les candidatures comptées (toutes sauf les abandonnées)
    pub comptees: usize,
    /// Celles qui ont eu une réponse (entretien, réponse positive ou négative)
    pub reponses: usize,
    pub mois: &'static str,
}

impl Bilan {
    /// Le taux de réponse en pour cent (rien sans candidature comptée).
    pub fn taux(&self) -> Option<u32> {
        (self.comptees > 0).then(|| ((self.reponses * 100 + self.comptees / 2) / self.comptees) as u32)
    }

    /// Les deux lignes du bilan.
    pub fn lignes(&self) -> Vec<String> {
        let envois = match self.envois_du_mois {
            0 => format!("Aucun envoi en {}", self.mois),
            1 => format!("1 envoi en {}", self.mois),
            n => format!("{n} envois en {}", self.mois),
        };
        let mut l = vec![envois];
        if let Some(t) = self.taux() {
            l.push(match self.reponses {
                0 => format!("Aucune réponse sur {}", pluriel(self.comptees as i64, "candidature", "candidatures")),
                r => format!(
                    "{} sur {}, soit {t} %",
                    pluriel(r as i64, "réponse", "réponses"),
                    pluriel(self.comptees as i64, "candidature", "candidatures")
                ),
            });
        }
        l
    }
}

/// A eu une réponse.
fn repondue(s: Statut) -> bool {
    matches!(s, Statut::Entretien | Statut::Positive | Statut::Negative)
}

pub fn bilan(liste: &[Candidature], aujourdhui: Jour) -> Bilan {
    let comptees: Vec<&Candidature> = liste.iter().filter(|c| c.statut != Statut::Abandonnee).collect();
    Bilan {
        envois_du_mois: liste
            .iter()
            .filter(|c| c.envoyee_le.annee == aujourdhui.annee && c.envoyee_le.mois == aujourdhui.mois)
            .count(),
        comptees: comptees.len(),
        reponses: comptees.iter().filter(|c| repondue(c.statut)).count(),
        mois: aujourdhui.nom_du_mois(),
    }
}

/// La liste rangée : chaque groupe avec ses candidatures, dans l'ordre de l'écran (« À faire » : les entretiens
/// d'abord, puis les relances les plus anciennes ; « En attente » et « Terminées » : les plus récentes d'abord).
pub fn ranger(liste: &[Candidature], aujourdhui: Jour, delai: u32) -> Vec<(Groupe, Vec<(&Candidature, Situation)>)> {
    let mut groupes: Vec<(Groupe, Vec<(&Candidature, Situation)>)> =
        Groupe::TOUS.iter().map(|g| (*g, Vec::new())).collect();
    for c in liste {
        let s = situation(c, aujourdhui, delai);
        if let Some(g) = groupes.iter_mut().find(|(g, _)| *g == s.groupe) {
            g.1.push((c, s));
        }
    }
    for (g, l) in &mut groupes {
        match g {
            Groupe::AFaire => l.sort_by_key(|(c, s)| match &s.action {
                Some(Action::Entretien { dans, .. }) => (0, *dans, c.id),
                Some(Action::Relancer { jours }) => (1, -jours, c.id),
                Some(Action::ClasserSansReponse { jours }) => (2, -jours, c.id),
                None => (3, 0, c.id),
            }),
            Groupe::EnAttente => l.sort_by_key(|(c, _)| (std::cmp::Reverse(c.envoyee_le), std::cmp::Reverse(c.id))),
            Groupe::Terminee => l.sort_by_key(|(c, _)| (std::cmp::Reverse(c.statut_le), std::cmp::Reverse(c.id))),
        }
    }
    groupes.retain(|(_, l)| !l.is_empty());
    groupes
}

/// Ce que la carte de Mindset montre (2 jauges, 3 lignes au plus).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeCarte {
    /// Les candidatures qui attendent encore une réponse (envoyées, relancées, entretien passé)
    pub en_attente: usize,
    pub comptees: usize,
    pub taux: Option<u32>,
    pub relances: usize,
    /// Le prochain entretien : « Entretien demain à 9 h 30 : Logistique des Trois Frontières »
    pub prochain_entretien: Option<(String, bool)>,
    pub envois: String,
}

pub fn resume_carte(liste: &[Candidature], aujourdhui: Jour, delai: u32) -> ResumeCarte {
    let b = bilan(liste, aujourdhui);
    let situations: Vec<(&Candidature, Situation)> =
        liste.iter().map(|c| (c, situation(c, aujourdhui, delai))).collect();
    let en_attente =
        liste.iter().filter(|c| matches!(c.statut, Statut::Envoyee | Statut::Relancee | Statut::Entretien)).count();
    let relances = situations
        .iter()
        .filter(|(_, s)| matches!(s.action, Some(Action::Relancer { .. }) | Some(Action::ClasserSansReponse { .. })))
        .count();
    let prochain = situations
        .iter()
        .filter_map(|(c, s)| match &s.action {
            Some(Action::Entretien { entretien, dans }) => Some((c, entretien, *dans)),
            _ => None,
        })
        .min_by_key(|(c, e, _)| (e.jour, e.heure, c.id))
        .map(|(c, e, dans)| {
            (format!("{} : {}", majuscule(&format!("entretien {}", e.quand(aujourdhui))), c.entreprise), dans <= 1)
        });
    ResumeCarte {
        en_attente,
        comptees: b.comptees,
        taux: b.taux(),
        relances,
        prochain_entretien: prochain,
        envois: b.lignes().into_iter().next().unwrap_or_default(),
    }
}

/// Une bulle due près de l'horloge : sa clé (une seule bulle par clé, pour toujours), la candidature, le genre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulleDue {
    pub cle: String,
    pub candidature: i64,
    pub entreprise: String,
    pub genre: GenreBulle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenreBulle {
    /// Une relance vient de devenir à faire
    Relance { poste: String, jours: i64 },
    /// Un entretien demain (ou aujourd'hui, s'il n'a pas pu être annoncé la veille)
    Entretien { entretien: Entretien, dans: i64 },
}

/// Les bulles dues aujourd'hui : une relance à faire (clé : la candidature et son jour d'envoi : changer la date en
/// refait une), un entretien demain ou aujourd'hui (clé : la candidature et le moment de l'entretien).
pub fn bulles_dues(liste: &[Candidature], aujourdhui: Jour, delai: u32) -> Vec<BulleDue> {
    let mut dues = Vec::new();
    for c in liste {
        match situation(c, aujourdhui, delai).action {
            Some(Action::Relancer { jours }) => dues.push(BulleDue {
                cle: format!("relance:{}:{}", c.id, c.envoyee_le.iso()),
                candidature: c.id,
                entreprise: c.entreprise.clone(),
                genre: GenreBulle::Relance { poste: c.poste.clone(), jours },
            }),
            Some(Action::Entretien { entretien, dans }) if dans <= 1 => dues.push(BulleDue {
                cle: format!("entretien:{}:{}", c.id, entretien.moment_iso()),
                candidature: c.id,
                entreprise: c.entreprise.clone(),
                genre: GenreBulle::Entretien { entretien, dans },
            }),
            _ => {}
        }
    }
    dues
}

/// Coupe un texte à `max` caractères (avec « ... » s'il est coupé).
pub fn couper(t: &str, max: usize) -> String {
    if t.chars().count() <= max {
        return t.to_string();
    }
    let mut c: String = t.chars().take(max.saturating_sub(3)).collect();
    c = c.trim_end().to_string();
    c.push_str("...");
    c
}

/// Le titre et le texte d'une bulle pour ces événements (plusieurs relances dues ensemble : une seule bulle qui les
/// regroupe). Titre de 63 caractères au plus, texte de 250 (ce que Mindset garde).
pub fn texte_bulle(dues: &[BulleDue]) -> Option<(String, String)> {
    let (titre, texte) = match dues {
        [] => return None,
        [d] => match &d.genre {
            GenreBulle::Relance { poste, jours } => (
                "Une relance à faire".to_string(),
                format!(
                    "{} ({}) : pas de réponse depuis {}. Ouvre le dossier pour copier un message.",
                    d.entreprise,
                    poste_en_phrase(poste),
                    pluriel(*jours, "jour", "jours")
                ),
            ),
            GenreBulle::Entretien { entretien, dans } => {
                let titre = if *dans <= 0 { "Entretien aujourd'hui" } else { "Entretien demain" };
                let mut t = match entretien.heure {
                    Some(h) => format!("{}, à {}", d.entreprise, h.texte()),
                    None => d.entreprise.clone(),
                };
                if !entretien.lieu.is_empty() {
                    t.push_str(", ");
                    t.push_str(&entretien.lieu);
                }
                t.push('.');
                (titre.to_string(), t)
            }
        },
        plusieurs => {
            let noms: Vec<&str> = plusieurs.iter().map(|d| d.entreprise.as_str()).collect();
            let liste = match noms.as_slice() {
                [a, b] => format!("{a} et {b}"),
                [a, b, reste @ ..] => format!("{a}, {b} et {}", pluriel(reste.len() as i64, "autre", "autres")),
                _ => noms.join(", "),
            };
            (format!("{} relances à faire", plusieurs.len()), format!("{liste} attendent une relance."))
        }
    };
    Some((couper(&titre, 63), couper(&texte, 250)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jour::Heure;

    fn j(a: i32, m: u32, d: u32) -> Jour {
        Jour::nouveau(a, m, d).unwrap()
    }
    fn auj() -> Jour {
        j(2026, 11, 18)
    }

    pub(crate) fn cand(id: i64, entreprise: &str, statut: Statut, envoyee: Jour) -> Candidature {
        Candidature {
            id,
            entreprise: entreprise.into(),
            poste: "Agent de quai".into(),
            ville: "Woippy".into(),
            statut,
            statut_le: envoyee,
            envoyee_le: envoyee,
            moyen: Moyen::Site,
            source: String::new(),
            lien: String::new(),
            envoye: String::new(),
            cv: String::new(),
            notes: String::new(),
            relance_le: None,
            entretien: None,
        }
    }

    #[test]
    fn envoyee_puis_relance_a_faire() {
        let c = cand(1, "Transports Moselle Fret", Statut::Envoyee, j(2026, 11, 5));
        // 13 jours : en attente, la relance prévue le 19
        let s = situation(&c, auj(), 14);
        assert_eq!(s, Situation { groupe: Groupe::EnAttente, action: None, prochaine: Some(j(2026, 11, 19)) });
        assert_eq!(ligne_action(&s, auj()), None);
        assert_eq!(
            phrase_calme(&c, &s, auj()),
            "Rien à faire pour l'instant. Si rien n'arrive, Jobmind te préviendra le jeudi 19 novembre."
        );
        // 14 jours pile : à faire
        let s = situation(&c, j(2026, 11, 19), 14);
        assert_eq!(s.groupe, Groupe::AFaire);
        assert_eq!(s.action, Some(Action::Relancer { jours: 14 }));
        assert_eq!(ligne_action(&s, auj()).unwrap(), "Relance à faire, 14 jours sans réponse");
        // Un délai plus court la rend à faire plus tôt
        assert_eq!(situation(&c, auj(), 7).groupe, Groupe::AFaire);
        let (titre, texte) = cadre_action(&c, &Action::Relancer { jours: 16 }, auj());
        assert_eq!(titre, "Relancer Transports Moselle Fret");
        assert_eq!(texte, "Pas de réponse depuis 16 jours. Un message court suffit.");
    }

    #[test]
    fn relancee_puis_classer() {
        let mut c = cand(2, "Brico Lorraine", Statut::Relancee, j(2026, 10, 20));
        c.relance_le = Some(j(2026, 11, 10));
        c.statut_le = j(2026, 11, 10);
        let s = situation(&c, auj(), 14);
        assert_eq!(s.groupe, Groupe::EnAttente);
        assert_eq!(s.prochaine, Some(j(2026, 11, 24)));
        assert!(phrase_calme(&c, &s, auj()).contains("te proposera de la classer le mardi 24 novembre"));
        assert_eq!(ou_on_en_est(&c, auj()).0, "Relancée il y a 8 jours, en attente de réponse.");
        let s = situation(&c, j(2026, 11, 24), 14);
        assert_eq!(s.action, Some(Action::ClasserSansReponse { jours: 14 }));
        assert_eq!(ligne_action(&s, auj()).unwrap(), "Rien depuis la relance, 14 jours");
        let (t, x) = cadre_action(&c, &Action::ClasserSansReponse { jours: 14 }, auj());
        assert_eq!(t, "Toujours rien après la relance");
        assert_eq!(x, "14 jours depuis la relance. Tu peux la classer en « Sans réponse ».");
    }

    #[test]
    fn entretiens_a_venir_et_passes() {
        let mut c = cand(3, "Logistique des Trois Frontières", Statut::Entretien, j(2026, 11, 5));
        c.entretien =
            Some(Entretien { jour: j(2026, 11, 20), heure: Heure::nouvelle(9, 30), lieu: "Zone Actipôle".into() });
        let s = situation(&c, auj(), 14);
        assert_eq!(s.groupe, Groupe::AFaire);
        assert_eq!(ligne_action(&s, auj()).unwrap(), "Entretien le 20 nov. à 9 h 30");
        assert_eq!(ligne_action(&situation(&c, j(2026, 11, 19), 14), auj()).unwrap(), "Entretien demain à 9 h 30");
        assert_eq!(ligne_action(&situation(&c, j(2026, 11, 20), 14), auj()).unwrap(), "Entretien aujourd'hui à 9 h 30");
        assert_eq!(ou_on_en_est(&c, auj()).0, "Un entretien est prévu dans 2 jours.");
        assert_eq!(ou_on_en_est(&c, j(2026, 11, 19)).0, "L'entretien est demain.");
        assert_eq!(ou_on_en_est(&c, j(2026, 11, 20)).0, "L'entretien est aujourd'hui.");
        let Some(a) = s.action.clone() else { panic!() };
        assert_eq!(
            cadre_action(&c, &a, auj()),
            ("Entretien vendredi 20 novembre à 9 h 30".into(), "Zone Actipôle. Pense à prendre ton CV.".into())
        );
        // Passé : en attente de la réponse
        let s = situation(&c, j(2026, 11, 21), 14);
        assert_eq!(s.groupe, Groupe::EnAttente);
        assert_eq!(ou_on_en_est(&c, j(2026, 11, 21)).0, "Entretien passé le 20 nov., en attente de la réponse.");
        assert_eq!(phrase_calme(&c, &s, auj()), "Rien à faire pour l'instant. Note la réponse dès que tu l'as.");
        // Sans date : en attente
        c.entretien = None;
        assert_eq!(situation(&c, auj(), 14).groupe, Groupe::EnAttente);
    }

    #[test]
    fn terminees_et_textes_du_dossier() {
        for st in [Statut::Positive, Statut::Negative, Statut::SansReponse, Statut::Abandonnee] {
            let mut c = cand(4, "A", st, j(2026, 10, 1));
            c.statut_le = j(2026, 11, 10);
            let s = situation(&c, auj(), 14);
            assert_eq!(s.groupe, Groupe::Terminee, "{st:?}");
            let calme = phrase_calme(&c, &s, auj());
            if st == Statut::Positive {
                assert!(calme.starts_with("Bonne nouvelle."), "{calme}");
            } else {
                assert_eq!(calme, "Dossier fermé. Il reste ici pour garder une trace.");
            }
            assert!(ou_on_en_est(&c, auj()).0.ends_with(" le 10 nov."), "{st:?}");
        }
        let mut c = cand(5, "A", Statut::Envoyee, j(2026, 11, 2));
        c.moyen = Moyen::Mail;
        c.source = "Indeed".into();
        assert_eq!(
            ou_on_en_est(&c, auj()),
            (
                "Envoyée il y a 16 jours, pas encore de réponse.".into(),
                "Envoyée le lundi 2 novembre, par mail, offre vue sur Indeed.".into()
            )
        );
        c.envoyee_le = auj();
        c.moyen = Moyen::Autre;
        c.source = String::new();
        assert_eq!(ou_on_en_est(&c, auj()), ("Envoyée aujourd'hui.".into(), "Envoyée le mercredi 18 novembre.".into()));
        c.envoyee_le = j(2026, 11, 17);
        assert_eq!(ou_on_en_est(&c, auj()).0, "Envoyée hier, pas encore de réponse.");
    }

    #[test]
    fn message_de_relance_professionnel() {
        let mut c = cand(6, "Transports Moselle Fret", Statut::Envoyee, j(2026, 11, 2));
        let m = message_relance(&c, auj());
        assert_eq!(
            m,
            "Madame, Monsieur,\n\nLe 2 novembre, je vous ai adressé ma candidature au poste d'agent de quai à Woippy. \
             Je souhaiterais savoir si elle a pu être étudiée.\n\nCe poste m'intéresse toujours et je reste disponible \
             pour un entretien, à la date qui vous conviendra.\n\nJe vous prie d'agréer, Madame, Monsieur, mes \
             salutations distinguées."
        );
        c.moyen = Moyen::Mail;
        c.poste = "Préparateur de commandes".into();
        c.ville = String::new();
        c.envoyee_le = j(2025, 12, 1);
        let m = message_relance(&c, auj());
        assert!(m.starts_with("Objet : Ma candidature au poste de préparateur de commandes\n\nMadame, Monsieur,\n\nLe 1er décembre 2025, "), "{m}");
        assert!(m.contains("au poste de préparateur de commandes. Je souhaiterais"));
        c.poste = "CACES 3".into();
        assert!(message_relance(&c, auj()).contains("au poste de CACES 3."));
        // Professionnel : vouvoiement, jamais de tutoiement ni de formule familière
        for mot in [" tu ", " te ", " ton ", "Salut", "Coucou", "Bisous", "Cordialement,\n"] {
            assert!(!m.contains(mot), "{mot}");
        }
        assert!(!m.contains(['\u{2013}', '\u{2014}']));
    }

    #[test]
    fn bilan_rangement_et_carte() {
        let mut l = vec![
            cand(1, "Envoyée récente", Statut::Envoyee, j(2026, 11, 16)),
            cand(2, "Relance vieille", Statut::Envoyee, j(2026, 10, 20)),
            cand(3, "Relance moins vieille", Statut::Envoyee, j(2026, 11, 1)),
            cand(4, "Positive", Statut::Positive, j(2026, 11, 3)),
            cand(5, "Abandonnée", Statut::Abandonnee, j(2026, 10, 3)),
            cand(6, "Entretien", Statut::Entretien, j(2026, 11, 5)),
        ];
        l[5].entretien = Some(Entretien { jour: j(2026, 11, 19), heure: Heure::nouvelle(14, 0), lieu: String::new() });
        let b = bilan(&l, auj());
        assert_eq!((b.envois_du_mois, b.comptees, b.reponses, b.taux()), (4, 5, 2, Some(40)));
        assert_eq!(b.lignes(), ["4 envois en novembre", "2 réponses sur 5 candidatures, soit 40 %"]);
        let r = ranger(&l, auj(), 14);
        let ids: Vec<(Groupe, Vec<i64>)> = r.iter().map(|(g, x)| (*g, x.iter().map(|(c, _)| c.id).collect())).collect();
        assert_eq!(
            ids,
            vec![(Groupe::AFaire, vec![6, 2, 3]), (Groupe::EnAttente, vec![1]), (Groupe::Terminee, vec![4, 5])]
        );
        let carte = resume_carte(&l, auj(), 14);
        assert_eq!(carte.en_attente, 4);
        assert_eq!(carte.relances, 2);
        assert_eq!(carte.taux, Some(40));
        assert_eq!(carte.prochain_entretien, Some(("Entretien demain à 14 h : Entretien".into(), true)));
        assert_eq!(carte.envois, "4 envois en novembre");
        // Rien du tout
        let vide = bilan(&[], auj());
        assert_eq!(vide.lignes(), ["Aucun envoi en novembre"]);
        assert!(ranger(&[], auj(), 14).is_empty());
        assert_eq!(
            bilan(&[cand(1, "A", Statut::Envoyee, j(2026, 9, 1))], auj()).lignes(),
            ["Aucun envoi en novembre", "Aucune réponse sur 1 candidature"]
        );
    }

    #[test]
    fn bulles_dues_une_par_evenement() {
        let mut l = vec![
            cand(1, "Transports Moselle Fret", Statut::Envoyee, j(2026, 11, 4)),
            cand(2, "Brico Lorraine", Statut::Envoyee, j(2026, 11, 10)),
            cand(3, "Hôtel des Remparts", Statut::Entretien, j(2026, 11, 1)),
        ];
        l[2].entretien = Some(Entretien { jour: j(2026, 11, 19), heure: Heure::nouvelle(9, 30), lieu: "Nancy".into() });
        let d = bulles_dues(&l, auj(), 14);
        assert_eq!(
            d.iter().map(|x| x.cle.as_str()).collect::<Vec<_>>(),
            ["relance:1:2026-11-04", "entretien:3:2026-11-19T09:30:00"]
        );
        assert_eq!(
            texte_bulle(&d[..1]).unwrap(),
            (
                "Une relance à faire".into(),
                "Transports Moselle Fret (agent de quai) : pas de réponse depuis 14 jours. Ouvre le dossier pour copier \
                 un message."
                    .into()
            )
        );
        assert_eq!(
            texte_bulle(&d[1..]).unwrap(),
            ("Entretien demain".into(), "Hôtel des Remparts, à 9 h 30, Nancy.".into())
        );
        // Deux jours avant l'entretien : pas encore
        assert_eq!(bulles_dues(&l, j(2026, 11, 17), 14).len(), 0);
        // Plusieurs relances : une seule bulle qui les regroupe
        let r = bulles_dues(&l, j(2026, 11, 26), 14);
        assert_eq!(texte_bulle(&r).unwrap().0, "2 relances à faire");
        assert_eq!(texte_bulle(&r).unwrap().1, "Transports Moselle Fret et Brico Lorraine attendent une relance.");
        let mut trois = r.clone();
        trois.push(BulleDue { cle: "x".into(), candidature: 9, entreprise: "C".into(), genre: r[0].genre.clone() });
        trois.push(BulleDue { cle: "y".into(), candidature: 10, entreprise: "D".into(), genre: r[0].genre.clone() });
        assert_eq!(
            texte_bulle(&trois).unwrap().1,
            "Transports Moselle Fret, Brico Lorraine et 2 autres attendent une relance."
        );
        assert_eq!(texte_bulle(&[]), None);
        // Les limites de Mindset
        let mut long = l[0].clone();
        long.entreprise = "x".repeat(300);
        let (t, x) = texte_bulle(&bulles_dues(&[long], auj(), 14)).unwrap();
        assert!(t.chars().count() <= 63 && x.chars().count() <= 250 && x.ends_with("..."));
    }

    #[test]
    fn delais_bornes() {
        assert_eq!(delai_borne(0), DELAI_MIN);
        assert_eq!(delai_borne(14), 14);
        assert_eq!(delai_borne(500), DELAI_MAX);
        assert_eq!(couper("abc", 3), "abc");
        assert_eq!(couper("abcdef", 5), "ab...");
    }
}
