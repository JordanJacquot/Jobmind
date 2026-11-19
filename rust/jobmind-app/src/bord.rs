//! Étape 12 : le tableau de bord (onglet Candidatures, « Mon bilan »), l'export des candidatures en CSV et en PDF, la
//! sauvegarde et sa restauration (Mon profil, Réglages). Le réglage du rafraîchissement automatique des recherches est
//! parti avec Adzuna (20/11/2026), la seule source qu'il refaisait.
//!
//! Tout fichier passe par une boîte de Windows ouverte sur un fil à part, posée sur la fenêtre de Jobmind, jamais une
//! autre fenêtre : « Enregistrer sous » (le dossier Documents la première fois) pour un export ou une sauvegarde,
//! « Ouvrir » pour une sauvegarde à remettre. Le geste prépare le fichier ([`Fichier`]) ; le programme ouvre la boîte,
//! écrit ou lit, puis rapporte ici ([`Bord::retour`]).
//!
//! La sauvegarde (jobmind-storage, `sauvegarde`) : la base copiée par `VACUUM INTO` et les CV importés, dans un .zip ;
//! jamais les clés (data\cles.json : chiffrées pour ce PC et ce compte Windows, elles ne s'ouvriraient pas ailleurs),
//! ni la boîte mail branchée, ni les connexions aux sites. Remettre une sauvegarde : le fichier entièrement vérifié
//! avant toute question, puis « Remettre cette sauvegarde ? » ; une copie de l'état actuel gardée avant (dossier
//! `sauvegardes`), la base fermée, remplacée, rouverte ; au moindre souci, tout revient comme avant.

use std::path::Path;

use jobmind_core::Jour;
use jobmind_core::tableau::{self, Tableau};
use jobmind_storage::sauvegarde::{self, Contenu, ErreurSauvegarde, Resume};
use jobmind_storage::{Base, Donnees, Moment, candidatures, offres as base_offres};
use serde_json::{Value, json};

use crate::suivi::{Fait, PAS_ENREGISTRE, SANS_BASE};

/// Les ids des gestes (les actions de la page).
pub const CSV: &str = "x-csv";
pub const PDF: &str = "x-pdf";
pub const SAUVER: &str = "s-sauver";
pub const RESTAURER: &str = "s-restaurer";
pub const OUI: &str = "s-oui";
pub const NON: &str = "s-non";
const GESTES: [&str; 6] = [CSV, PDF, SAUVER, RESTAURER, OUI, NON];

/// Un geste de l'étape 12.
pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Ce qui s'enregistre ou s'ouvre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genre {
    Csv,
    Pdf,
    Zip,
}

impl Genre {
    /// « le tableau », « le PDF », « la sauvegarde » (pour les phrases).
    fn quoi(self) -> &'static str {
        match self {
            Genre::Csv => "Le tableau de tes candidatures",
            Genre::Pdf => "Le PDF de tes candidatures",
            Genre::Zip => "Ta sauvegarde",
        }
    }
}

/// Ce que le programme doit faire avec une boîte de Windows. Aucune donnée secrète ici.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fichier {
    /// « Enregistrer sous » : ce nom proposé, puis ces octets écrits (à côté puis renommés)
    Enregistrer { genre: Genre, nom: String, octets: Vec<u8> },
    /// « Ouvrir » une sauvegarde, puis la lire et la vérifier sur le même fil
    Ouvrir,
}

/// Ce que le fil de la boîte rapporte.
#[derive(Debug)]
pub enum RetourFichier {
    /// Écrit (le nom du fichier seulement, jamais le chemin entier dans une phrase)
    Enregistre { genre: Genre, nom: String },
    /// La boîte fermée sans choix
    Annule,
    /// La boîte n'a pas pu s'ouvrir, ou le fichier pas pu s'écrire (la raison, pour le journal)
    Erreur(String),
    /// Une sauvegarde choisie, lue et vérifiée (ou refusée)
    Lu { nom: String, contenu: Result<Box<Contenu>, ErreurSauvegarde> },
}

/// Une sauvegarde lue et vérifiée, qui attend le « oui » de Jordan.
#[derive(Debug)]
pub struct Attente {
    pub nom: String,
    pub contenu: Box<Contenu>,
}

