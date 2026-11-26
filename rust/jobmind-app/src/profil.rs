//! Les gestes de Mon profil (étape 3 du plan) : les CV (importer un fichier, coller, corriger, choisir le principal,
//! supprimer), « Ce que je cherche », les textes pour le style, les clés. Chaque saisie est vérifiée ici par
//! jobmind-core (jamais confiance dans la page), puis écrite dans la base, le dossier `cv` ou le fichier des clés.
//!
//! L'import d'un fichier se fait en trois temps, sans jamais bloquer la fenêtre : « Importer un fichier » ([`IMPORTER`])
//! demande au programme d'ouvrir la boîte « Ouvrir » de Windows sur un fil à part ([`Fait::importer`]) ; le fichier
//! choisi est lu sur ce fil ([`lire_fichier`] : taille vérifiée avant de lire, puis jobmind-lecture) ; le résultat
//! revient ici ([`Profil::import_fini`]) : le fichier est copié dans le dossier `cv` des données, son texte remis au
//! propre et montré pour être corrigé. Un import a son numéro (`jeton`) : un résultat arrivé trop tard est laissé.
//!
//! 0.2.0, « Ce que Jobmind a compris de ton CV » : la fiche du CV principal (jobmind_core::parcours : les métiers
//! exercés, les compétences, les diplômes, les permis et habilitations) est refaite d'après son texte chaque fois
//! qu'il change (elle est gardée en mémoire tant qu'il ne change pas), puis corrigée par ce que la personne y a retiré
//! ([`PC_RETIRER`]) ou ajouté ([`PC_AJOUTER`]), gardé dans la base (jobmind_storage::parcours). Les corrections suivent
//! la personne : un autre CV principal, ou le même corrigé, donne une fiche refaite où les mêmes corrections
//! s'appliquent.
//!
//! Le journal ne reçoit jamais un texte de Jordan ni une clé : seulement ce qui a été fait.

use std::path::Path;

use jobmind_core::cle::{Cle, Service, verifier_cle};
use jobmind_core::parcours::{self as regles_parcours, Correction, Liste, Parcours};
use jobmind_core::profil::{
    CV_NOMBRE_MAX, CV_TEXTE_MAX, STYLE_NOMBRE_MAX, couper, nettoyer_texte, nom_depuis_fichier, verifier_nom_cv,
    verifier_style, verifier_texte_cv,
};
use jobmind_core::recherche::{SaisieRecherche, verifier_criteres};
use jobmind_lecture::{Erreur, Format, Lu, TAILLE_MAX};
use jobmind_storage::cles::Cles;
use jobmind_storage::parcours as base_parcours;
use jobmind_storage::profil::{self, GenreCv, NouveauCv};
use jobmind_storage::rusqlite;
use jobmind_storage::{Base, Moment};
use serde_json::{Value, json};

use crate::suivi::{Fait, PAS_ENREGISTRE};
use crate::vue_profil::{VueProfil, taille_texte};

/// Les ids des gestes du profil (les actions de la page).
pub const IMPORTER: &str = "cv-importer";
pub const CV_OUVRIR: &str = "cv-ouvrir";
pub const CV_ENREGISTRER: &str = "cv-enregistrer";
pub const CV_PRINCIPAL: &str = "cv-principal";
pub const CV_SUPPRIMER: &str = "cv-supprimer";
pub const RECHERCHE: &str = "recherche-enregistrer";
pub const STYLE_AJOUTER: &str = "style-ajouter";
pub const STYLE_OUVRIR: &str = "style-ouvrir";
pub const STYLE_SUPPRIMER: &str = "style-supprimer";
pub const CLE_GARDER: &str = "cle-enregistrer";
pub const CLE_EFFACER: &str = "cle-effacer";
pub const COPIE: &str = "p-copie";
/// 0.2.0 : une ligne ajoutée à « Ce que Jobmind a compris de ton CV » (`liste`, `texte`), une ligne retirée (`liste`,
/// `cle`).
pub const PC_AJOUTER: &str = "pc-ajouter";
pub const PC_RETIRER: &str = "pc-retirer";
const GESTES: [&str; 14] = [
    IMPORTER,
    CV_OUVRIR,
    CV_ENREGISTRER,
    CV_PRINCIPAL,
    CV_SUPPRIMER,
    RECHERCHE,
    STYLE_AJOUTER,
    STYLE_OUVRIR,
    STYLE_SUPPRIMER,
    CLE_GARDER,
    CLE_EFFACER,
    COPIE,
    PC_AJOUTER,
    PC_RETIRER,
];

/// Un geste du profil.
pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Le message quand le fichier des clés n'a pas pu être écrit.
pub const CLE_NON_GARDEE: &str = "Jobmind n'a pas pu garder cette clé. Réessaie dans un instant.";

/// Un texte d'un objet de la page (absent ou d'un autre type : vide).
fn texte(v: &Value, champ: &str) -> String {
    v.get(champ).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// Un numéro (un entier positif).
fn numero(v: &Value) -> Option<i64> {
    v.get("id").and_then(Value::as_i64).filter(|n| *n > 0)
}

fn message(m: impl Into<String>) -> Fait {
    Fait { message: Some(m.into()), ..Fait::default() }
}

/// Un import en cours : son numéro, et le nom du fichier une fois choisi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnCours {
    pub jeton: u64,
    pub nom: Option<String>,
}

/// Ce que le fil de l'import rapporte.
#[derive(Debug)]
pub struct Import {
    pub jeton: u64,
    /// Le nom du fichier choisi (sans son dossier)
    pub nom: String,
    /// Ses octets (vides s'il n'a pas été lu : trop gros, illisible)
    pub octets: Vec<u8>,
    pub lu: Result<Lu, Erreur>,
}

/// Lit le fichier choisi : sa taille d'abord (trop gros : pas lu du tout), puis ses octets et son texte. Ne s'arrête
/// jamais net. Sur le fil de l'import.
pub fn lire_fichier(chemin: &Path, jeton: u64) -> Import {
    let nom = chemin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let refus = |e: Erreur| Import { jeton, nom: nom.clone(), octets: Vec::new(), lu: Err(e) };
    match std::fs::metadata(chemin) {
        Ok(m) if m.len() > TAILLE_MAX => return refus(Erreur::TropGros),
        Ok(m) if m.len() == 0 => return refus(Erreur::Vide),
        Ok(_) => {}
        Err(e) => return refus(Erreur::Abime(format!("fichier illisible ({e})"))),
    }
    let octets = match std::fs::read(chemin) {
        Ok(o) if o.len() as u64 > TAILLE_MAX => return refus(Erreur::TropGros),
        Ok(o) => o,
        Err(e) => return refus(Erreur::Abime(format!("fichier illisible ({e})"))),
    };
    let lu = jobmind_lecture::lire(&octets);
    Import { jeton, nom, octets, lu }
}

/// Ce que les gestes du profil touchent : la base, le dossier de données (les fichiers de CV) et les clés.
pub struct Contexte<'a> {
    pub base: Option<&'a Base>,
    pub dossier: &'a Path,
    pub cles: Option<&'a mut Cles>,
}