/// L'état de l'étape 12 sur le fil de la fenêtre : une boîte ouverte, une sauvegarde qui attend sa réponse, la
/// dernière phrase dite dans Réglages.
#[derive(Debug, Default)]
pub struct Bord {
    pub boite: bool,
    pub attente: Option<Attente>,
    pub annonce: Option<(String, &'static str)>,
}

/// Ce qu'il faut pour les gestes.
pub struct Contexte<'a> {
    pub base: Option<&'a Base>,
    pub dossier: &'a Path,
}

fn message(m: impl Into<String>) -> Fait {
    Fait { message: Some(m.into()), ..Fait::default() }
}

/// « jeudi 19 novembre 2026 à 14 h 05 ».
pub fn quand(m: Moment) -> String {
    let j = m.jour();
    let jour = j.long(Jour { annee: j.annee + 1, ..j });
    if m.minute == 0 { format!("{jour} à {} h", m.heure) } else { format!("{jour} à {} h {:02}", m.heure, m.minute) }
}

/// « 12 candidatures », « 1 candidature », « 0 offre ».
fn n(nombre: usize, un: &str, plusieurs: &str) -> String {
    if nombre <= 1 { format!("{nombre} {un}") } else { format!("{nombre} {plusieurs}") }
}

/// Le résumé d'une sauvegarde en une phrase : « Faite le jeudi 19 novembre 2026 à 14 h 05 : 12 candidatures,
/// 140 offres et 2 CV. »
pub fn phrase_resume(r: &Resume) -> String {
    format!(
        "Faite le {} : {}, {} et {}.",
        quand(r.faite_le),
        n(r.candidatures as usize, "candidature", "candidatures"),
        n(r.offres as usize, "offre", "offres"),
        n(r.cv as usize, "CV", "CV")
    )
}

impl Bord {
    /// Un geste de la page : vérifié, puis préparé. « Oui, remettre » passe par [`Bord::restaurer`] (il faut les données
    /// entières).
    pub fn geste(&mut self, ctx: &Contexte, id: &str, _valeur: &Value, maintenant: Moment) -> Fait {
        if id == NON {
            if self.attente.take().is_some() {
                self.annonce = Some(("Rien n'a été changé.".into(), "ok"));
                return Fait {
                    message: Some("Rien n'a été changé.".into()),
                    journal: Some("Sauvegarde : remise annulée par Jordan".into()),
                    change: true,
                    ..Fait::default()
                };
            }
            return Fait::default();
        }
        // (remettre une sauvegarde reste possible quand la base ne s'ouvre pas : c'est justement là qu'il le faut)
        if id == RESTAURER && ctx.base.is_none() {
            if self.boite {
                return message("Une fenêtre pour choisir un fichier est déjà ouverte.");
            }
            self.attente = None;
            self.boite = true;
            return Fait {
                journal: Some(format!("Bord : {id} (base pas ouverte), boîte de Windows demandée")),
                fichier: Some(Fichier::Ouvrir),
                ..Fait::default()
            };
        }
        let Some(base) = ctx.base else {
            return Fait::refus(SANS_BASE, format!("Bord : {id} impossible, la base n'est pas ouverte"));
        };
        if self.boite {
            return message("Une fenêtre pour choisir un fichier est déjà ouverte.");
        }
        if self.attente.is_some() && id != RESTAURER {
            return message("Réponds d'abord à la question sur la sauvegarde.");
        }
        let jour = maintenant.jour();
        let fichier = match id {
            CSV | PDF => {
                let liste = match candidatures::lister(base.connexion()) {
                    Ok((l, _)) => l,
                    Err(e) => return Fait::refus(PAS_ENREGISTRE, format!("Export : base illisible ({e})")),
                };
                if liste.is_empty() {
                    return message("Aucune candidature à exporter pour l'instant.");
                }
                let (genre, octets, ext) = if id == CSV {
                    (Genre::Csv, jobmind_core::export::csv(&liste, jour), "csv")
                } else {
                    (Genre::Pdf, jobmind_core::export::pdf(&liste, jour), "pdf")
                };
                Fichier::Enregistrer { genre, nom: jobmind_core::export::nom_fichier(ext, jour), octets }
            }
            SAUVER => match sauvegarde::faire(base, ctx.dossier, maintenant) {
                Ok((octets, _)) => {
                    Fichier::Enregistrer { genre: Genre::Zip, nom: sauvegarde::nom_fichier(maintenant), octets }
                }
                Err(e) => {
                    return Fait {
                        message: Some(e.message()),
                        journal: Some(format!("Sauvegarde : pas faite ({e})")),
                        ..Fait::default()
                    };
                }
            },
            RESTAURER => {
                self.attente = None;
                Fichier::Ouvrir
            }
            _ => return Fait::default(),
        };
        self.boite = true;
        Fait {
            journal: Some(format!("Bord : {id}, boîte de Windows demandée")),
            fichier: Some(fichier),
            ..Fait::default()
        }
    }