/// Le profil : le CV et le texte ouverts, l'import en cours.
#[derive(Debug, Clone, Default)]
pub struct Profil {
    pub cv_ouvert: Option<i64>,
    pub style_ouvert: Option<i64>,
    pub import: Option<EnCours>,
    /// Le numéro du prochain import
    jetons: u64,
    /// 0.2.0 : la fiche lue dans le CV principal, avant les corrections, et l'empreinte du texte dont elle vient
    /// (jobmind_core::parcours::empreinte) : refaite seulement quand le texte change
    fiche: Option<(u64, Parcours)>,
}

impl Profil {
    /// Une sauvegarde vient d'être remise : tout revient au départ, sauf le compteur des imports.
    pub fn apres_remise(&mut self) {
        let jetons = self.jetons;
        *self = Profil::default();
        self.jetons = jetons;
    }

    /// Ce que l'écran montre, d'après la base et les clés. Une base illisible : rien, et la raison pour le journal.
    pub fn vue(&mut self, base: Option<&Base>, cles: Option<&Cles>) -> (VueProfil, Option<String>) {
        let mut v = VueProfil {
            import: self.import.as_ref().map(|i| i.nom.clone()),
            cles: Cle::TOUTES.iter().map(|c| (*c, cles.and_then(|x| x.etat(*c).cloned()))).collect(),
            ..VueProfil::default()
        };
        let Some(b) = base else { return (v, None) };
        let c = b.connexion();
        let lu = (|| -> rusqlite::Result<_> {
            Ok((profil::lister_cv(c)?, profil::lister_style(c)?, profil::lire_recherche(c)?))
        })();
        match lu {
            Ok((cvs, style, recherche)) => {
                if !self.cv_ouvert.is_some_and(|id| cvs.iter().any(|x| x.id == id)) {
                    self.cv_ouvert = cvs.iter().find(|x| x.principal).or(cvs.first()).map(|x| x.id);
                }
                if !self.style_ouvert.is_some_and(|id| style.iter().any(|x| x.id == id)) {
                    self.style_ouvert = None;
                }
                v.cvs = cvs;
                v.cv_ouvert = self.cv_ouvert;
                v.style = style;
                v.style_ouvert = self.style_ouvert;
                v.recherche = recherche;
                v.criteres = profil::lire_criteres(c).unwrap_or_default();
                v.regles = crate::style::vue(Some(b));
                // (0.2.0 : la fiche du CV principal, corrigée par la personne ; des corrections illisibles : la
                // fiche telle que lue, et une ligne au journal)
                let principal = v.cvs.iter().find(|x| x.principal).map(|x| x.texte.clone());
                let mut raison = None;
                v.parcours = principal.map(|texte| {
                    let lue = self.fiche_lue(&texte);
                    match base_parcours::lire(c) {
                        Ok(corrections) => lue.corrige(&corrections),
                        Err(e) => {
                            raison = Some(format!("Profil : corrections du parcours illisibles ({e})"));
                            lue
                        }
                    }
                });
                (v, raison)
            }
            Err(e) => (v, Some(format!("Profil : base illisible ({e})"))),
        }
    }

    /// Un geste de la page : vérifié, puis écrit.
    pub fn geste(&mut self, ctx: &mut Contexte, id: &str, valeur: &Value, maintenant: Moment) -> Fait {
        if id == COPIE {
            let service = Service::depuis_code(&texte(valeur, "service")).map_or("?", Service::code);
            return Fait { journal: Some(format!("Clés : adresse de {service} copiée")), ..Fait::default() };
        }
        if matches!(id, CLE_GARDER | CLE_EFFACER) {
            return self.cle(ctx, id, valeur);
        }
        let Some(base) = ctx.base else {
            return Fait::refus(
                crate::suivi::SANS_BASE,
                format!("Profil : {id} impossible, la base n'est pas ouverte"),
            );
        };
        let r = match id {
            IMPORTER => self.importer(base),
            CV_OUVRIR => {
                self.cv_ouvert = numero(valeur);
                Ok(Fait { change: true, ..Fait::default() })
            }
            CV_ENREGISTRER => self.cv_enregistrer(base, valeur, maintenant),
            CV_PRINCIPAL => self.cv_principal(base, valeur),
            CV_SUPPRIMER => self.cv_supprimer(base, ctx.dossier, valeur),
            RECHERCHE => recherche(base, valeur, maintenant),
            STYLE_AJOUTER => self.style_ajouter(base, valeur, maintenant),
            STYLE_OUVRIR => {
                let id = numero(valeur);
                self.style_ouvert = if self.style_ouvert == id { None } else { id };
                Ok(Fait { change: true, ..Fait::default() })
            }
            STYLE_SUPPRIMER => self.style_supprimer(base, valeur),
            PC_AJOUTER => self.parcours_ajouter(base, valeur, maintenant),
            PC_RETIRER => self.parcours_retirer(base, valeur, maintenant),
            _ => Ok(Fait::default()),
        };
        r.unwrap_or_else(|e| Fait::refus(PAS_ENREGISTRE, format!("Profil : {id} non enregistré ({e})")))
    }

    /// 0.2.0 : la fiche lue dans ce texte de CV, avant les corrections (refaite seulement si le texte a changé).
    fn fiche_lue(&mut self, texte: &str) -> Parcours {
        let empreinte = regles_parcours::empreinte(texte);
        match &self.fiche {
            Some((e, p)) if *e == empreinte => p.clone(),
            _ => {
                let p = regles_parcours::lire(texte);
                self.fiche = Some((empreinte, p.clone()));
                p
            }
        }
    }

    /// 0.2.0 : la fiche lue dans le CV principal, avant les corrections (rien : aucun CV).
    fn fiche_du_principal(&mut self, base: &Base) -> rusqlite::Result<Option<Parcours>> {
        let principal = profil::lister_cv(base.connexion())?.into_iter().find(|x| x.principal);
        Ok(principal.map(|cv| self.fiche_lue(&cv.texte)))
    }

    /// 0.2.0 : une ligne ajoutée à la fiche : vérifiée, puis gardée. Le journal dit la liste, jamais la ligne.
    fn parcours_ajouter(&mut self, base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let Some(liste) = Liste::depuis_code(&texte(v, "liste")) else { return Ok(Fait::default()) };
        if self.fiche_du_principal(base)?.is_none() {
            return Ok(message("Ajoute d'abord ton CV : Jobmind y lira ton parcours."));
        }
        let t = match regles_parcours::verifier_ajout(liste, &texte(v, "texte")) {
            Ok(t) => t,
            Err(r) => return Ok(message(r)),
        };
        let c = base.connexion();
        if base_parcours::nombre_ajouts(c, liste)? >= regles_parcours::AJOUTS_MAX {
            return Ok(message("Cette liste est pleine : retire une ligne pour en ajouter une autre."));
        }
        let x = Correction { liste, ajout: true, cle: regles_parcours::cle_ligne(liste, &t), texte: t };
        base_parcours::garder(c, &x, maintenant)?;
        Ok(Fait {
            ordre: Some(json!({ "kit": "accueil", "parcours": liste.code() })),
            journal: Some(format!("Profil : parcours, une ligne ajoutée ({})", liste.code())),
            change: true,
            ..Fait::default()
        })
    }

    /// 0.2.0 : une ligne retirée de la fiche. Une ligne lue dans le CV : le retrait est gardé (elle ne reviendra pas
    /// à la prochaine lecture) ; une ligne ajoutée à la main : elle est effacée, il n'en reste rien.
    fn parcours_retirer(&mut self, base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let Some(liste) = Liste::depuis_code(&texte(v, "liste")) else { return Ok(Fait::default()) };
        let cle = texte(v, "cle");
        if cle.is_empty() || cle.chars().count() > 200 {
            return Ok(Fait::default());
        }
        let c = base.connexion();
        let lue = self.fiche_du_principal(base)?.is_some_and(|p| p.cles(liste).contains(&cle));
        if lue {
            base_parcours::garder(c, &Correction { liste, ajout: false, cle, texte: String::new() }, maintenant)?;
        } else if !base_parcours::effacer(c, liste, &cle)? {
            return Ok(Fait { change: true, ..Fait::default() });
        }
        Ok(Fait {
            journal: Some(format!("Profil : parcours, une ligne retirée ({})", liste.code())),
            change: true,
            ..Fait::default()
        })
    }

    fn importer(&mut self, base: &Base) -> rusqlite::Result<Fait> {
        if self.import.is_some() {
            return Ok(message("Un fichier est déjà en train d'être importé."));
        }
        if profil::nombre_cv(base.connexion())? >= CV_NOMBRE_MAX {
            return Ok(message(format!("Tu as déjà {CV_NOMBRE_MAX} CV : supprime-en un pour en ajouter un autre.")));
        }
        self.jetons += 1;
        self.import = Some(EnCours { jeton: self.jetons, nom: None });
        Ok(Fait {
            importer: true,
            change: true,
            journal: Some("Profil : boîte d'ouverture demandée".into()),
            ..Fait::default()
        })
    }

    /// La boîte « Ouvrir » a rendu un fichier : sa lecture commence (le nom est montré).
    pub fn import_choisi(&mut self, jeton: u64, nom: &str) -> bool {
        match self.import.as_mut() {
            Some(i) if i.jeton == jeton => {
                i.nom = Some(nom.to_string());
                true
            }
            _ => false,
        }
    }

    /// La boîte « Ouvrir » a été fermée sans choix, ou n'a pas pu s'ouvrir (la raison) : l'import s'arrête.
    pub fn import_annule(&mut self, jeton: u64, raison: Option<&str>) -> Fait {
        if !self.import.as_ref().is_some_and(|i| i.jeton == jeton) {
            return Fait::default();
        }
        self.import = None;
        match raison {
            None => Fait { change: true, journal: Some("Profil : import annulé".into()), ..Fait::default() },
            Some(r) => Fait::refus(
                "La fenêtre pour choisir un fichier n'a pas pu s'ouvrir. Tu peux coller le texte de ton CV.",
                format!("Profil : {r}"),
            ),
        }
    }

    /// La lecture a duré trop longtemps : l'import s'arrête (un résultat qui arriverait ensuite est laissé).
    pub fn import_trop_long(&mut self) -> Fait {
        self.import = None;
        Fait::refus(
            "Jobmind n'a pas réussi à lire ce fichier. Colle le texte de ton CV à la main.",
            "Profil : lecture du fichier trop longue, abandonnée".into(),
        )
    }

    /// Le résultat de l'import : le fichier copié dans le dossier `cv`, le CV ajouté et ouvert, son texte à relire.
    pub fn import_fini(&mut self, ctx: &mut Contexte, imp: Import, maintenant: Moment) -> Fait {
        if !self.import.as_ref().is_some_and(|i| i.jeton == imp.jeton) {
            return Fait {
                journal: Some("Profil : résultat d'un import abandonné, laissé".into()), ..Fait::default()
            };
        }
        self.import = None;
        let Some(base) = ctx.base else {
            return Fait::refus(
                crate::suivi::SANS_BASE,
                "Profil : import impossible, la base n'est pas ouverte".into(),
            );
        };
        // Le texte : lu, ou rien (un PDF scanné : le fichier est gardé, le texte se colle à la main)
        let (format, texte, mut avertissement) = match imp.lu {
            Ok(Lu { format, texte }) => (format, nettoyer_texte(&texte), None),
            Err(e @ Erreur::SansTexte(f)) => (f, String::new(), Some(e.message())),
            Err(e) => {
                return Fait::refus(e.message(), format!("Profil : fichier refusé, {}", e.raison()));
            }
        };
        let (texte, coupe) = couper(&texte, CV_TEXTE_MAX);
        let r = (|| -> Result<(i64, usize), String> {
            if profil::nombre_cv(base.connexion()).map_err(|e| e.to_string())? >= CV_NOMBRE_MAX {
                return Err("plein".into());
            }
            let fichier = profil::copier_fichier(ctx.dossier, &imp.octets, format.code(), maintenant)
                .map_err(|e| format!("copie du fichier ({e})"))?;
            let genre = if format == Format::Pdf { GenreCv::Pdf } else { GenreCv::Docx };
            let n = NouveauCv {
                nom: &nom_depuis_fichier(&imp.nom),
                fichier: Some(&fichier),
                genre,
                texte: &texte,
                taille: Some(imp.octets.len() as i64),
            };
            let id = match profil::ajouter_cv(base.connexion(), &n, maintenant) {
                Ok(id) => id,
                Err(e) => {
                    if let Some(c) = profil::chemin_fichier(ctx.dossier, &fichier) {
                        let _ = std::fs::remove_file(c);
                    }
                    return Err(e.to_string());
                }
            };
            Ok((id, texte.chars().count()))
        })();
        match r {
            Err(e) if e == "plein" => {
                message(format!("Tu as déjà {CV_NOMBRE_MAX} CV : supprime-en un pour en ajouter un autre."))
            }
            Err(e) => Fait::refus(PAS_ENREGISTRE, format!("Profil : CV non importé ({e})")),
            Ok((id, n)) => {
                self.cv_ouvert = Some(id);
                let mut m = match avertissement.take() {
                    Some(a) => format!("{a} Le fichier est gardé."),
                    None => "CV importé. Relis son texte et corrige ce qui a été mal lu.".to_string(),
                };
                if coupe {
                    m.push_str(" Il était très long : seul le début a été gardé.");
                }
                Fait {
                    message: Some(m),
                    bravo: n > 0,
                    ordre: Some(json!({ "kit": "accueil", "cv": id })),
                    journal: Some(format!(
                        "Profil : CV {id} importé ({}, {}, {n} caractères{})",
                        format.nom(),
                        taille_texte(imp.octets.len() as i64),
                        if coupe { ", coupé" } else { "" }
                    )),
                    change: true,
                    ..Fait::default()
                }
            }
        }
    }