    /// Ce que le fil de la boîte a rapporté.
    pub fn retour(&mut self, r: RetourFichier) -> Fait {
        self.boite = false;
        match r {
            RetourFichier::Annule => {
                Fait { journal: Some("Bord : boîte fermée sans choix".into()), ..Fait::default() }
            }
            RetourFichier::Erreur(e) => {
                let t = "Le fichier n'a pas pu être enregistré. Choisis un autre dossier, par exemple Documents, puis \
                         réessaie.";
                self.annonce = Some((t.into(), "erreur"));
                Fait::refus(t, format!("Bord : fichier non écrit ({e})"))
            }
            RetourFichier::Enregistre { genre, nom } => {
                let t = format!("{} est enregistré : « {nom} ».", genre.quoi());
                let t = if genre == Genre::Zip { t.replace("enregistré", "enregistrée") } else { t };
                self.annonce = Some((t.clone(), "ok"));
                Fait {
                    message: Some(t),
                    bravo: true,
                    journal: Some(format!("Bord : {genre:?} enregistré")),
                    change: true,
                    ..Fait::default()
                }
            }
            RetourFichier::Lu { nom, contenu } => match contenu {
                Ok(c) => {
                    self.annonce = None;
                    self.attente = Some(Attente { nom, contenu: c });
                    Fait {
                        journal: Some("Sauvegarde : fichier choisi, lu et vérifié ; Jordan doit confirmer".into()),
                        ordre: Some(json!({ "kit": "accueil", "sauvegarde": "question" })),
                        change: true,
                        ..Fait::default()
                    }
                }
                Err(e) => {
                    let t = e.message();
                    self.annonce = Some((t.clone(), "erreur"));
                    Fait::refus(t, format!("Sauvegarde : fichier refusé ({e})"))
                }
            },
        }
    }