    fn cv_enregistrer(&mut self, base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let nom = match verifier_nom_cv(&texte(v, "nom")) {
            Ok(n) => n,
            Err(r) => return Ok(message(r)),
        };
        let t = match verifier_texte_cv(&texte(v, "texte")) {
            Ok(t) => t,
            Err(r) => return Ok(message(r)),
        };
        let c = base.connexion();
        let (id, m, j) = match numero(v) {
            Some(id) => {
                if !profil::modifier_cv(c, id, &nom, &t, maintenant)? {
                    return Ok(Fait::refus("Ce CV n'existe plus.", format!("Profil : CV {id} introuvable")));
                }
                (id, "CV enregistré.", format!("Profil : CV {id} corrigé"))
            }
            None => {
                if profil::nombre_cv(c)? >= CV_NOMBRE_MAX {
                    return Ok(message(format!(
                        "Tu as déjà {CV_NOMBRE_MAX} CV : supprime-en un pour en ajouter un autre."
                    )));
                }
                let n = NouveauCv { nom: &nom, fichier: None, genre: GenreCv::Texte, texte: &t, taille: None };
                let id = profil::ajouter_cv(c, &n, maintenant)?;
                (id, "CV enregistré.", format!("Profil : CV {id} collé"))
            }
        };
        self.cv_ouvert = Some(id);
        Ok(Fait {
            message: Some(m.into()),
            bravo: true,
            ordre: Some(json!({ "kit": "accueil", "cv": id })),
            journal: Some(j),
            change: true,
            ..Fait::default()
        })
    }

    fn cv_principal(&mut self, base: &Base, v: &Value) -> rusqlite::Result<Fait> {
        let Some(id) = numero(v) else { return Ok(Fait::default()) };
        if !profil::rendre_principal(base.connexion(), id)? {
            return Ok(Fait::refus("Ce CV n'existe plus.", format!("Profil : CV {id} introuvable")));
        }
        self.cv_ouvert = Some(id);
        Ok(Fait {
            message: Some("C'est maintenant ton CV principal.".into()),
            journal: Some(format!("Profil : CV {id} principal")),
            change: true,
            ..Fait::default()
        })
    }

    fn cv_supprimer(&mut self, base: &Base, dossier: &Path, v: &Value) -> rusqlite::Result<Fait> {
        let Some(id) = numero(v) else { return Ok(Fait::default()) };
        let Some(x) = profil::supprimer_cv(base.connexion(), id)? else {
            return Ok(Fait { change: true, ..Fait::default() });
        };
        // (le fichier copié part avec lui, seulement s'il est bien dans le dossier cv)
        let mut j = format!("Profil : CV {id} supprimé");
        if let Some(f) = x.fichier.as_deref().and_then(|f| profil::chemin_fichier(dossier, f))
            && let Err(e) = std::fs::remove_file(&f)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            j.push_str(&format!(", fichier non effacé ({e})"));
        }
        if self.cv_ouvert == Some(id) {
            self.cv_ouvert = None;
        }
        Ok(Fait {
            message: Some("CV supprimé.".into()),
            ordre: Some(json!({ "kit": "accueil", "cv_supprime": id })),
            journal: Some(j),
            change: true,
            ..Fait::default()
        })
    }

    fn style_ajouter(&mut self, base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let c = base.connexion();
        if profil::nombre_style(c)? >= STYLE_NOMBRE_MAX {
            return Ok(message(format!(
                "Tu as déjà {STYLE_NOMBRE_MAX} textes : supprime-en un pour en ajouter un autre."
            )));
        }
        let (titre, t) = match verifier_style(&texte(v, "titre"), &texte(v, "texte")) {
            Ok(x) => x,
            Err(r) => return Ok(message(r)),
        };
        let id = profil::ajouter_style(c, &titre, &t, maintenant)?;
        self.style_ouvert = None;
        Ok(Fait {
            message: Some("Texte ajouté.".into()),
            bravo: true,
            ordre: Some(json!({ "kit": "accueil", "style": id })),
            journal: Some(format!("Profil : texte {id} ajouté pour le style")),
            change: true,
            ..Fait::default()
        })
    }

    fn style_supprimer(&mut self, base: &Base, v: &Value) -> rusqlite::Result<Fait> {
        let Some(id) = numero(v) else { return Ok(Fait::default()) };
        if !profil::supprimer_style(base.connexion(), id)? {
            return Ok(Fait { change: true, ..Fait::default() });
        }
        if self.style_ouvert == Some(id) {
            self.style_ouvert = None;
        }
        Ok(Fait {
            message: Some("Texte supprimé.".into()),
            journal: Some(format!("Profil : texte {id} supprimé")),
            change: true,
            ..Fait::default()
        })
    }

    /// Une clé donnée ou effacée : chaque partie vérifiée, puis toutes gardées ensemble (chiffrées), ou aucune.
    fn cle(&mut self, ctx: &mut Contexte, id: &str, v: &Value) -> Fait {
        let Some(service) = Service::depuis_code(&texte(v, "service")) else { return Fait::default() };
        let Some(cles) = ctx.cles.as_deref_mut() else {
            return Fait::refus(CLE_NON_GARDEE, "Clés : coffre absent".into());
        };
        let nom = match service {
            Service::Claude => "Claude",
            // (la boîte Gmail a sa rubrique à elle : crate::mail)
            Service::Gmail | Service::Sites => return Fait::default(),
        };
        if id == CLE_EFFACER {
            return match cles.effacer(service.parties()) {
                Ok(true) => Fait {
                    message: Some(format!("Clé {nom} effacée.")),
                    ordre: Some(json!({ "kit": "accueil", "cle": service.code() })),
                    journal: Some(format!("Clés : clé {} effacée", service.code())),
                    change: true,
                    ..Fait::default()
                },
                Ok(false) => Fait { change: true, ..Fait::default() },
                Err(e) => Fait::refus(CLE_NON_GARDEE, format!("Clés : {} non effacée ({e})", service.code())),
            };
        }
        let valeurs = v.get("valeurs").cloned().unwrap_or(Value::Null);
        let mut parties: Vec<(Cle, String)> = Vec::new();
        for c in service.parties() {
            match verifier_cle(*c, &texte(&valeurs, c.code())) {
                Ok(x) => parties.push((*c, x)),
                Err(r) => {
                    return Fait {
                        message: Some(r),
                        journal: Some(format!("Clés : {} refusée (forme)", c.code())),
                        ..Fait::default()
                    };
                }
            }
        }
        let l: Vec<(Cle, &str)> = parties.iter().map(|(c, x)| (*c, x.as_str())).collect();
        match cles.garder_plusieurs(&l) {
            Ok(()) => Fait {
                message: Some(format!("Clé {nom} enregistrée. Elle est chiffrée sur ce PC.")),
                bravo: true,
                ordre: Some(json!({ "kit": "accueil", "cle": service.code() })),
                journal: Some(format!("Clés : clé {} enregistrée", service.code())),
                change: true,
                ..Fait::default()
            },
            Err(e) => Fait::refus(CLE_NON_GARDEE, format!("Clés : {} non gardée ({e})", service.code())),
        }
    }
}

/// « Ce que je cherche », les critères communs (0.1.8 : les postes et les lieux sont dans le tableau des recherches,
/// offres.rs) : vérifiés puis enregistrés.
fn recherche(base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
    let contrats: Vec<String> = v
        .get("contrats")
        .and_then(Value::as_array)
        .map(|l| l.iter().take(20).filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();
    let s = SaisieRecherche {
        postes: texte(v, "postes"),
        lieu: texte(v, "lieu"),
        ville: texte(v, "ville"),
        rayon: v.get("rayon").and_then(Value::as_i64).unwrap_or(0),
        departement: texte(v, "departement"),
        region: texte(v, "region"),
        contrats,
        teletravail: texte(v, "teletravail"),
        sans_diplome: v.get("sans_diplome").and_then(Value::as_bool).unwrap_or(false),
        debutant: v.get("debutant").and_then(Value::as_bool).unwrap_or(false),
        salaire: texte(v, "salaire"),
    };
    let r = match verifier_criteres(&s) {
        Ok(r) => r,
        Err(m) => {
            return Ok(Fait {
                message: Some(m),
                journal: Some("Profil : recherche refusée".into()),
                ..Fait::default()
            });
        }
    };
    profil::enregistrer_criteres(base.connexion(), &r, maintenant)?;
    Ok(Fait {
        message: Some("C'est enregistré. Jobmind s'en servira pour trier les offres.".into()),
        bravo: true,
        ordre: Some(json!({ "kit": "accueil", "recherche": "enregistree" })),
        journal: Some("Profil : critères enregistrés".into()),
        change: true,
        ..Fait::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jobmind_storage::cles::{Coffre, EtatCle};

    fn m(h: u32) -> Moment {
        Moment::nouveau(2026, 11, 18, h, 0, 0).unwrap()
    }

    fn essai(nom: &str) -> std::path::PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-{nom}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Un faux coffre (le vrai, DPAPI, est essayé dans windows\dpapi.rs).
    struct Faux;
    impl Coffre for Faux {
        fn fermer(&self, c: &[u8]) -> Result<Vec<u8>, String> {
            Ok(c.iter().rev().map(|b| b ^ 0x33).collect())
        }
        fn ouvrir(&self, c: &[u8]) -> Result<Vec<u8>, String> {
            Ok(c.iter().rev().map(|b| b ^ 0x33).collect())
        }
    }

    /// Un PDF et un .docx d'exemple, fabriqués ici (jamais pris sur le PC).
    fn pdf(lignes: &[&str]) -> Vec<u8> {
        let mut contenu = String::from("BT /F1 12 Tf 16 TL 72 770 Td\n");
        for l in lignes {
            let s: String =
                l.chars().map(|c| if c.is_ascii() { c.to_string() } else { format!("\\{:03o}", c as u32) }).collect();
            contenu.push_str(&format!("({s}) Tj T*\n"));
        }
        contenu.push_str("ET\n");
        let objets = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_string(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
            format!("<< /Length {} >>\nstream\n{contenu}\nendstream", contenu.len() + 1),
        ];
        let mut p = b"%PDF-1.4\n".to_vec();
        let mut places = Vec::new();
        for (i, o) in objets.iter().enumerate() {
            places.push(p.len());
            p.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
        }
        let x = p.len();
        p.extend(b"xref\n0 6\n0000000000 65535 f \n");
        for pl in places {
            p.extend(format!("{pl:010} 00000 n \n").as_bytes());
        }
        p.extend(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n").as_bytes());
        p
    }

    fn import(jeton: u64, nom: &str, octets: Vec<u8>) -> Import {
        let lu = jobmind_lecture::lire(&octets);
        Import { jeton, nom: nom.into(), octets, lu }
    }

    /// Le scénario de l'écran : importer un PDF (la boîte, puis le fichier lu), corriger son texte, coller un second
    /// CV, en faire le principal, supprimer le premier (son fichier part avec lui).
    #[test]
    fn scenario_import_correction_principal() {
        let d = essai("profil-import");
        let (b, _) = Base::en_memoire(m(9)).unwrap();
        let mut p = Profil::default();
        let mut ctx = Contexte { base: Some(&b), dossier: &d, cles: None };
        // « Importer un fichier » : la boîte est demandée, un seul import à la fois
        let f = p.geste(&mut ctx, IMPORTER, &Value::Null, m(9));
        assert!(f.importer && f.change);
        let jeton = p.import.as_ref().unwrap().jeton;
        let f = p.geste(&mut ctx, IMPORTER, &Value::Null, m(9));
        assert!(!f.importer);
        assert_eq!(f.message.as_deref(), Some("Un fichier est déjà en train d'être importé."));
        let (v, _) = p.vue(Some(&b), None);
        assert_eq!(v.import, Some(None));
        // Le fichier choisi, puis lu
        assert!(p.import_choisi(jeton, "CV Jordan 2026.pdf"));
        assert_eq!(p.vue(Some(&b), None).0.import, Some(Some("CV Jordan 2026.pdf".into())));
        let octets = pdf(&["Jordan Exemple", "Cariste, CACES 1, 3 et 5", "Préparateur de commandes à Woippy"]);
        let f = p.import_fini(&mut ctx, import(jeton, "CV Jordan 2026.pdf", octets.clone()), m(10));
        assert_eq!(f.message.as_deref(), Some("CV importé. Relis son texte et corrige ce qui a été mal lu."));
        assert!(f.journal.as_deref().unwrap().starts_with("Profil : CV 1 importé (PDF, 1 Ko, "), "{f:?}");
        assert!(p.import.is_none());
        let (v, _) = p.vue(Some(&b), None);
        assert_eq!(v.cvs.len(), 1);
        let cv = &v.cvs[0];
        assert_eq!((cv.nom.as_str(), cv.genre, cv.principal), ("CV Jordan 2026", GenreCv::Pdf, true));
        assert!(cv.texte.contains("Préparateur de commandes à Woippy"), "{}", cv.texte);
        // Le fichier est copié dans le dossier cv des données, tel quel
        let copie = profil::chemin_fichier(&d, cv.fichier.as_deref().unwrap()).unwrap();
        assert_eq!(std::fs::read(&copie).unwrap(), octets);
        // Corrigé
        let f = p.geste(
            &mut ctx,
            CV_ENREGISTRER,
            &json!({ "id": cv.id, "nom": "CV 2026", "texte": "Jordan Exemple\nCariste depuis 2023" }),
            m(11),
        );
        assert_eq!(f.message.as_deref(), Some("CV enregistré."));
        // Collé : un second CV
        let f = p.geste(
            &mut ctx,
            CV_ENREGISTRER,
            &json!({ "nom": "CV vente", "texte": "Jordan Exemple, vendeur à Metz pendant un an" }),
            m(12),
        );
        assert_eq!(f.ordre, Some(json!({ "kit": "accueil", "cv": 2 })));
        let f = p.geste(&mut ctx, CV_PRINCIPAL, &json!({ "id": 2 }), m(12));
        assert_eq!(f.message.as_deref(), Some("C'est maintenant ton CV principal."));
        // Le premier supprimé : son fichier part avec lui
        let f = p.geste(&mut ctx, CV_SUPPRIMER, &json!({ "id": 1 }), m(13));
        assert_eq!(f.message.as_deref(), Some("CV supprimé."));
        assert!(!copie.exists());
        let (v, _) = p.vue(Some(&b), None);
        assert_eq!(v.cvs.iter().map(|c| (c.id, c.principal)).collect::<Vec<_>>(), [(2, true)]);
        assert_eq!(v.cv_ouvert, Some(2));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Les fichiers refusés : un PDF scanné est gardé (son texte se colle à la main), un fichier abîmé ou trop gros
    /// ne l'est pas ; un résultat d'un import abandonné est laissé ; la boîte fermée sans choix ne dit rien.
    #[test]
    fn imports_refuses_ou_annules() {
        let d = essai("profil-refus");
        let (b, _) = Base::en_memoire(m(9)).unwrap();
        let mut p = Profil::default();
        let mut ctx = Contexte { base: Some(&b), dossier: &d, cles: None };
        // Annulé
        p.geste(&mut ctx, IMPORTER, &Value::Null, m(9));
        let j = p.import.as_ref().unwrap().jeton;
        let f = p.import_annule(j, None);
        assert!(f.message.is_none() && f.change && p.import.is_none());
        // Abîmé : rien de gardé
        p.geste(&mut ctx, IMPORTER, &Value::Null, m(9));
        let j = p.import.as_ref().unwrap().jeton;
        let mut abime = pdf(&["Jordan Exemple, cariste"]);
        abime.truncate(60);
        let f = p.import_fini(&mut ctx, import(j, "cv.pdf", abime), m(9));
        assert!(f.message.unwrap().contains("abîmé"));
        assert_eq!(profil::nombre_cv(b.connexion()).unwrap(), 0);
        // Scanné (aucun texte) : le fichier est gardé, le texte est vide, le message dit de coller
        p.geste(&mut ctx, IMPORTER, &Value::Null, m(9));
        let j = p.import.as_ref().unwrap().jeton;
        let vide = pdf(&[]);
        let f = p.import_fini(&mut ctx, import(j, "scan.pdf", vide), m(9));
        let msg = f.message.unwrap();
        assert!(msg.contains("Colle le texte") && msg.ends_with("Le fichier est gardé."), "{msg}");
        let (v, _) = p.vue(Some(&b), None);
        assert_eq!(v.cvs.len(), 1);
        assert!(v.cvs[0].texte.is_empty() && v.cvs[0].fichier.is_some());
        // Un résultat qui arrive après l'abandon : laissé
        p.geste(&mut ctx, IMPORTER, &Value::Null, m(9));
        let j = p.import.as_ref().unwrap().jeton;
        p.import_trop_long();
        let f = p.import_fini(&mut ctx, import(j, "cv.pdf", pdf(&["Jordan Exemple, cariste depuis 2023"])), m(9));
        assert!(f.message.is_none());
        assert_eq!(profil::nombre_cv(b.connexion()).unwrap(), 1);
        // Trop gros : refusé avant d'être lu
        let gros = d.join("gros.pdf");
        let fichier = std::fs::File::create(&gros).unwrap();
        fichier.set_len(TAILLE_MAX + 1).unwrap();
        drop(fichier);
        let i = lire_fichier(&gros, 7);
        assert_eq!((i.lu.unwrap_err(), i.octets.len(), i.nom.as_str()), (Erreur::TropGros, 0, "gros.pdf"));
        let i = lire_fichier(&d.join("absent.pdf"), 7);
        assert!(matches!(i.lu, Err(Erreur::Abime(_))));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn recherche_style_et_cles() {
        let d = essai("profil-cles");
        let (b, _) = Base::en_memoire(m(9)).unwrap();
        let (mut cles, _) = Cles::ouvrir(&d, Box::new(Faux));
        let mut p = Profil::default();
        let mut ctx = Contexte { base: Some(&b), dossier: &d, cles: Some(&mut cles) };
        // Ce que je cherche
        let saisie = json!({
            "postes": "Agent de quai, cariste", "lieu": "departement", "ville": "", "rayon": 30, "departement": "57",
            "region": "", "contrats": ["cdi", "interim"], "teletravail": "peu_importe", "sans_diplome": true,
            "debutant": true, "salaire": "",
        });
        let f = p.geste(&mut ctx, RECHERCHE, &saisie, m(9));
        assert!(f.bravo, "{f:?}");
        let mut faux = saisie.clone();
        // (0.1.8 : seuls les critères sont gardés ici ; les postes et le lieu envoyés ne sont plus regardés)
        assert_eq!(profil::lire_recherche(b.connexion()).unwrap(), None);
        assert_eq!(profil::lire_criteres(b.connexion()).unwrap().contrats.len(), 2);
        faux["salaire"] = json!("beaucoup");
        assert_eq!(
            p.geste(&mut ctx, RECHERCHE, &faux, m(9)).message.as_deref(),
            Some("Le salaire s'écrit en chiffres, par exemple 1 800.")
        );
        // Le style : ajouté, ouvert, supprimé
        let f = p.geste(&mut ctx, STYLE_AJOUTER, &json!({ "titre": "", "texte": "Bonjour, je vous écris pour la place au magasin. J'ai fait un an de vente à Metz." }), m(9));
        assert_eq!(f.message.as_deref(), Some("Texte ajouté."));
        assert!(
            p.geste(&mut ctx, STYLE_AJOUTER, &json!({ "texte": "Trop court" }), m(9))
                .message
                .unwrap()
                .contains("très court")
        );
        p.geste(&mut ctx, STYLE_OUVRIR, &json!({ "id": 1 }), m(9));
        assert_eq!(p.style_ouvert, Some(1));
        p.geste(&mut ctx, STYLE_SUPPRIMER, &json!({ "id": 1 }), m(9));
        assert_eq!(p.style_ouvert, None);
        // Les clés : Claude, refusée si elle est fausse (rien gardé), puis gardée ; Adzuna (retiré) : rien
        let mauvaise = json!({ "service": "claude", "valeurs": { "claude": "sk-ant-court" } });
        let f = p.geste(&mut ctx, CLE_GARDER, &mauvaise, m(9));
        assert!(f.message.unwrap().contains("incomplète"));
        assert!(!f.journal.unwrap().contains("court"));
        let retiree = json!({ "service": "adzuna", "valeurs": { "adzuna_id": "1a2b3c4d", "adzuna_cle": "0123456789abcdef0123456789abcdef" } });
        assert_eq!(p.geste(&mut ctx, CLE_GARDER, &retiree, m(9)), Fait::default());
        let f = p.geste(&mut ctx, CLE_GARDER, &json!({ "service": "claude", "valeurs": { "claude": "sk-ant-api03-InventeeInventeeInventeeInventee_cdef" } }), m(9));
        assert!(f.bravo);
        assert_eq!(f.message.as_deref(), Some("Clé Claude enregistrée. Elle est chiffrée sur ce PC."));
        assert_eq!(f.journal.as_deref(), Some("Clés : clé claude enregistrée"));
        let (v, _) = p.vue(Some(&b), ctx.cles.as_deref());
        assert_eq!(
            v.cles.iter().find(|(c, _)| *c == Cle::Claude).unwrap().1,
            Some(EtatCle::Gardee("se termine par cdef".into()))
        );
        assert!(v.cles.iter().all(|(c, _)| !c.code().contains("adzuna")), "{:?}", v.cles);
        // L'état de la page ne contient aucune clé
        let e = crate::ecran::etat("0.3.0", &crate::ecran::Vue { profil: &v, ..crate::ecran::Vue::vide(m(9).jour()) })
            .to_string();
        for cle in ["0123456789abcdef0123456789abcdef", "1a2b3c4d", "InventeeInventee"] {
            assert!(!e.contains(cle), "clé dans l'état : {cle}");
        }
        assert!(e.contains("se termine par cdef") && e.contains("Ta clé Claude est là"));
        assert_eq!(p.geste(&mut ctx, CLE_EFFACER, &json!({ "service": "adzuna" }), m(9)), Fait::default());
        let f = p.geste(&mut ctx, CLE_EFFACER, &json!({ "service": "claude" }), m(9));
        assert_eq!(f.message.as_deref(), Some("Clé Claude effacée."));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// 0.2.0 : « Ce que Jobmind a compris de ton CV » : la fiche du CV principal, les lignes retirées et ajoutées
    /// gardées dans la base (elles survivent à la fermeture), la fiche refaite quand le CV change, plusieurs CV (le
    /// principal sert), plus aucun CV (plus de fiche). Le journal ne reçoit jamais une ligne de la fiche.
    #[test]
    fn parcours_lu_corrige_et_garde() {
        use jobmind_core::parcours::Liste;
        let d = essai("profil-parcours");
        let (b, _) = Base::en_memoire(m(9)).unwrap();
        let mut p = Profil::default();
        let mut ctx = Contexte { base: Some(&b), dossier: &d, cles: None };
        // Sans CV : pas de fiche, et rien ne s'ajoute
        assert_eq!(p.vue(Some(&b), None).0.parcours, None);
        let f = p.geste(&mut ctx, PC_AJOUTER, &json!({ "liste": "permis", "texte": "Permis B" }), m(9));
        assert_eq!(f.message.as_deref(), Some("Ajoute d'abord ton CV : Jobmind y lira ton parcours."));
        // Un CV collé : sa fiche
        let cv = "Lucien Tartempion\nEXPERIENCES\n2019 - 2022 : Cariste chez Logi-Fiction\n2015 : Manutentionnaire en intérim\n\
                  FORMATION\nCAP Agent d'entreposage\nPermis B, CACES 3";
        p.geste(&mut ctx, CV_ENREGISTRER, &json!({ "nom": "CV logistique", "texte": cv }), m(9));
        let noms = |v: &VueProfil| -> Vec<String> {
            v.parcours.as_ref().unwrap().metiers.iter().map(|x| x.nom.clone()).collect()
        };
        let (v, _) = p.vue(Some(&b), None);
        assert_eq!(noms(&v), ["Cariste", "Manutentionnaire"]);
        assert_eq!(v.parcours.as_ref().unwrap().permis, ["Permis B", "CACES 3"]);
        // Une ligne retirée, des lignes ajoutées ; le journal dit la liste, jamais la ligne
        let f = p.geste(&mut ctx, PC_RETIRER, &json!({ "liste": "metiers", "cle": "manutentionnaire" }), m(10));
        assert_eq!(f.journal.as_deref(), Some("Profil : parcours, une ligne retirée (metiers)"));
        assert!(f.change && f.message.is_none());
        let f = p.geste(&mut ctx, PC_AJOUTER, &json!({ "liste": "metiers", "texte": "magasinier, 2 ans" }), m(10));
        assert_eq!(f.journal.as_deref(), Some("Profil : parcours, une ligne ajoutée (metiers)"));
        assert_eq!(f.ordre, Some(json!({ "kit": "accueil", "parcours": "metiers" })));
        p.geste(&mut ctx, PC_AJOUTER, &json!({ "liste": "permis", "texte": "ce" }), m(10));
        p.geste(&mut ctx, PC_AJOUTER, &json!({ "liste": "savoirs", "texte": "Inventaire" }), m(10));
        p.geste(&mut ctx, PC_AJOUTER, &json!({ "liste": "diplomes", "texte": "Bac pro Logistique" }), m(10));
        let (v, _) = p.vue(Some(&b), None);
        let fiche = v.parcours.clone().unwrap();
        assert_eq!(noms(&v), ["Cariste", "Magasinier"]);
        assert_eq!(fiche.metiers[1].mois, Some(24));
        assert_eq!(fiche.permis, ["Permis B", "CACES 3", "Permis CE"]);
        assert_eq!(fiche.competences, ["Inventaire"]);
        assert_eq!(fiche.diplomes, ["CAP Agent d'entreposage", "Bac pro Logistique"]);
        // Des saisies refusées : une phrase simple, rien de gardé
        for (liste, texte) in [("metiers", " "), ("savoirs", "x"), ("permis", ""), ("diplomes", &"a".repeat(80))] {
            let f = p.geste(&mut ctx, PC_AJOUTER, &json!({ "liste": liste, "texte": texte }), m(10));
            assert!(f.message.is_some() && f.ordre.is_none() && !f.change, "{liste} {texte:?} : {f:?}");
        }
        for v in [json!({ "liste": "autre", "texte": "Cariste" }), json!(null), json!({ "liste": "permis" })] {
            assert!(p.geste(&mut ctx, PC_AJOUTER, &v, m(10)).ordre.is_none());
            assert!(p.geste(&mut ctx, PC_RETIRER, &v, m(10)).journal.is_none());
        }
        assert_eq!(p.vue(Some(&b), None).0.parcours, Some(fiche.clone()));
        // L'appli fermée puis rouverte (un profil neuf sur la même base) : la même fiche corrigée
        let mut p = Profil::default();
        assert_eq!(p.vue(Some(&b), None).0.parcours, Some(fiche.clone()));
        // Le texte du CV corrigé : la fiche est refaite, les corrections tiennent (la ligne retirée ne revient pas,
        // les lignes ajoutées restent), ce qui est nouveau dans le CV arrive
        let cv2 = cv.replace("\nFORMATION", "\n2022 - 2024 : Agent de quai chez Trans-Fiction\nFORMATION");
        p.geste(&mut ctx, CV_ENREGISTRER, &json!({ "id": 1, "nom": "CV logistique", "texte": cv2 }), m(11));
        let (v, _) = p.vue(Some(&b), None);
        assert_eq!(noms(&v), ["Cariste", "Agent de quai", "Magasinier"]);
        assert_eq!(v.parcours.as_ref().unwrap().permis, ["Permis B", "CACES 3", "Permis CE"]);
        // Une ligne ajoutée à la main puis retirée : il n'en reste rien ; une ligne retirée puis ajoutée : elle revient
        p.geste(&mut ctx, PC_RETIRER, &json!({ "liste": "permis", "cle": "permis ce" }), m(11));
        p.geste(&mut ctx, PC_AJOUTER, &json!({ "liste": "metiers", "texte": "Manutentionnaire" }), m(11));
        let (v, _) = p.vue(Some(&b), None);
        assert_eq!(v.parcours.as_ref().unwrap().permis, ["Permis B", "CACES 3"]);
        assert_eq!(noms(&v), ["Cariste", "Manutentionnaire", "Agent de quai", "Magasinier"]);
        assert_eq!(base_parcours::nombre_ajouts(b.connexion(), Liste::Permis).unwrap(), 0);
        // Plusieurs CV : c'est le principal qui sert ; les lignes ajoutées suivent la personne
        p.geste(
            &mut ctx,
            CV_ENREGISTRER,
            &json!({ "nom": "CV vente", "texte": "Lucien Tartempion\nExpérience\n2010 - 2014 : Vendeur en boulangerie\nPermis B" }),
            m(12),
        );
        assert_eq!(noms(&p.vue(Some(&b), None).0), ["Cariste", "Manutentionnaire", "Agent de quai", "Magasinier"]);
        p.geste(&mut ctx, CV_PRINCIPAL, &json!({ "id": 2 }), m(12));
        let (v, _) = p.vue(Some(&b), None);
        assert_eq!(noms(&v), ["Vendeur en boulangerie", "Manutentionnaire", "Magasinier"]);
        assert_eq!(v.parcours.as_ref().unwrap().permis, ["Permis B"]);
        // Ce que la page reçoit : le bloc, ses quatre listes, la durée en mots, la clé de chaque ligne
        let r = crate::vue_profil::rubriques(&v, m(12).jour());
        let bloc = &r[0]["dossier"]["parcours"];
        assert_eq!(bloc["titre"], "Ce que Jobmind a compris de ton CV");
        assert_eq!(bloc["note"], "D'après ton CV principal, « CV vente ».");
        let ids: Vec<&str> = bloc["listes"].as_array().unwrap().iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["metiers", "savoirs", "diplomes", "permis"]);
        assert_eq!(
            bloc["listes"][0]["lignes"][0],
            json!({ "cle": "vendeur boulangerie", "nom": "Vendeur en boulangerie", "sous": "4 ans" })
        );
        // Le principal supprimé : l'autre CV redevient principal, sa fiche revient avec les corrections
        p.geste(&mut ctx, CV_SUPPRIMER, &json!({ "id": 2 }), m(13));
        assert_eq!(noms(&p.vue(Some(&b), None).0), ["Cariste", "Manutentionnaire", "Agent de quai", "Magasinier"]);
        // Plus aucun CV : plus de fiche (les corrections restent dans la base pour le prochain CV)
        p.geste(&mut ctx, CV_SUPPRIMER, &json!({ "id": 1 }), m(13));
        let (v, _) = p.vue(Some(&b), None);
        assert_eq!(v.parcours, None);
        assert_eq!(crate::vue_profil::rubriques(&v, m(13).jour())[0]["dossier"]["parcours"], Value::Null);
        p.geste(
            &mut ctx,
            CV_ENREGISTRER,
            &json!({ "nom": "Nouveau CV", "texte": "Lucien Tartempion, cariste depuis 2024" }),
            m(14),
        );
        assert_eq!(noms(&p.vue(Some(&b), None).0), ["Cariste", "Manutentionnaire", "Magasinier"]);
        // Un CV sans texte (un PDF scanné) ou presque vide : une fiche vide, que la personne remplit à la main
        let dernier = p.vue(Some(&b), None).0.cvs[0].id;
        p.geste(&mut ctx, CV_SUPPRIMER, &json!({ "id": dernier }), m(15));
        b.connexion().execute("DELETE FROM parcours_corrections", []).unwrap();
        p.geste(
            &mut ctx,
            CV_ENREGISTRER,
            &json!({ "nom": "CV court", "texte": "Lucien Tartempion, 06 00 00 00 00, bonjour" }),
            m(15),
        );
        let (v, _) = p.vue(Some(&b), None);
        assert!(v.parcours.as_ref().unwrap().vide());
        let bloc = &crate::vue_profil::rubriques(&v, m(15).jour())[0]["dossier"]["parcours"];
        assert!(bloc["vide"].as_str().unwrap().starts_with("Jobmind n'a rien réussi à lire dans ton CV."));
        p.geste(&mut ctx, PC_AJOUTER, &json!({ "liste": "metiers", "texte": "Cariste" }), m(15));
        assert!(p.vue(Some(&b), None).0.parcours.unwrap().utilisable());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Contre-épreuve : des gestes mal formés ne changent rien et ne plantent pas.
    #[test]
    fn gestes_mal_formes() {
        let d = essai("profil-mal");
        let (b, _) = Base::en_memoire(m(9)).unwrap();
        let mut p = Profil::default();
        let mut ctx = Contexte { base: Some(&b), dossier: &d, cles: None };
        for (id, v) in [
            (CV_ENREGISTRER, json!({ "nom": "", "texte": "x" })),
            (CV_ENREGISTRER, json!({ "id": 99, "nom": "CV", "texte": "Jordan Exemple, cariste à Woippy" })),
            (CV_ENREGISTRER, json!([1, 2])),
            (CV_PRINCIPAL, json!({ "id": "1" })),
            (CV_SUPPRIMER, json!(null)),
            (RECHERCHE, json!("rien")),
            (STYLE_AJOUTER, json!({ "texte": 3 })),
            (CLE_GARDER, json!({ "service": "google", "valeurs": {} })),
            (
                CLE_GARDER,
                // (une source retirée : Jooble, depuis une ancienne page ; rien n'est gardé)
                json!({ "service": "jooble", "valeurs": { "jooble": "aaaabbbb-cccc-dddd-eeee-ffff00001111" } }),
            ),
        ] {
            let f = p.geste(&mut ctx, id, &v, m(9));
            assert!(!f.bravo && f.ordre.is_none() && !f.importer, "{id} {v}");
        }
        assert_eq!(profil::nombre_cv(b.connexion()).unwrap(), 0);
        assert!(est_geste(IMPORTER) && est_geste(COPIE) && !est_geste("c-ouvrir"));
        assert!(est_geste(PC_AJOUTER) && est_geste(PC_RETIRER));
        // Sans base : la phrase de la suite
        let mut ctx = Contexte { base: None, dossier: &d, cles: None };
        assert_eq!(p.geste(&mut ctx, IMPORTER, &Value::Null, m(9)).message.as_deref(), Some(crate::suivi::SANS_BASE));
        let _ = std::fs::remove_dir_all(&d);
    }
}