    /// « Oui, remettre cette sauvegarde » : une copie de l'état actuel d'abord, puis la base fermée, remplacée par
    /// celle de la sauvegarde et rouverte (migrée si elle est plus ancienne). Rien n'est fait pendant qu'autre chose
    /// tourne (`occupee` : ce qui tourne, en mots simples). Au moindre souci, tout revient comme avant.
    ///
    /// La base ne s'ouvre pas, ou la copie de l'état actuel ne peut pas être faite (base abîmée) : les fichiers de la
    /// base sont gardés tels quels dans le dossier des sauvegardes (`remplacer_base_abimee`), puis la remise suit le
    /// même chemin. La phrase est dite dans la carte de la sauvegarde seulement (pas en double en haut de l'écran).
    pub fn restaurer(&mut self, d: &mut Donnees, occupee: Option<&str>, maintenant: Moment) -> Fait {
        let Some(a) = self.attente.take() else { return Fait::default() };
        if let Some(q) = occupee {
            self.attente = Some(a);
            return message(format!("Attends la fin de {q}, puis réessaie."));
        }
        let dossier = d.dossier().to_path_buf();
        let copie = match d.base.as_ref().map(|b| sauvegarde::avant_restauration(b, &dossier, maintenant)) {
            Some(Ok(p)) => Ok(p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
            Some(Err(e)) => Err(format!("copie d'avant pas faite ({e})")),
            None => Err("base pas ouverte".to_string()),
        };
        // (la base fermée avant d'être remplacée)
        d.base = None;
        let annoncer = |bord: &mut Bord, t: &str, genre: &'static str, journal: String| {
            bord.annonce = Some((t.into(), genre));
            Fait { journal: Some(journal), change: true, ..Fait::default() }
        };
        let rouvrir = |d: &mut Donnees| match Base::ouvrir(&dossier, maintenant) {
            Ok((b, _)) => {
                d.base = Some(b);
                None
            }
            Err(e) => Some(e.to_string()),
        };
        let echec = |bord: &mut Bord, d: &mut Donnees, raison: String| {
            let _ = sauvegarde::annuler(&dossier);
            let encore = rouvrir(d).map(|e| format!(" ; base pas rouverte : {e}")).unwrap_or_default();
            let t = "La sauvegarde n'a pas pu être remise. Rien n'a été changé.";
            annoncer(bord, t, "erreur", format!("Sauvegarde : remise ratée ({raison}), tout remis comme avant{encore}"))
        };
        let copie = match copie {
            Ok(nom) => {
                if let Err(e) = sauvegarde::remplacer(&dossier, &a.contenu) {
                    return echec(self, d, e.to_string());
                }
                format!("l'état d'avant est gardé dans sauvegardes ({nom})")
            }
            // (base abîmée ou pas ouverte : ses fichiers gardés tels quels, puis la même remise)
            Err(raison) => match sauvegarde::remplacer_base_abimee(&dossier, &a.contenu, maintenant) {
                Ok(Some(p)) => format!(
                    "{raison} : la base d'avant est gardée telle quelle dans sauvegardes ({})",
                    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
                ),
                Ok(None) => format!("{raison} : il n'y avait pas de base avant"),
                Err(e) => {
                    let t = "Jobmind n'a pas pu garder une copie de tes données actuelles. Rien n'a été changé.";
                    let encore = rouvrir(d).map(|e| format!(" ; base pas rouverte : {e}")).unwrap_or_default();
                    return annoncer(
                        self,
                        t,
                        "erreur",
                        format!("Sauvegarde : {raison}, copie de la base abîmée pas faite ({e}), rien changé{encore}"),
                    );
                }
            },
        };
        if let Some(e) = rouvrir(d) {
            return echec(self, d, format!("base remise pas ouverte : {e}"));
        }
        let mut journal = format!("Sauvegarde : remise ; {copie}");
        match sauvegarde::confirmer_bilan(&dossier) {
            Ok(None) => {}
            Ok(Some(l)) => journal.push_str(&format!(" ; {l}")),
            Err(e) => {
                // (la marque pas effacée : au prochain démarrage, tout reviendrait comme avant ; défait maintenant)
                d.base = None;
                return echec(self, d, format!("confirmation impossible ({e})"));
            }
        }
        let t = format!(
            "Ta sauvegarde est remise ({}). Tes clés et ta boîte mail restent celles de ce PC.",
            phrase_resume(&a.contenu.resume).trim_end_matches('.').replacen("Faite le", "faite le", 1)
        );
        let mut f = annoncer(self, &t, "ok", journal);
        f.bravo = true;
        f
    }
}

/// Mon profil, Réglages : l'export et la sauvegarde.
pub fn vue_reglages(bord: &Bord, dossier: &Path, maintenant: Moment) -> Value {
    let autos = sauvegarde::automatiques(dossier);
    let auj = maintenant.jour();
    let auto = match autos.first() {
        None => "Jobmind fait aussi une copie toute seule chaque jour, dans son dossier sur ce PC (les 7 dernières \
                 sont gardées)."
            .to_string(),
        Some((_, j)) => format!(
            "Jobmind fait aussi une copie toute seule chaque jour, dans son dossier sur ce PC (les 7 dernières sont \
             gardées). La dernière : {}.",
            if *j == auj { "aujourd'hui".to_string() } else { format!("le {}", j.long(auj)) }
        ),
    };
    let attente = bord.attente.as_ref().map(|a| {
        json!({
            "titre": "Remettre cette sauvegarde ?",
            "fichier": a.nom,
            "detail": phrase_resume(&a.contenu.resume),
            "texte": "Elle remplacera tout ce qui est dans Jobmind maintenant : candidatures, offres, profil et CV. \
                      Une copie de ce qui est là aujourd'hui est gardée avant, au cas où.",
            "oui": "Oui, remettre cette sauvegarde",
            "non": "Annuler",
        })
    });
    json!({
        "export": {
            "titre": "Exporter mes candidatures",
            "texte": "Un tableau à ouvrir dans Excel, ou un PDF à imprimer. Tu choisis où le ranger (Documents au \
                      départ).",
            "csv": "En tableau (CSV)",
            "pdf": "En PDF",
        },
        "sauvegarde": {
            "titre": "Sauvegarder mes données",
            "texte": "Un seul fichier avec tes candidatures, tes offres, ton profil et tes CV, à garder sur une clé USB \
                      ou un autre disque.",
            "cles": "Tes clés et ta boîte mail ne sont pas dans la sauvegarde : elles ne marchent que sur ce PC. Après \
                     une remise sur un autre PC, tu les redonnes dans Mes clés et Ma boîte mail.",
            "auto": auto,
            "sauver": "Sauvegarder maintenant",
            "restaurer": "Remettre une sauvegarde",
            "attente": attente,
        },
        "boite": bord.boite,
        "annonce": bord.annonce.as_ref().map(|(t, g)| json!({ "texte": t, "genre": g })),
    })
}

/// Le tableau de bord de l'onglet Candidatures (« Mon bilan »), écrit pour l'écran.
pub fn vue_tableau(t: &Tableau) -> Value {
    let carte = |nombre: String, texte: String, ton: &str| json!({ "nombre": nombre, "texte": texte, "ton": ton });
    let cartes = vec![
        carte(
            t.semaine.to_string(),
            if t.semaine == 1 { "envoi cette semaine" } else { "envois cette semaine" }.into(),
            "",
        ),
        carte(t.mois.to_string(), if t.mois == 1 { "envoi ce mois-ci" } else { "envois ce mois-ci" }.into(), ""),
        carte(t.reponses.to_string(), if t.reponses == 1 { "réponse reçue" } else { "réponses reçues" }.into(), ""),
        carte(format!("{} %", t.taux.unwrap_or(0)), "de réponses".into(), ""),
        carte(
            t.entretiens.to_string(),
            match t.entretiens_a_venir {
                0 => if t.entretiens == 1 { "entretien" } else { "entretiens" }.to_string(),
                k => format!("{}, dont {k} à venir", if t.entretiens == 1 { "entretien" } else { "entretiens" }),
            },
            if t.entretiens_a_venir > 0 { "bien" } else { "" },
        ),
        carte(
            t.relances.to_string(),
            if t.relances == 1 { "relance à faire" } else { "relances à faire" }.into(),
            if t.relances > 0 { "attention" } else { "" },
        ),
    ];

    let total_statuts: usize = t.statuts.iter().map(|(_, k)| k).sum();
    let statuts: Vec<Value> = t
        .statuts
        .iter()
        .map(|(s, k)| {
            json!({
                "id": s.code(),
                "nom": s.nom(),
                "nombre": k,
                "part": (k * 1000).checked_div(total_statuts).unwrap_or(0) as u64,
            })
        })
        .collect();
    let max_source = t.par_source.iter().map(|(_, k)| *k).max().unwrap_or(0);
    json!({
        "titre": "Mon bilan",
        "sous": match t.total {
            0 => "Aucune candidature pour l'instant.".to_string(),
            k => format!("{} en tout, depuis le début.", n(k, "candidature", "candidatures")),
        },
        "cartes": cartes,
        "barres_titre": "Mes envois, semaine par semaine",
        "barres": t.barres.iter().map(|b| json!({ "texte": b.texte, "nombre": b.nombre, "part": b.part })).collect::<Vec<_>>(),
        "statuts_titre": "Où en sont mes candidatures",
        "statuts": statuts,
        "offres_titre": "Offres nouvelles cette semaine",
        "offres_texte": match t.offres_nouvelles {
            0 => "Aucune offre nouvelle ces 7 derniers jours.".to_string(),
            k => format!("{} ces 7 derniers jours.", n(k, "offre nouvelle", "offres nouvelles")),
        },
        "sources": t.par_source.iter().map(|(nom, k)| json!({
            "nom": nom,
            "nombre": k,
            "part": if max_source == 0 { 0 } else { (k * 100).div_ceil(max_source) as u64 },
        })).collect::<Vec<_>>(),
        "exporter": "Exporter",
        "csv": "En tableau (CSV)",
        "pdf": "En PDF",
        "fermer": "Revenir à mes candidatures",
    })
}

/// Le tableau de bord d'après la base (rien : la base illisible, et la raison).
pub fn tableau_de(base: &Base, aujourdhui: Jour) -> Result<Tableau, String> {
    let c = base.connexion();
    let (liste, _) = candidatures::lister(c).map_err(|e| e.to_string())?;
    let delai = candidatures::delai_relance(c).map_err(|e| e.to_string())?;
    let offres = base_offres::lister(c).map_err(|e| e.to_string())?;
    Ok(tableau::calculer(&liste, &offres, aujourdhui, delai))
}

#[cfg(test)]
mod tests;
