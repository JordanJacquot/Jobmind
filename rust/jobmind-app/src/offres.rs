//! Les gestes de l'onglet Offres (étape 4 du plan) : ouvrir une offre, favori, « plus tard », « Pas pour moi », les
//! filtres, les recherches gardées (tirées de « Ce que je cherche » ou tapées), « Chercher maintenant », « J'ai
//! postulé », « Voir sur le site » (étape 5 bis : la petite fenêtre du site, crate::coulisse) ; et dans Mes clés, « Essayer
//! la clé ». Chaque saisie est vérifiée ici par jobmind-core (jamais confiance dans la page), puis écrite dans la base.
//!
//! « Chercher maintenant » ne bloque jamais la fenêtre : le geste prépare la tâche ([`Tache`], sans aucune clé) ; le
//! programme lit les clés au dernier moment et la fait tourner sur un fil à part (une demande à chaque source, jamais
//! de boucle) ; le résultat revient ici ([`Offres::fini`]), qui garde chaque offre (doublons rattachés) et dit ce qui a
//! été trouvé. Une recherche a son numéro (`jeton`) : un résultat arrivé trop tard est laissé.
//!
//! Jobmind reste très en dessous des limites des sources : une recherche à la fois, pas deux fois la même dans la
//! minute, et au plus [`LIMITE_ADZUNA`] demandes par jour à Adzuna, [`LIMITE_JOOBLE`] à Jooble (dont la limite gratuite
//! est de 100 par jour).
//!
//! Le journal ne reçoit jamais une clé ni une adresse entière : seulement ce qui a été fait, et l'hôte d'un lien.
//!
//! Étape 6 : avant de montrer la liste, chaque offre pas encore lue est lue par les règles, et chaque note faite avec
//! un autre « Ce que je cherche » ou un autre CV est refaite (jobmind-storage, `lecture`) ; les filtres « Aucun
//! diplôme requis » et « Débutant accepté », le tri (les plus pertinentes ou les plus récentes), la rubrique « Hors
//! sujet » et « Pas hors sujet ». Les offres floues (les règles n'ont pas trouvé le diplôme ou l'expérience) sont
//! données au petit modèle de Claude, par lots de [`LOT_IA`], après une recherche ou sur un clic, jamais plus de
//! [`PLAFOND_IA`] par jour, une seule fois chacune ; sans clé Claude, seules les règles servent, et c'est dit.

use std::collections::HashSet;
use std::sync::LazyLock;

use jobmind_core::candidature::{
    ENTREPRISE_MAX, LIEN_MAX, POSTE_MAX, SOURCE_MAX, Saisie, VILLE_MAX, verifier as verifier_candidature,
};
use jobmind_core::cle::{Cle, Service};
use jobmind_core::offre::{
    self, Appui, FiltreLieu, Filtres, Offre, SaisieFiltres, centre_ville, cle_ville, garder, lien_ouvrable,
    trier_selon, verifier_filtres,
};
use jobmind_core::pertinence::{self, OffreANoter};
use jobmind_core::recherche::Lieu;
use jobmind_core::recherches::{self, NOMBRE_MAX, SaisieRechercheGardee, depuis_profil};
use jobmind_llm::{ErreurLlm, LlmProvider, Modele};
use jobmind_sources::{ErreurSource, OffreTrouvee, Recherche};
use jobmind_storage::cles::{Cles, EtatCle};
use jobmind_storage::lecture::{self as base_lecture, ResultatIa};
use jobmind_storage::offres::{self as base_offres, Bilan, Marque, OffreComplete, RechercheGardee};
use jobmind_storage::rusqlite;
use jobmind_storage::{Base, Moment, candidatures, profil};
use serde_json::{Value, json};

use crate::suivi::{Fait, PAS_ENREGISTRE};

/// Les ids des gestes de l'onglet Offres (les actions de la page).
pub const OUVRIR: &str = "o-ouvrir";
pub const FAVORI: &str = "o-favori";
pub const PLUS_TARD: &str = "o-plus-tard";
pub const CACHER: &str = "o-cacher";
pub const FILTRES: &str = "o-filtres";
pub const CHERCHER: &str = "o-chercher";
pub const CHOISIR: &str = "o-choisir";
pub const RECHERCHE: &str = "o-recherche";
pub const RECHERCHE_SUPPRIMER: &str = "o-recherche-supprimer";
pub const DEPUIS_PROFIL: &str = "o-depuis-profil";
pub const POSTULE: &str = "o-postule";
pub const VOIR: &str = "o-voir";
pub const ADZUNA: &str = "o-adzuna";
/// Étape 6 : « Pas hors sujet » (et son contraire), « Lire avec Claude »
pub const PAS_HORS_SUJET: &str = "o-pas-hors-sujet";
pub const LIRE_IA: &str = "o-lire-ia";
/// Mes clés : « Essayer la clé »
pub const ESSAYER: &str = "cle-essayer";
const GESTES: [&str; 16] = [
    PAS_HORS_SUJET,
    LIRE_IA,
    OUVRIR,
    FAVORI,
    PLUS_TARD,
    CACHER,
    FILTRES,
    CHERCHER,
    CHOISIR,
    RECHERCHE,
    RECHERCHE_SUPPRIMER,
    DEPUIS_PROFIL,
    POSTULE,
    VOIR,
    ADZUNA,
    ESSAYER,
];

/// Un geste de l'onglet Offres (ou « Essayer la clé »).
pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Les demandes par jour que Jobmind s'autorise, par source.
pub const LIMITE_ADZUNA: u32 = 100;
pub const LIMITE_JOOBLE: u32 = 50;
/// Une même recherche ne repart pas avant ce nombre de secondes.
pub const PAUSE_SECONDES: i64 = 60;
/// La liste montre au plus ce nombre d'offres (les filtres resserrent).
pub const LISTE_MAX: usize = 150;
/// Étape 6 : Claude lit au plus ce nombre d'offres floues à la fois...
pub const LOT_IA: usize = 8;
/// ... et au plus ce nombre par jour.
pub const PLAFOND_IA: u32 = 150;

/// Une offre à faire lire par Claude : son numéro, son titre, son texte.
pub type OffreIa = (i64, String, String);

/// Ce que le fil de Claude doit lire : des offres (leur numéro, leur titre, leur texte). Aucune clé ici.
#[derive(Debug, Clone, PartialEq)]
pub struct TacheIa {
    pub jeton: u64,
    pub offres: Vec<OffreIa>,
}

/// Ce que le fil de Claude rapporte : chaque offre lue (déjà vérifiée), et l'erreur qui a arrêté le lot.
#[derive(Debug, Clone)]
pub struct RetourIa {
    pub jeton: u64,
    pub resultats: Vec<ResultatIa>,
    pub erreur: Option<ErreurLlm>,
}

/// Le travail du fil de Claude : chaque offre demandée l'une après l'autre, la réponse vérifiée (une citation absente
/// de l'offre n'est jamais gardée). Une erreur de clé, de réseau, de délai ou de surcharge arrête le lot ; une réponse
/// illisible est notée (pas de deuxième appel pour la même offre).
pub fn lire_lot(ia: &dyn LlmProvider, t: &TacheIa) -> RetourIa {
    let mut resultats = Vec::new();
    let mut erreur = None;
    for (id, titre, texte) in &t.offres {
        let d = jobmind_llm::lecture::demande(titre, texte);
        let base = ResultatIa {
            offre_id: *id,
            modele: Modele::Petit.id().into(),
            etat: "illisible",
            jetons_entree: 0,
            jetons_sortie: 0,
            cout_millioniemes: 0,
            reponse: String::new(),
            diplome: None,
            experience: None,
        };
        match ia.repondre(&d) {
            Ok(r) => {
                let cout = Modele::Petit.cout_millioniemes(r.jetons_entree, r.jetons_sortie);
                let mut x = ResultatIa {
                    jetons_entree: r.jetons_entree,
                    jetons_sortie: r.jetons_sortie,
                    cout_millioniemes: cout,
                    reponse: r.texte.clone(),
                    ..base
                };
                if let Ok(lu) = jobmind_llm::lecture::comprendre(&d.texte, &r.texte) {
                    x.etat = if lu.citation_inventee { "citation_inventee" } else { "ok" };
                    x.diplome = lu.diplome;
                    x.experience = lu.experience;
                }
                resultats.push(x);
            }
            Err(e) if !e.arrete_le_lot() => resultats.push(ResultatIa { reponse: e.code().into(), ..base }),
            Err(e) => {
                erreur = Some(e);
                break;
            }
        }
    }
    RetourIa { jeton: t.jeton, resultats, erreur }
}

/// Un coût en millionièmes de dollar, en mots : « moins d'un centime », « environ 3 centimes ».
pub fn cout_texte(millioniemes: u64) -> String {
    match millioniemes.div_ceil(10_000) {
        0 | 1 if millioniemes < 10_000 => "moins d'un centime".into(),
        1 => "environ 1 centime".into(),
        n => format!("environ {n} centimes"),
    }
}

/// Le nom d'une source (« Adzuna »).
pub fn nom_source(code: &str) -> &'static str {
    match code {
        "adzuna" => "Adzuna",
        "jooble" => "Jooble",
        // (étape 5 : les sites lus par Jobmind)
        "linkedin" => "LinkedIn",
        "indeed" => "Indeed",
        "wttj" => "Welcome to the Jungle",
        "francetravail" => "France Travail",
        _ => "un autre site",
    }
}

fn limite(s: Service) -> u32 {
    if s == Service::Adzuna { LIMITE_ADZUNA } else { LIMITE_JOOBLE }
}

/// La clé de ce service est entière et lisible.
pub fn cle_prete(cles: Option<&Cles>, s: Service) -> bool {
    cles.is_some_and(|c| s.parties().iter().all(|p| matches!(c.etat(*p), Some(EtatCle::Gardee(_)))))
}

/// Ce que le fil des recherches doit faire. Aucune clé ici : le programme les lit au moment de lancer le fil.
#[derive(Debug, Clone, PartialEq)]
pub enum Tache {
    /// « Chercher maintenant » : cette recherche, chez ces sources
    Chercher { jeton: u64, recherche_id: i64, recherche: Recherche, services: Vec<Service> },
    /// « Essayer la clé » : une toute petite recherche
    Essayer { jeton: u64, service: Service },
}

/// Ce que le fil rapporte : chaque source et sa réponse.
#[derive(Debug, Clone)]
pub struct Retour {
    pub jeton: u64,
    pub resultats: Vec<(Service, Result<Vec<OffreTrouvee>, ErreurSource>)>,
}

/// La phrase sous « Chercher maintenant » : ce qu'a donné la dernière fois, ou ce qui ne va pas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annonce {
    pub texte: String,
    /// « ok », « attention » (une source a répondu, pas l'autre) ou « erreur »
    pub genre: &'static str,
    /// Le souci vient d'une clé : l'écran propose « Ouvrir Mes clés »
    pub cles: bool,
}

/// Une recherche en cours.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnCours {
    pub jeton: u64,
    pub recherche_id: Option<i64>,
    pub texte: String,
}

/// Ce que l'écran montre de l'onglet Offres, lu dans la base.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VueOffres {
    /// Les offres qui passent les filtres (au plus [`LISTE_MAX`])
    pub liste: Vec<Offre>,
    /// Le nombre d'offres qui passent les filtres
    pub total: usize,
    /// Le nombre d'offres gardées (une par groupe de doublons)
    pub gardees: usize,
    pub nouvelles: usize,
    /// Le nombre de favoris, de « plus tard », de cachées
    pub favoris: usize,
    pub plus_tard: usize,
    pub cachees: usize,
    pub ouverte: Option<OffreComplete>,
    pub recherches: Vec<RechercheGardee>,
    pub recherche: Option<i64>,
    pub en_cours: Option<String>,
    pub annonce: Option<Annonce>,
    pub saisie: SaisieFiltres,
    pub filtres_actifs: bool,
    /// Les clés prêtes : Adzuna, Jooble
    pub cles: (bool, bool),
    /// « Ce que je cherche » est enregistré
    pub profil: bool,
    /// Étape 6 : le nombre d'offres rangées hors sujet, et d'offres floues que Claude pourrait lire
    pub hors_sujet: usize,
    pub floues: usize,
    /// La clé Claude est prête
    pub claude: bool,
    /// Claude lit (la phrase), ou ce qu'a donné sa dernière lecture
    pub ia_en_cours: Option<String>,
    pub ia_annonce: Option<Annonce>,
}

/// Rien encore (les tests, une base qui ne s'ouvre pas).
pub static VIDE: LazyLock<VueOffres> = LazyLock::new(|| VueOffres { saisie: saisie_vide(), ..VueOffres::default() });

fn saisie_vide() -> SaisieFiltres {
    SaisieFiltres {
        genre: "partout".into(),
        rayon: 30,
        teletravail: "peu_importe".into(),
        liste: "toutes".into(),
        tri: "pertinence".into(),
        ..SaisieFiltres::default()
    }
}

/// L'onglet Offres : l'offre ouverte, les filtres, la recherche choisie, celle en cours.
#[derive(Debug, Clone)]
pub struct Offres {
    pub ouverte: Option<i64>,
    pub filtres: Filtres,
    pub saisie: SaisieFiltres,
    pub recherche: Option<i64>,
    pub en_cours: Option<EnCours>,
    pub annonce: Option<Annonce>,
    /// Le moment où chaque recherche est partie (pas deux fois dans la minute)
    parties: Vec<(i64, i64)>,
    jetons: u64,
    /// Étape 6 : la lecture par Claude en cours (son jeton, le nombre d'offres), et ce qu'a donné la dernière
    pub ia: Option<(u64, usize)>,
    pub ia_annonce: Option<Annonce>,
}

impl Default for Offres {
    fn default() -> Offres {
        Offres {
            ouverte: None,
            filtres: Filtres::tous(),
            saisie: saisie_vide(),
            recherche: None,
            en_cours: None,
            annonce: None,
            parties: Vec::new(),
            jetons: 0,
            ia: None,
            ia_annonce: None,
        }
    }
}

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

/// Coupe un texte à `n` caractères.
fn coupe(t: &str, n: usize) -> String {
    t.chars().take(n).collect::<String>().trim().to_string()
}

/// « 12 offres », « 1 offre ».
pub fn n_offres(n: usize) -> String {
    format!("{n} offre{}", if n > 1 { "s" } else { "" })
}

/// Ce qu'il faut pour les gestes : la base et les clés (seulement pour savoir lesquelles sont prêtes).
pub struct Contexte<'a> {
    pub base: Option<&'a Base>,
    pub cles: Option<&'a Cles>,
}

impl Offres {
    /// Ce que l'écran montre, d'après la base. Une base illisible : rien, et la raison pour le journal.
    pub fn vue(&mut self, base: Option<&Base>, cles: Option<&Cles>) -> (VueOffres, Option<String>) {
        let mut v = VueOffres {
            saisie: self.saisie.clone(),
            filtres_actifs: self.filtres.actifs(),
            en_cours: self.en_cours.as_ref().map(|e| e.texte.clone()),
            annonce: self.annonce.clone(),
            cles: (cle_prete(cles, Service::Adzuna), cle_prete(cles, Service::Jooble)),
            claude: cle_prete(cles, Service::Claude),
            ia_en_cours: self
                .ia
                .map(|(_, n)| format!("Claude lit {} floue{}...", n_offres(n), if n > 1 { "s" } else { "" })),
            ia_annonce: self.ia_annonce.clone(),
            ..VueOffres::default()
        };
        let Some(b) = base else { return (v, None) };
        match self.lire(b, &mut v) {
            Ok(()) => (v, None),
            Err(e) => (v, Some(format!("Offres : base illisible ({e})"))),
        }
    }

    fn lire(&mut self, b: &Base, v: &mut VueOffres) -> rusqlite::Result<()> {
        let c = b.connexion();
        preparer(c)?;
        let toutes = base_offres::lister(c)?;
        v.gardees = toutes.iter().filter(|o| !o.cachee).count();
        v.hors_sujet = toutes.iter().filter(|o| !o.cachee && o.rangee_hors_sujet()).count();
        v.floues = base_lecture::nombre_floues(c)?;
        v.nouvelles = toutes.iter().filter(|o| !o.vue && !o.cachee).count();
        v.favoris = toutes.iter().filter(|o| o.favori).count();
        v.plus_tard = toutes.iter().filter(|o| o.plus_tard).count();
        v.cachees = toutes.iter().filter(|o| o.cachee).count();
        v.recherches = base_offres::lister_recherches(c)?;
        v.profil = profil::lire_recherche(c)?.is_some();
        if !self.recherche.is_some_and(|id| v.recherches.iter().any(|r| r.id == id)) {
            self.recherche = v.recherches.first().map(|r| r.id);
        }
        v.recherche = self.recherche;
        // Les filtres
        let textes: Option<HashSet<i64>> = if self.filtres.texte.is_empty() {
            None
        } else {
            Some(base_offres::chercher_texte(c, &self.filtres.texte)?)
        };
        let (centre, autour): (Option<(f64, f64)>, Vec<i64>) = match &self.filtres.lieu {
            FiltreLieu::Ville { cle, rayon, .. } => (
                centre_ville(&toutes, cle),
                v.recherches
                    .iter()
                    .filter(
                        |r| matches!(&r.lieu, Lieu::Ville { nom, rayon: x } if cle_ville(nom) == *cle && x <= rayon),
                    )
                    .map(|r| r.id)
                    .collect(),
            ),
            _ => (None, Vec::new()),
        };
        let appui = Appui { textes: textes.as_ref(), centre, recherches_ville: &autour };
        let mut liste: Vec<Offre> = toutes.into_iter().filter(|o| garder(o, &self.filtres, &appui)).collect();
        trier_selon(&mut liste, self.filtres.tri);
        v.total = liste.len();
        liste.truncate(LISTE_MAX);
        // L'offre ouverte : celle choisie si elle est dans la liste, sinon la première
        if !self.ouverte.is_some_and(|id| liste.iter().any(|o| o.id == id)) {
            self.ouverte = liste.first().map(|o| o.id);
        }
        v.ouverte = match self.ouverte {
            Some(id) => base_offres::lire(c, id)?,
            None => None,
        };
        v.liste = liste;
        Ok(())
    }

    /// Un geste de la page : vérifié, puis écrit.
    pub fn geste(&mut self, ctx: &Contexte, id: &str, valeur: &Value, maintenant: Moment) -> Fait {
        if id == FILTRES {
            return self.filtrer(valeur);
        }
        if id == ADZUNA {
            return Fait {
                voir: Some(jobmind_sources::adzuna::SITE.to_string()),
                journal: Some("Offres : site d'Adzuna demandé (petite fenêtre du site)".into()),
                change: true,
                ..Fait::default()
            };
        }
        if id == ESSAYER {
            return self.essayer(ctx, valeur, maintenant);
        }
        let Some(base) = ctx.base else {
            return Fait::refus(
                crate::suivi::SANS_BASE,
                format!("Offres : {id} impossible, la base n'est pas ouverte"),
            );
        };
        let r = match id {
            OUVRIR => self.ouvrir(base, valeur),
            FAVORI => marquer(base, valeur, Marque::Favori),
            PLUS_TARD => marquer(base, valeur, Marque::PlusTard),
            CACHER => marquer(base, valeur, Marque::PasPourMoi),
            CHOISIR => {
                self.recherche = numero(valeur);
                Ok(Fait { change: true, ..Fait::default() })
            }
            CHERCHER => self.chercher(base, ctx.cles, valeur, maintenant),
            RECHERCHE => self.enregistrer_recherche(base, valeur, maintenant),
            RECHERCHE_SUPPRIMER => supprimer_recherche(base, valeur),
            DEPUIS_PROFIL => self.depuis_profil(base, maintenant),
            POSTULE => postule(base, valeur, maintenant),
            VOIR => voir(base, valeur),
            PAS_HORS_SUJET => marquer(base, valeur, Marque::PasHorsSujet),
            LIRE_IA => return self.lire_floues(ctx, numero(valeur), false, maintenant),
            _ => Ok(Fait::default()),
        };
        r.unwrap_or_else(|e| Fait::refus(PAS_ENREGISTRE, format!("Offres : {id} non enregistré ({e})")))
    }

    fn ouvrir(&mut self, base: &Base, v: &Value) -> rusqlite::Result<Fait> {
        let Some(id) = numero(v) else { return Ok(Fait::default()) };
        let Some(p) = base_offres::principale(base.connexion(), id)? else {
            return Ok(Fait { change: true, ..Fait::default() });
        };
        base_offres::marquer_vue(base.connexion(), p)?;
        self.ouverte = Some(p);
        Ok(Fait { change: true, ..Fait::default() })
    }

    fn filtrer(&mut self, v: &Value) -> Fait {
        let s = SaisieFiltres {
            genre: texte(v, "genre"),
            ville: coupe(&texte(v, "ville"), 80),
            rayon: v.get("rayon").and_then(Value::as_i64).unwrap_or(30),
            departement: texte(v, "departement"),
            region: texte(v, "region"),
            contrat: texte(v, "contrat"),
            teletravail: texte(v, "teletravail"),
            texte: coupe(&texte(v, "texte"), 200),
            liste: texte(v, "liste"),
            sans_diplome: v.get("sans_diplome").and_then(Value::as_bool).unwrap_or(false),
            debutant: v.get("debutant").and_then(Value::as_bool).unwrap_or(false),
            tri: texte(v, "tri"),
        };
        match verifier_filtres(&s) {
            Ok(f) => {
                self.filtres = f;
                self.saisie = s;
                Fait { change: true, ..Fait::default() }
            }
            Err(m) => Fait { message: Some(m), journal: Some("Offres : filtre refusé".into()), ..Fait::default() },
        }
    }

    /// « Chercher maintenant » : la tâche du fil des recherches, si une clé est prête et les limites le permettent.
    fn chercher(&mut self, base: &Base, cles: Option<&Cles>, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let c = base.connexion();
        if self.en_cours.is_some() {
            return Ok(message("Une recherche est déjà en cours : attends sa fin."));
        }
        let rid = numero(v).or(self.recherche);
        let Some(r) = base_offres::lister_recherches(c)?.into_iter().find(|r| Some(r.id) == rid) else {
            return Ok(message("Choisis d'abord une recherche."));
        };
        let prets: Vec<Service> =
            [Service::Adzuna, Service::Jooble].into_iter().filter(|s| cle_prete(cles, *s)).collect();
        if prets.is_empty() {
            let t = "Pour chercher des offres, Jobmind a besoin de ta clé Adzuna ou Jooble. Donne-la dans Mon profil, \
                     rubrique Mes clés.";
            self.annonce = Some(Annonce { texte: t.into(), genre: "erreur", cles: true });
            return Ok(Fait {
                message: Some(t.into()),
                journal: Some("Offres : recherche impossible, aucune clé".into()),
                change: true,
                ..Fait::default()
            });
        }
        let s = maintenant.secondes();
        self.parties.retain(|(_, quand)| s - quand < PAUSE_SECONDES);
        if self.parties.iter().any(|(x, _)| *x == r.id) {
            return Ok(message("Cette recherche vient d'être faite. Réessaie dans une minute."));
        }
        // Les limites du jour, comptées avant de partir
        let jour = maintenant.jour();
        let mut services = Vec::new();
        for sv in prets {
            if base_offres::appels(c, jour, sv.code())? < limite(sv) {
                base_offres::compter_appel(c, jour, sv.code())?;
                services.push(sv);
            }
        }
        if services.is_empty() {
            return Ok(message("Tu as déjà fait beaucoup de recherches aujourd'hui. Réessaie demain."));
        }
        self.parties.push((r.id, s));
        self.jetons += 1;
        let (lieu, rayon) = recherches::lieu_pour_source(&r.lieu);
        let noms = offre::noms_sources(&services.iter().map(|x| nom_source(x.code())).collect::<Vec<_>>());
        self.recherche = Some(r.id);
        self.en_cours = Some(EnCours {
            jeton: self.jetons,
            recherche_id: Some(r.id),
            texte: format!("Recherche « {} » en cours sur {noms}...", r.nom),
        });
        Ok(Fait {
            reseau: Some(Tache::Chercher {
                jeton: self.jetons,
                recherche_id: r.id,
                recherche: Recherche { mots_cles: r.mots.clone(), lieu, rayon_km: rayon, nombre: 0 },
                services: services.clone(),
            }),
            journal: Some(format!(
                "Offres : recherche {} lancée ({})",
                r.id,
                services.iter().map(|s| s.code()).collect::<Vec<_>>().join(", ")
            )),
            change: true,
            ..Fait::default()
        })
    }

    /// « Essayer la clé » (Mes clés) : une toute petite recherche, comptée comme les autres.
    fn essayer(&mut self, ctx: &Contexte, v: &Value, maintenant: Moment) -> Fait {
        let service = match Service::depuis_code(&texte(v, "service")) {
            Some(s @ (Service::Adzuna | Service::Jooble)) => s,
            _ => return Fait::default(),
        };
        let nom = nom_source(service.code());
        if !cle_prete(ctx.cles, service) {
            return message(format!("Donne d'abord ta clé {nom}."));
        }
        if self.en_cours.is_some() {
            return message("Une recherche est déjà en cours : attends sa fin.");
        }
        let Some(base) = ctx.base else {
            return Fait::refus(crate::suivi::SANS_BASE, "Offres : essai impossible, la base n'est pas ouverte".into());
        };
        let jour = maintenant.jour();
        let compte = base_offres::appels(base.connexion(), jour, service.code()).and_then(|n| {
            if n < limite(service) {
                base_offres::compter_appel(base.connexion(), jour, service.code()).map(Some)
            } else {
                Ok(None)
            }
        });
        match compte {
            Ok(Some(_)) => {}
            Ok(None) => return message("Tu as déjà fait beaucoup de recherches aujourd'hui. Réessaie demain."),
            Err(e) => return Fait::refus(PAS_ENREGISTRE, format!("Offres : essai non compté ({e})")),
        }
        self.jetons += 1;
        self.en_cours = Some(EnCours {
            jeton: self.jetons,
            recherche_id: None,
            texte: format!("Essai de la clé {nom} en cours..."),
        });
        Fait {
            message: Some(format!("Essai de la clé {nom} en cours...")),
            reseau: Some(Tache::Essayer { jeton: self.jetons, service }),
            journal: Some(format!("Clés : essai de la clé {} lancé", service.code())),
            change: true,
            ..Fait::default()
        }
    }

    /// Le résultat du fil des recherches : chaque offre gardée, ce qui a été trouvé dit en une phrase.
    pub fn fini(&mut self, base: Option<&Base>, r: Retour, maintenant: Moment) -> Fait {
        let Some(en_cours) = self.en_cours.as_ref().filter(|e| e.jeton == r.jeton).cloned() else {
            return Fait {
                journal: Some("Offres : résultat d'une recherche abandonnée, laissé".into()),
                ..Fait::default()
            };
        };
        self.en_cours = None;
        let Some(rid) = en_cours.recherche_id else { return essai_fini(r) };
        let Some(base) = base else {
            return Fait::refus(crate::suivi::SANS_BASE, "Offres : résultat perdu, la base n'est pas ouverte".into());
        };
        let mut bilan = Bilan::default();
        let mut trouvees = 0usize;
        let mut ok: Vec<&str> = Vec::new();
        let mut erreurs: Vec<ErreurSource> = Vec::new();
        let mut journal: Vec<String> = Vec::new();
        for (s, res) in &r.resultats {
            match res {
                Ok(l) => match base_offres::ajouter_trouvees(base.connexion(), s.code(), l, Some(rid), maintenant) {
                    Ok(b) => {
                        bilan.ajouter(b);
                        trouvees += l.len();
                        ok.push(nom_source(s.code()));
                        journal.push(format!("{} : {} offres, {} nouvelles", s.code(), l.len(), b.nouvelles));
                    }
                    Err(e) => {
                        journal.push(format!("{} : non gardées ({e})", s.code()));
                        erreurs.push(ErreurSource::Illisible(nom_source(s.code()).into(), "base".into()));
                    }
                },
                Err(e) => {
                    journal.push(format!("{} : {} ({e})", s.code(), e.code()));
                    erreurs.push(e.clone());
                }
            }
        }
        let mut texte = String::new();
        if !ok.is_empty() {
            texte = if trouvees == 0 {
                format!(
                    "Aucune offre trouvée sur {}. Essaie d'autres mots ou un lieu plus large.",
                    offre::noms_sources(&ok)
                )
            } else {
                let nouv = match bilan.nouvelles - bilan.fusionnees {
                    0 => "aucune nouvelle".to_string(),
                    1 => "1 nouvelle".to_string(),
                    n => format!("{n} nouvelles"),
                };
                let mut t = format!("{} trouvées sur {}, dont {nouv}.", n_offres(trouvees), offre::noms_sources(&ok));
                if trouvees == 1 {
                    t = t.replacen("trouvées", "trouvée", 1);
                }
                if bilan.fusionnees > 0 {
                    t.push_str(" Les offres en double sont regroupées.");
                }
                t
            };
        }
        // (un même souci chez plusieurs sources : une seule phrase)
        let mut dits: Vec<&str> = Vec::new();
        for e in &erreurs {
            if dits.contains(&e.code()) {
                continue;
            }
            dits.push(e.code());
            let memes: Vec<&str> = erreurs.iter().filter(|x| x.code() == e.code()).map(ErreurSource::source).collect();
            if !texte.is_empty() {
                texte.push(' ');
            }
            texte.push_str(&if memes.len() > 1 { e.message_groupe(&offre::noms_sources(&memes)) } else { e.message() });
        }
        let genre = match (ok.is_empty(), erreurs.is_empty()) {
            (_, true) => "ok",
            (false, false) => "attention",
            (true, false) => "erreur",
        };
        let cles = erreurs.iter().any(ErreurSource::vient_de_la_cle);
        self.annonce = Some(Annonce { texte: texte.clone(), genre, cles });
        let nombre = i64::try_from(trouvees).unwrap_or(i64::MAX);
        let _ = base_offres::noter_recherche(base.connexion(), rid, maintenant, nombre, &texte);
        Fait {
            message: Some(texte),
            bravo: bilan.nouvelles > bilan.fusionnees,
            journal: Some(format!("Offres : recherche {rid} finie ({})", journal.join(" ; "))),
            change: true,
            ..Fait::default()
        }
    }

    /// La recherche a duré trop longtemps : abandonnée (un résultat qui arriverait ensuite est laissé).
    pub fn trop_long(&mut self) -> Fait {
        self.en_cours = None;
        let t = "La recherche a pris trop de temps. Vérifie ta connexion à internet, puis réessaie.";
        self.annonce = Some(Annonce { texte: t.into(), genre: "erreur", cles: false });
        Fait::refus(t, "Offres : recherche trop longue, abandonnée".into())
    }

    /// Étape 6 : les offres floues données à Claude (`seule` : celle-ci seulement, sur un clic dans l'offre). `auto` :
    /// après une recherche, sans rien dire s'il n'y a rien à faire (pas de clé, rien de flou, le plafond atteint).
    pub fn lire_floues(&mut self, ctx: &Contexte, seule: Option<i64>, auto: bool, maintenant: Moment) -> Fait {
        let silence = |j: &str| Fait { journal: Some(j.into()), ..Fait::default() };
        let Some(base) = ctx.base else {
            return if auto {
                Fait::default()
            } else {
                Fait::refus(crate::suivi::SANS_BASE, "IA : base fermée".into())
            };
        };
        if !cle_prete(ctx.cles, Service::Claude) {
            if auto {
                return Fait::default();
            }
            let t = ErreurLlm::CleManquante.message();
            self.ia_annonce = Some(Annonce { texte: t.clone(), genre: "attention", cles: true });
            return Fait {
                message: Some(t),
                journal: Some("IA : lecture impossible, pas de clé Claude".into()),
                change: true,
                ..Fait::default()
            };
        }
        if self.ia.is_some() {
            return if auto {
                silence("IA : une lecture est déjà en cours")
            } else {
                message("Claude lit déjà des offres : attends la fin.")
            };
        }
        let c = base.connexion();
        let lu = (|| -> rusqlite::Result<(u32, Vec<OffreIa>)> {
            let (n, _) = base_lecture::ia_du_jour(c, maintenant.jour())?;
            let reste = PLAFOND_IA.saturating_sub(n) as usize;
            let l = match seule {
                Some(id) => {
                    let p = base_offres::principale(c, id)?.unwrap_or(id);
                    base_lecture::a_lire_par_ia(c, 5000)?.into_iter().filter(|x| x.0 == p).take(reste.min(1)).collect()
                }
                None => base_lecture::a_lire_par_ia(c, LOT_IA.min(reste))?,
            };
            Ok((n, l))
        })();
        let (deja, offres) = match lu {
            Ok(x) => x,
            Err(e) => return Fait::refus(PAS_ENREGISTRE, format!("IA : offres floues illisibles ({e})")),
        };
        if offres.is_empty() {
            if deja >= PLAFOND_IA {
                return if auto {
                    silence("IA : plafond du jour atteint")
                } else {
                    message(format!("Claude a déjà lu {PLAFOND_IA} offres aujourd'hui. La suite demain."))
                };
            }
            return if auto {
                Fait::default()
            } else {
                message("Aucune offre floue à lire : Jobmind a déjà tout compris.")
            };
        }
        self.jetons += 1;
        let n = offres.len();
        self.ia = Some((self.jetons, n));
        self.ia_annonce = None;
        Fait {
            ia: Some(TacheIa { jeton: self.jetons, offres }),
            journal: Some(format!("IA : {n} offres floues données à Claude")),
            change: true,
            ..Fait::default()
        }
    }

    /// Étape 6 : ce que Claude a lu, gardé, dit en une phrase.
    pub fn ia_finie(&mut self, base: Option<&Base>, r: RetourIa, maintenant: Moment) -> Fait {
        if self.ia.map(|x| x.0) != Some(r.jeton) {
            return Fait {
                journal: Some("IA : résultat d'une lecture abandonnée, laissé".into()), ..Fait::default()
            };
        }
        self.ia = None;
        let Some(base) = base else {
            return Fait::refus(crate::suivi::SANS_BASE, "IA : résultat perdu, la base n'est pas ouverte".into());
        };
        let (mut trouvees, mut inventees, mut illisibles, mut cout, mut jin, mut jout) = (0, 0, 0, 0u64, 0u64, 0u64);
        for x in &r.resultats {
            if let Err(e) = base_lecture::garder_ia(base.connexion(), x, maintenant) {
                return Fait::refus(PAS_ENREGISTRE, format!("IA : lecture non gardée ({e})"));
            }
            trouvees += usize::from(x.diplome.is_some() || x.experience.is_some());
            inventees += usize::from(x.etat == "citation_inventee");
            illisibles += usize::from(x.etat == "illisible");
            cout += x.cout_millioniemes;
            jin += u64::from(x.jetons_entree);
            jout += u64::from(x.jetons_sortie);
        }
        let n = r.resultats.len();
        let mut texte = String::new();
        if n > 0 {
            texte = format!(
                "Claude a lu {} floue{} : {} en {} plus (coût : {}).",
                n_offres(n),
                if n > 1 { "s" } else { "" },
                trouvees,
                if trouvees > 1 { "disent" } else { "dit" },
                cout_texte(cout)
            );
            if trouvees == 0 {
                texte = format!(
                    "Claude a lu {} floue{} : rien de plus n'y est dit (coût : {}).",
                    n_offres(n),
                    if n > 1 { "s" } else { "" },
                    cout_texte(cout)
                );
            }
        }
        if let Some(e) = &r.erreur {
            if !texte.is_empty() {
                texte.push(' ');
            }
            texte.push_str(&e.message());
        }
        let genre = match (&r.erreur, n) {
            (None, _) => "ok",
            (Some(_), 0) => "erreur",
            (Some(_), _) => "attention",
        };
        let cles = matches!(r.erreur, Some(ErreurLlm::CleRefusee(_) | ErreurLlm::CleManquante));
        self.ia_annonce = Some(Annonce { texte: texte.clone(), genre, cles });
        Fait {
            message: Some(texte),
            bravo: trouvees > 0,
            journal: Some(format!(
                "IA : {n} offres lues par Claude ({trouvees} avec du nouveau, {inventees} citations rejetées, \
                 {illisibles} illisibles, {jin} et {jout} jetons, {cout} millionièmes de dollar){}",
                r.erreur.as_ref().map(|e| format!(", arrêt : {} ({e})", e.code())).unwrap_or_default()
            )),
            change: true,
            ..Fait::default()
        }
    }

    /// Étape 6 : la lecture par Claude a duré trop longtemps : abandonnée.
    pub fn ia_trop_longue(&mut self) -> Fait {
        self.ia = None;
        let t = ErreurLlm::Delai.message();
        self.ia_annonce = Some(Annonce { texte: t.clone(), genre: "erreur", cles: false });
        Fait::refus(t, "IA : lecture trop longue, abandonnée".into())
    }

    /// Le jeton de la lecture par Claude en cours.
    pub fn jeton_ia(&self) -> Option<u64> {
        self.ia.map(|x| x.0)
    }

    /// Le jeton de la recherche en cours.
    pub fn jeton_en_cours(&self) -> Option<u64> {
        self.en_cours.as_ref().map(|e| e.jeton)
    }

    fn enregistrer_recherche(&mut self, base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let s = SaisieRechercheGardee {
            nom: texte(v, "nom"),
            mots: texte(v, "mots"),
            lieu: texte(v, "lieu"),
            ville: texte(v, "ville"),
            rayon: v.get("rayon").and_then(Value::as_i64).unwrap_or(0),
            departement: texte(v, "departement"),
            region: texte(v, "region"),
        };
        let n = match recherches::verifier(&s) {
            Ok(n) => n,
            Err(m) => {
                return Ok(Fait {
                    message: Some(m),
                    journal: Some("Offres : recherche refusée".into()),
                    ..Fait::default()
                });
            }
        };
        let c = base.connexion();
        let (id, m) = match numero(v) {
            Some(id) => {
                if !base_offres::modifier_recherche(c, id, &n)? {
                    return Ok(Fait::refus(
                        "Cette recherche n'existe plus.",
                        format!("Offres : recherche {id} introuvable"),
                    ));
                }
                (id, "Recherche enregistrée.")
            }
            None => {
                if base_offres::nombre_recherches(c)? >= NOMBRE_MAX {
                    return Ok(message(format!(
                        "Tu as déjà {NOMBRE_MAX} recherches : supprime-en une pour en ajouter une autre."
                    )));
                }
                (base_offres::ajouter_recherche(c, &n, maintenant)?, "Recherche ajoutée.")
            }
        };
        self.recherche = Some(id);
        Ok(Fait {
            message: Some(m.into()),
            bravo: true,
            ordre: Some(json!({ "kit": "accueil", "recherche_offres": id })),
            journal: Some(format!("Offres : recherche {id} enregistrée")),
            change: true,
            ..Fait::default()
        })
    }

    /// Les recherches tirées de « Ce que je cherche » : une par poste (celles qui existent déjà ne sont pas refaites).
    fn depuis_profil(&mut self, base: &Base, maintenant: Moment) -> rusqlite::Result<Fait> {
        let c = base.connexion();
        let Some(r) = profil::lire_recherche(c)? else {
            return Ok(message("Remplis d'abord « Ce que je cherche » dans Mon profil."));
        };
        let deja = base_offres::lister_recherches(c)?;
        let mut n = deja.len();
        let mut ajoutees = 0;
        let mut premiere = None;
        for x in depuis_profil(&r) {
            let existe = deja.iter().any(|d| d.mots.to_lowercase() == x.mots.to_lowercase() && d.lieu == x.lieu);
            if existe || n >= NOMBRE_MAX {
                continue;
            }
            let id = base_offres::ajouter_recherche(c, &x, maintenant)?;
            premiere = premiere.or(Some(id));
            n += 1;
            ajoutees += 1;
        }
        if ajoutees == 0 {
            return Ok(message("Tes recherches sont déjà là."));
        }
        self.recherche = premiere.or(self.recherche);
        Ok(Fait {
            message: Some(if ajoutees == 1 {
                "1 recherche créée à partir de « Ce que je cherche ».".to_string()
            } else {
                format!("{ajoutees} recherches créées à partir de « Ce que je cherche ».")
            }),
            bravo: true,
            ordre: Some(json!({ "kit": "accueil", "recherche_offres": premiere })),
            journal: Some(format!("Offres : {ajoutees} recherches créées depuis le profil")),
            change: true,
            ..Fait::default()
        })
    }
}

/// Étape 6 : avant de montrer la liste, les offres pas encore lues sont lues par les règles, et les notes faites avec
/// un autre « Ce que je cherche » ou un autre CV sont refaites (sans « Ce que je cherche » : aucune note).
fn preparer(c: &rusqlite::Connection) -> rusqlite::Result<()> {
    base_lecture::relire(c)?;
    let Some(r) = profil::lire_recherche(c)? else {
        base_lecture::effacer_notes(c)?;
        return Ok(());
    };
    let cv = profil::lister_cv(c)?.into_iter().find(|x| x.principal).map(|x| x.texte).unwrap_or_default();
    let mut p = pertinence::Profil::nouveau(&r, &cv);
    if let Lieu::Ville { nom, .. } = &r.lieu {
        let toutes = base_offres::lister(c)?;
        let cle = cle_ville(nom);
        p.centre = centre_ville(&toutes, &cle);
        p.departement_ville = toutes.iter().filter(|o| cle_ville(&o.lieu) == cle).find_map(|o| o.departement.clone());
    }
    let signature = pertinence::signature(&r, &cv);
    base_lecture::renoter(c, &signature, |o| {
        pertinence::noter(
            &p,
            &OffreANoter {
                titre: &o.titre,
                description: &o.description,
                lieu: &o.lieu,
                departement: o.departement.as_deref(),
                region: o.region.as_deref(),
                point: o.point,
                contrat: o.contrat,
                teletravail: o.teletravail,
                salaire: &o.salaire,
                diplome: o.diplome,
                experience: o.experience,
            },
        )
    })?;
    Ok(())
}

/// Le résultat d'un essai de clé.
fn essai_fini(r: Retour) -> Fait {
    let Some((s, res)) = r.resultats.into_iter().next() else { return Fait { change: true, ..Fait::default() } };
    let nom = nom_source(s.code());
    match res {
        Ok(_) => Fait {
            message: Some(format!("La clé {nom} marche : {nom} a bien répondu.")),
            bravo: true,
            journal: Some(format!("Clés : essai de la clé {} réussi", s.code())),
            change: true,
            ..Fait::default()
        },
        Err(e) => Fait {
            message: Some(e.message()),
            journal: Some(format!("Clés : essai de la clé {} : {} ({e})", s.code(), e.code())),
            change: true,
            ..Fait::default()
        },
    }
}

fn marquer(base: &Base, v: &Value, m: Marque) -> rusqlite::Result<Fait> {
    let Some(id) = numero(v) else { return Ok(Fait::default()) };
    let Some(oui) = base_offres::basculer(base.connexion(), id, m)? else {
        return Ok(Fait { change: true, ..Fait::default() });
    };
    let texte = match (m, oui) {
        (Marque::Favori, true) => "Ajoutée aux favoris.",
        (Marque::Favori, false) => "Retirée des favoris.",
        (Marque::PlusTard, true) => "Gardée pour plus tard.",
        (Marque::PlusTard, false) => "Retirée de « Plus tard ».",
        (Marque::PasPourMoi, true) => "Offre cachée. Tu la retrouves dans « Cachées ».",
        (Marque::PasPourMoi, false) => "L'offre est de nouveau dans la liste.",
        (Marque::PasHorsSujet, true) => "C'est noté : l'offre revient dans la liste.",
        (Marque::PasHorsSujet, false) => "L'offre est de nouveau rangée dans « Hors sujet ».",
    };
    let code = match m {
        Marque::Favori => "favori",
        Marque::PlusTard => "plus tard",
        Marque::PasPourMoi => "pas pour moi",
        Marque::PasHorsSujet => "pas hors sujet",
    };
    Ok(Fait {
        message: Some(texte.into()),
        journal: Some(format!("Offres : offre {id}, {code} {}", if oui { "oui" } else { "non" })),
        change: true,
        ..Fait::default()
    })
}

fn supprimer_recherche(base: &Base, v: &Value) -> rusqlite::Result<Fait> {
    let Some(id) = numero(v) else { return Ok(Fait::default()) };
    if !base_offres::supprimer_recherche(base.connexion(), id)? {
        return Ok(Fait { change: true, ..Fait::default() });
    }
    Ok(Fait {
        message: Some("Recherche supprimée. Ses offres restent gardées.".into()),
        ordre: Some(json!({ "kit": "accueil", "recherche_supprimee": id })),
        journal: Some(format!("Offres : recherche {id} supprimée")),
        change: true,
        ..Fait::default()
    })
}

/// « Voir sur le site » : l'adresse de l'offre chez cette source, si elle s'ouvre sans risque.
fn voir(base: &Base, v: &Value) -> rusqlite::Result<Fait> {
    let Some(id) = numero(v) else { return Ok(Fait::default()) };
    let Some(o) = base_offres::lire(base.connexion(), id)? else {
        return Ok(Fait { change: true, ..Fait::default() });
    };
    let source = texte(v, "source");
    let lien =
        o.liens.iter().find(|l| l.source == source && !l.url.is_empty()).or(o.liens.iter().find(|l| !l.url.is_empty()));
    match lien {
        Some(l) if lien_ouvrable(&l.url) => Ok(Fait {
            voir: Some(l.url.clone()),
            change: true,
            journal: Some(format!("Offres : offre {id} ouverte sur le site ({})", offre::hote(&l.url))),
            ..Fait::default()
        }),
        _ => Ok(message("Cette offre n'a pas d'adresse sur le site.")),
    }
}

/// « J'ai postulé » : une candidature du suivi, liée à l'offre (la même ouverte si elle existe déjà).
fn postule(base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
    let Some(id) = numero(v) else { return Ok(Fait::default()) };
    let c = base.connexion();
    let Some(o) = base_offres::lire(c, id)? else { return Ok(Fait { change: true, ..Fait::default() }) };
    if let Some(k) = o.offre.candidature {
        return Ok(Fait {
            candidature: Some(k),
            ordre: Some(json!({ "kit": "accueil", "candidature": k })),
            change: true,
            ..Fait::default()
        });
    }
    let ville = o.offre.lieu.split([',', '(']).next().unwrap_or_default().trim().to_string();
    let sources: Vec<&str> = o.offre.sources.iter().map(|s| nom_source(s)).collect();
    let lien = o.liens.iter().map(|l| l.url.clone()).find(|u| lien_ouvrable(u) && u.chars().count() <= LIEN_MAX);
    let entreprise = if o.offre.entreprise.trim().is_empty() {
        "Entreprise non précisée".to_string()
    } else {
        o.offre.entreprise.clone()
    };
    let s = Saisie {
        entreprise: coupe(&entreprise, ENTREPRISE_MAX),
        poste: coupe(&o.offre.titre, POSTE_MAX),
        ville: coupe(&ville, VILLE_MAX),
        date: maintenant.jour().saisie(),
        moyen: "site".into(),
        source: coupe(&offre::noms_sources(&sources), SOURCE_MAX),
        lien: lien.unwrap_or_default(),
        envoye: String::new(),
        cv: String::new(),
        notes: String::new(),
    };
    let f = match verifier_candidature(&s, maintenant.jour()) {
        Ok(f) => f,
        Err(r) => return Ok(Fait::refus(r, format!("Offres : candidature de l'offre {id} refusée"))),
    };
    let k = candidatures::ajouter(c, &f, maintenant)?;
    base_offres::lier_candidature(c, o.offre.id, k)?;
    Ok(Fait {
        message: Some("C'est noté dans tes candidatures. Ajoute ce que tu as envoyé avec « Modifier ».".into()),
        bravo: true,
        candidature: Some(k),
        ordre: Some(json!({ "kit": "accueil", "candidature": k })),
        journal: Some(format!("Offres : candidature {k} créée pour l'offre {}", o.offre.id)),
        change: true,
        ..Fait::default()
    })
}

/// Les parties de clé d'un service, lues en clair au moment de lancer le fil (jamais gardées ailleurs).
pub fn cles_claires(cles: &Cles, s: Service) -> Option<Vec<String>> {
    s.parties().iter().map(|p: &Cle| cles.lire(*p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jobmind_sources::{adzuna, faux, jooble};
    use jobmind_storage::cles::Coffre;

    fn m(h: u32, mi: u32) -> Moment {
        Moment::nouveau(2026, 11, 18, h, mi, 0).unwrap()
    }

    struct Faux;
    impl Coffre for Faux {
        fn fermer(&self, c: &[u8]) -> Result<Vec<u8>, String> {
            Ok(c.iter().rev().map(|b| b ^ 0x5a).collect())
        }
        fn ouvrir(&self, c: &[u8]) -> Result<Vec<u8>, String> {
            Ok(c.iter().rev().map(|b| b ^ 0x5a).collect())
        }
    }

    fn essai(nom: &str) -> std::path::PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-{nom}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn reponses() -> Vec<(Service, Result<Vec<OffreTrouvee>, ErreurSource>)> {
        vec![
            (Service::Adzuna, adzuna::lire(200, &faux::reponse_adzuna(), false)),
            (Service::Jooble, jooble::lire(200, &faux::reponse_jooble(), false)),
        ]
    }

    /// Le scénario de l'écran : sans clé, « Chercher maintenant » renvoie vers Mes clés ; les recherches tirées de
    /// « Ce que je cherche » ; avec les clés, la tâche part (sans clé dedans), le résultat est gardé, les doublons
    /// regroupés ; pas deux fois dans la minute ; puis favori, filtre, « J'ai postulé ».
    #[test]
    fn scenario_chercher_garder_filtrer_postuler() {
        let d = essai("offres");
        let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
        let (mut cles, _) = Cles::ouvrir(&d, Box::new(Faux));
        let mut o = Offres::default();
        // Ce que je cherche, puis les recherches qui en viennent
        let s = jobmind_core::recherche::verifier_recherche(&jobmind_core::recherche::SaisieRecherche {
            postes: "agent de quai, cariste".into(),
            lieu: "ville".into(),
            ville: "Metz".into(),
            rayon: 30,
            teletravail: "peu_importe".into(),
            ..Default::default()
        })
        .unwrap();
        profil::enregistrer_recherche(b.connexion(), &s, m(9, 0)).unwrap();
        let f = o.geste(&Contexte { base: Some(&b), cles: Some(&cles) }, DEPUIS_PROFIL, &Value::Null, m(9, 0));
        assert_eq!(f.message.as_deref(), Some("2 recherches créées à partir de « Ce que je cherche »."));
        let f = o.geste(&Contexte { base: Some(&b), cles: Some(&cles) }, DEPUIS_PROFIL, &Value::Null, m(9, 0));
        assert_eq!(f.message.as_deref(), Some("Tes recherches sont déjà là."));
        // Sans clé : rien ne part, l'annonce renvoie vers Mes clés
        let f = o.geste(&Contexte { base: Some(&b), cles: Some(&cles) }, CHERCHER, &json!({}), m(9, 1));
        assert!(f.reseau.is_none());
        assert!(f.message.unwrap().contains("Mes clés"));
        assert!(o.annonce.as_ref().unwrap().cles);
        // Avec les deux clés
        cles.garder_plusieurs(&[(Cle::AdzunaId, "1a2b3c4d"), (Cle::AdzunaCle, "0123456789abcdef0123456789abcdef")])
            .unwrap();
        cles.garder(Cle::Jooble, "aaaabbbb-cccc-dddd-eeee-ffff00001111").unwrap();
        let ctx = Contexte { base: Some(&b), cles: Some(&cles) };
        let f = o.geste(&ctx, CHERCHER, &json!({}), m(9, 2));
        let Some(Tache::Chercher { jeton, recherche_id, recherche, services }) = f.reseau.clone() else {
            panic!("{f:?}")
        };
        assert_eq!(services, [Service::Adzuna, Service::Jooble]);
        assert_eq!(
            (recherche.mots_cles.as_str(), recherche.lieu.as_str(), recherche.rayon_km),
            ("agent de quai", "Metz", 30)
        );
        // La tâche ne contient aucune clé
        let t = format!("{f:?}");
        assert!(!t.contains("0123456789abcdef") && !t.contains("aaaabbbb"), "{t}");
        let (v, _) = o.vue(Some(&b), Some(&cles));
        assert_eq!(v.en_cours.as_deref(), Some("Recherche « Agent de quai » en cours sur Adzuna et Jooble..."));
        // Une seconde pendant la première : refusée
        assert!(o.geste(&ctx, CHERCHER, &json!({}), m(9, 2)).reseau.is_none());
        // Le résultat : 19 offres, 17 gardées (2 en double), l'annonce
        let f = o.fini(Some(&b), Retour { jeton, resultats: reponses() }, m(9, 3));
        assert_eq!(
            f.message.as_deref(),
            Some("19 offres trouvées sur Adzuna et Jooble, dont 17 nouvelles. Les offres en double sont regroupées.")
        );
        assert!(f.bravo);
        assert!(
            f.journal
                .as_deref()
                .unwrap()
                .starts_with(&format!("Offres : recherche {recherche_id} finie (adzuna : 12 offres"))
        );
        let (v, _) = o.vue(Some(&b), Some(&cles));
        // (étape 6 : les offres qui ne parlent ni d'agent de quai ni de cariste sont rangées dans « Hors sujet »)
        assert_eq!((v.gardees, v.total, v.nouvelles, v.liste.len(), v.hors_sujet), (17, 6, 17, 6, 11));
        assert_eq!(v.recherches[0].dernier_nombre, Some(19));
        assert!(v.en_cours.is_none());
        // Un résultat qui arrive trop tard : laissé
        assert!(o.fini(Some(&b), Retour { jeton, resultats: reponses() }, m(9, 3)).message.is_none());
        // Pas deux fois dans la minute ; les demandes du jour comptées
        let f =
            o.geste(&ctx, CHERCHER, &json!({ "id": recherche_id }), Moment::nouveau(2026, 11, 18, 9, 2, 50).unwrap());
        assert_eq!(f.message.as_deref(), Some("Cette recherche vient d'être faite. Réessaie dans une minute."));
        assert_eq!(base_offres::appels(b.connexion(), m(9, 0).jour(), "adzuna").unwrap(), 1);
        // Ouvrir, favori, cacher
        let quai = v.liste.iter().find(|x| x.titre == "Agent de quai H/F").unwrap().id;
        o.geste(&ctx, OUVRIR, &json!({ "id": quai }), m(9, 4));
        let f = o.geste(&ctx, FAVORI, &json!({ "id": quai }), m(9, 4));
        assert_eq!(f.message.as_deref(), Some("Ajoutée aux favoris."));
        let (v, _) = o.vue(Some(&b), Some(&cles));
        let ouv = v.ouverte.as_ref().unwrap();
        assert_eq!((ouv.offre.id, ouv.offre.favori, ouv.liens.len()), (quai, true, 2));
        assert_eq!(v.nouvelles, 16);
        // Les filtres : favoris ; puis un texte ; puis un département
        o.geste(&ctx, FILTRES, &json!({ "genre": "partout", "liste": "favoris" }), m(9, 5));
        assert_eq!(o.vue(Some(&b), Some(&cles)).0.total, 1);
        o.geste(&ctx, FILTRES, &json!({ "genre": "partout", "texte": "chariot élévateur" }), m(9, 5));
        let (v, _) = o.vue(Some(&b), Some(&cles));
        assert_eq!(v.liste.iter().map(|x| x.titre.as_str()).collect::<Vec<_>>(), ["Cariste CACES 3"]);
        assert_eq!(v.ouverte.as_ref().unwrap().offre.titre, "Cariste CACES 3");
        o.geste(&ctx, FILTRES, &json!({ "genre": "departement", "departement": "54" }), m(9, 5));
        // (Nancy chez Adzuna, Pont-à-Mousson ; « Nancy » seul chez Jooble ne dit pas son département ; étape 6 : les
        // deux sont hors sujet pour « agent de quai, cariste »)
        assert_eq!(o.vue(Some(&b), Some(&cles)).0.total, 0);
        o.geste(&ctx, FILTRES, &json!({ "genre": "departement", "departement": "54", "liste": "hors_sujet" }), m(9, 5));
        assert_eq!(o.vue(Some(&b), Some(&cles)).0.total, 2);
        let f = o.geste(&ctx, FILTRES, &json!({ "genre": "departement", "departement": "99" }), m(9, 5));
        assert_eq!(f.message.as_deref(), Some("Choisis un département dans la liste."));
        o.geste(&ctx, FILTRES, &json!({ "genre": "ville", "ville": "Metz", "rayon": 10 }), m(9, 5));
        let (v, _) = o.vue(Some(&b), Some(&cles));
        assert!(v.liste.iter().all(|x| !x.lieu.contains("Nancy") && !x.lieu.contains("Thionville")), "{:?}", v.liste);
        assert!(v.liste.iter().any(|x| x.lieu.starts_with("Woippy")));
        o.geste(&ctx, FILTRES, &json!({ "genre": "partout" }), m(9, 5));
        // « J'ai postulé » : la candidature, liée ; une seconde fois : la même
        let f = o.geste(&ctx, POSTULE, &json!({ "id": quai }), m(10, 0));
        let k = f.candidature.unwrap();
        let c = candidatures::lire(b.connexion(), k).unwrap().unwrap();
        assert_eq!(
            (c.entreprise.as_str(), c.poste.as_str(), c.ville.as_str()),
            ("Entrepôts du Val de Moselle", "Agent de quai H/F", "Ennery")
        );
        assert_eq!(c.source, "Adzuna et Jooble");
        assert!(c.lien.starts_with("https://www.adzuna.fr/"));
        assert_eq!(o.geste(&ctx, POSTULE, &json!({ "id": quai }), m(10, 0)).candidature, Some(k));
        // « Voir sur le site » : l'adresse de la source choisie, jamais dans le journal
        let f = o.geste(&ctx, VOIR, &json!({ "id": quai, "source": "jooble" }), m(10, 0));
        assert!(matches!(&f.voir, Some(u) if u.starts_with("https://jooble.org/desc/")));
        assert_eq!(f.journal.as_deref(), Some(&*format!("Offres : offre {quai} ouverte sur le site (jooble.org)")));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Les erreurs des sources : une seule source répond, aucune ne répond (réseau, clé refusée) ; la recherche trop
    /// longue ; l'essai d'une clé.
    #[test]
    fn erreurs_et_essai_de_cle() {
        let d = essai("offres-erreurs");
        let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
        let (mut cles, _) = Cles::ouvrir(&d, Box::new(Faux));
        cles.garder(Cle::Jooble, "aaaabbbb-cccc-dddd-eeee-ffff00001111").unwrap();
        let mut o = Offres::default();
        let ctx = Contexte { base: Some(&b), cles: Some(&cles) };
        let f = o.geste(&ctx, RECHERCHE, &json!({ "mots": "cariste", "lieu": "partout" }), m(9, 0));
        assert_eq!(f.message.as_deref(), Some("Recherche ajoutée."));
        // Seulement Jooble (pas de clé Adzuna) ; il ne répond pas
        let f = o.geste(&ctx, CHERCHER, &json!({}), m(9, 1));
        let Some(Tache::Chercher { jeton, services, .. }) = f.reseau else { panic!() };
        assert_eq!(services, [Service::Jooble]);
        let f = o.fini(
            Some(&b),
            Retour {
                jeton,
                resultats: vec![(
                    Service::Jooble,
                    Err(ErreurSource::Reseau("Jooble".into(), "connexion impossible".into())),
                )],
            },
            m(9, 2),
        );
        assert_eq!(f.message.as_deref(), Some("Jooble ne répond pas. Vérifie ta connexion à internet, puis réessaie."));
        assert_eq!(o.annonce.as_ref().unwrap().genre, "erreur");
        assert!(!o.annonce.as_ref().unwrap().cles);
        // L'une répond, l'autre refuse la clé
        let f = o.geste(&ctx, CHERCHER, &json!({}), m(9, 5));
        let Some(Tache::Chercher { jeton, .. }) = f.reseau else { panic!("{f:?}") };
        let f = o.fini(
            Some(&b),
            Retour {
                jeton,
                resultats: vec![
                    (Service::Adzuna, Err(ErreurSource::CleRefusee("Adzuna".into(), "statut 401".into()))),
                    (Service::Jooble, jooble::lire(200, &faux::reponse_jooble(), false)),
                ],
            },
            m(9, 6),
        );
        assert_eq!(
            f.message.as_deref(),
            Some(
                "7 offres trouvées sur Jooble, dont 7 nouvelles. Adzuna refuse ta clé. Vérifie-la dans Mon profil, rubrique Mes clés."
            )
        );
        let a = o.annonce.clone().unwrap();
        assert_eq!((a.genre, a.cles), ("attention", true));
        // Trop longue
        let f = o.geste(&ctx, CHERCHER, &json!({}), m(9, 10));
        assert!(f.reseau.is_some());
        let f = o.trop_long();
        assert!(f.message.unwrap().contains("trop de temps"));
        assert!(o.en_cours.is_none());
        // Essayer la clé : sans clé Adzuna, rien ; avec Jooble, la tâche ; son résultat
        assert_eq!(
            o.geste(&ctx, ESSAYER, &json!({ "service": "adzuna" }), m(9, 11)).message.as_deref(),
            Some("Donne d'abord ta clé Adzuna.")
        );
        assert!(o.geste(&ctx, ESSAYER, &json!({ "service": "claude" }), m(9, 11)).reseau.is_none());
        let f = o.geste(&ctx, ESSAYER, &json!({ "service": "jooble" }), m(9, 11));
        let Some(Tache::Essayer { jeton, service }) = f.reseau else { panic!() };
        assert_eq!(service, Service::Jooble);
        let f = o.fini(Some(&b), Retour { jeton, resultats: vec![(Service::Jooble, Ok(Vec::new()))] }, m(9, 12));
        assert_eq!(f.message.as_deref(), Some("La clé Jooble marche : Jooble a bien répondu."));
        let f = o.geste(&ctx, ESSAYER, &json!({ "service": "jooble" }), m(9, 13));
        let Some(Tache::Essayer { jeton, .. }) = f.reseau else { panic!() };
        let f = o.fini(
            Some(&b),
            Retour {
                jeton,
                resultats: vec![(Service::Jooble, Err(ErreurSource::Bloquee("Jooble".into(), "statut 403".into())))],
            },
            m(9, 14),
        );
        assert_eq!(f.message.as_deref(), Some("Jooble a bloqué la demande pour l'instant. Réessaie plus tard."));
        // La limite du jour : au bout, plus rien ne part
        for _ in 0..LIMITE_JOOBLE {
            base_offres::compter_appel(b.connexion(), m(9, 0).jour(), "jooble").unwrap();
        }
        let f = o.geste(&ctx, CHERCHER, &json!({}), m(11, 0));
        assert!(f.reseau.is_none());
        assert_eq!(f.message.as_deref(), Some("Tu as déjà fait beaucoup de recherches aujourd'hui. Réessaie demain."));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Le même souci chez les deux sources : une seule phrase, au pluriel.
    #[test]
    fn meme_souci_une_phrase() {
        let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
        let mut o = Offres::default();
        o.geste(
            &Contexte { base: Some(&b), cles: None },
            RECHERCHE,
            &json!({ "mots": "cariste", "lieu": "partout" }),
            m(9, 0),
        );
        o.en_cours = Some(EnCours { jeton: 7, recherche_id: o.recherche, texte: String::new() });
        let reseau = |n: &str| Err(ErreurSource::Reseau(n.into(), "connexion impossible".into()));
        let f = o.fini(
            Some(&b),
            Retour {
                jeton: 7,
                resultats: vec![(Service::Adzuna, reseau("Adzuna")), (Service::Jooble, reseau("Jooble"))],
            },
            m(9, 1),
        );
        assert_eq!(
            f.message.as_deref(),
            Some("Adzuna et Jooble ne répondent pas. Vérifie ta connexion à internet, puis réessaie.")
        );
    }

    /// Une fausse IA : rend ces réponses dans l'ordre (aucun réseau).
    struct FausseIa(std::sync::Mutex<Vec<Result<jobmind_llm::Reponse, ErreurLlm>>>);
    impl LlmProvider for FausseIa {
        fn nom(&self) -> &str {
            "Fausse"
        }
        fn repondre(&self, _: &jobmind_llm::Demande) -> Result<jobmind_llm::Reponse, ErreurLlm> {
            let mut l = self.0.lock().unwrap();
            if l.is_empty() { Err(ErreurLlm::Reseau("plus rien".into())) } else { l.remove(0) }
        }
    }

    fn rep(t: &str) -> Result<jobmind_llm::Reponse, ErreurLlm> {
        Ok(jobmind_llm::Reponse { texte: t.into(), jetons_entree: 800, jetons_sortie: 60 })
    }

    const FLOU: &str = "Notre magasin recrute un employé polyvalent pour la mise en rayon, l'accueil des clients et la \
                        caisse. Vous aimez le contact avec les clients et le travail en équipe. L'équipe vous accueille \
                        avec plaisir du mardi au samedi, horaires en journée.";

    /// Étape 6 : la note et la raison dans la liste, « Aucun diplôme requis », Hors sujet et « Pas hors sujet ».
    #[test]
    fn la_note_les_filtres_et_hors_sujet() {
        let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
        let c = b.connexion();
        let t = |id: &str, titre: &str, d: &str| OffreTrouvee {
            id_source: id.into(),
            titre: titre.into(),
            entreprise: "Exemple".into(),
            lieu: "Metz (57)".into(),
            zones: Vec::new(),
            url: String::new(),
            description: d.into(),
            contrat: String::new(),
            salaire: String::new(),
            publiee_le: String::new(),
            latitude: None,
            longitude: None,
            brut: "{}".into(),
        };
        base_offres::ajouter_trouvees(
            c,
            "adzuna",
            &[
                t("1", "Cariste", "Sans diplôme, débutant accepté."),
                t("2", "Cariste confirmé", "CACES 3, 2 ans d'expérience exigés."),
                t("3", "Infirmier de nuit", "Diplôme d'État exigé."),
            ],
            None,
            m(9, 0),
        )
        .unwrap();
        let mut o = Offres::default();
        // Sans « Ce que je cherche » : pas de note, la phrase qui le demande
        let (v, _) = o.vue(Some(&b), None);
        assert!(v.liste.iter().all(|x| x.pertinence.is_none()));
        let s = jobmind_core::recherche::verifier_recherche(&jobmind_core::recherche::SaisieRecherche {
            postes: "cariste".into(),
            lieu: "ville".into(),
            ville: "Metz".into(),
            rayon: 30,
            teletravail: "peu_importe".into(),
            sans_diplome: true,
            debutant: true,
            ..Default::default()
        })
        .unwrap();
        profil::enregistrer_recherche(c, &s, m(9, 0)).unwrap();
        let (v, _) = o.vue(Some(&b), None);
        assert_eq!(v.liste.iter().map(|x| x.titre.as_str()).collect::<Vec<_>>(), ["Cariste", "Cariste confirmé"]);
        assert_eq!(v.liste[0].raison, "Aucun diplôme demandé, à Metz, débutant accepté");
        assert_eq!(v.hors_sujet, 1);
        // Les filtres de la lecture
        let ctx = Contexte { base: Some(&b), cles: None };
        o.geste(&ctx, FILTRES, &json!({ "genre": "partout", "sans_diplome": true }), m(9, 1));
        assert_eq!(o.vue(Some(&b), None).0.total, 1);
        o.geste(&ctx, FILTRES, &json!({ "genre": "partout", "liste": "hors_sujet" }), m(9, 1));
        let (v, _) = o.vue(Some(&b), None);
        let infirmier = v.liste[0].id;
        assert_eq!(v.liste[0].titre, "Infirmier de nuit");
        let f = o.geste(&ctx, PAS_HORS_SUJET, &json!({ "id": infirmier }), m(9, 2));
        assert_eq!(f.message.as_deref(), Some("C'est noté : l'offre revient dans la liste."));
        assert_eq!(o.vue(Some(&b), None).0.total, 0);
        o.geste(&ctx, FILTRES, &json!({ "genre": "partout", "tri": "recentes" }), m(9, 3));
        let (v, _) = o.vue(Some(&b), None);
        assert_eq!(v.liste.len(), 3);
        // « Ce que je cherche » change : la note suit
        let s2 = jobmind_core::recherche::SaisieRecherche {
            postes: "infirmier".into(),
            lieu: "partout".into(),
            teletravail: "peu_importe".into(),
            ..Default::default()
        };
        profil::enregistrer_recherche(c, &jobmind_core::recherche::verifier_recherche(&s2).unwrap(), m(9, 4)).unwrap();
        let (v, _) = o.vue(Some(&b), None);
        assert_eq!(v.liste.iter().map(|x| x.titre.as_str()).collect::<Vec<_>>(), ["Infirmier de nuit"]);
        assert!(o.geste(&ctx, FILTRES, &json!({ "tri": "hasard" }), m(9, 5)).message.is_some());
    }

    /// Étape 6 : Claude lit les offres floues ; sans clé, rien ne part et c'est dit ; une citation inventée est
    /// rejetée ; 429 arrête le lot ; pas de deuxième appel pour la même offre.
    #[test]
    fn claude_lit_les_offres_floues() {
        let d = essai("offres-ia");
        let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
        let c = b.connexion();
        let t = |id: &str, titre: &str| OffreTrouvee {
            id_source: id.into(),
            titre: titre.into(),
            entreprise: "Exemple".into(),
            lieu: "Metz".into(),
            zones: Vec::new(),
            url: String::new(),
            description: FLOU.into(),
            contrat: String::new(),
            salaire: String::new(),
            publiee_le: String::new(),
            latitude: None,
            longitude: None,
            brut: "{}".into(),
        };
        base_offres::ajouter_trouvees(
            c,
            "adzuna",
            &[t("1", "Employé polyvalent"), t("2", "Vendeur"), t("3", "Caissier")],
            None,
            m(9, 0),
        )
        .unwrap();
        let (mut cles, _) = Cles::ouvrir(&d, Box::new(Faux));
        let mut o = Offres::default();
        let (v, _) = o.vue(Some(&b), Some(&cles));
        assert_eq!((v.floues, v.claude), (3, false));
        // Sans clé : rien ne part ; après une recherche, rien n'est dit
        let ctx = Contexte { base: Some(&b), cles: Some(&cles) };
        assert!(o.lire_floues(&ctx, None, true, m(9, 1)).ia.is_none());
        let f = o.geste(&ctx, LIRE_IA, &json!({}), m(9, 1));
        assert!(f.ia.is_none());
        assert_eq!(
            f.message.as_deref(),
            Some(
                "Ajoute ta clé Claude dans Mon profil, rubrique Mes clés, pour que Jobmind lise aussi les offres floues."
            )
        );
        // Avec la clé : un lot, sans clé dedans
        cles.garder(Cle::Claude, "sk-ant-api03-InventeeInventeeInventeeInventeeInventee_0000-AAAA").unwrap();
        let ctx = Contexte { base: Some(&b), cles: Some(&cles) };
        let f = o.geste(&ctx, LIRE_IA, &json!({}), m(9, 2));
        let tache = f.ia.clone().unwrap();
        assert_eq!(tache.offres.len(), 3);
        assert!(!format!("{f:?}").contains("sk-ant"));
        assert!(o.vue(Some(&b), Some(&cles)).0.ia_en_cours.unwrap().starts_with("Claude lit 3 offres floues"));
        // La fausse IA : une réponse juste, une citation inventée, puis 429
        let ia = FausseIa(std::sync::Mutex::new(vec![
            rep(
                r#"{"diplome":"non_dit","diplome_citation":"","experience":"souhaitee","experience_mois":0,"experience_citation":"Vous aimez le contact avec les clients"}"#,
            ),
            rep(
                r#"{"diplome":"bac","diplome_citation":"Bac exigé pour ce poste","experience":"non_dit","experience_mois":0,"experience_citation":""}"#,
            ),
            Err(ErreurLlm::Limite),
        ]));
        let r = lire_lot(&ia, &tache);
        assert_eq!((r.resultats.len(), r.erreur.clone()), (2, Some(ErreurLlm::Limite)));
        assert_eq!(r.resultats[1].etat, "citation_inventee");
        let f = o.ia_finie(Some(&b), r, m(9, 3));
        assert_eq!(
            f.message.as_deref(),
            Some(
                "Claude a lu 2 offres floues : 1 en dit plus (coût : moins d'un centime). Claude reçoit trop de demandes pour l'instant. Réessaie dans quelques minutes."
            )
        );
        assert!(f.journal.unwrap().contains("1 citations rejetées"));
        // La troisième n'a pas été lue : elle reste floue ; les deux autres ne seront plus jamais redonnées
        let (v, _) = o.vue(Some(&b), Some(&cles));
        assert_eq!(v.floues, 1);
        let lue = v.liste.iter().find(|x| x.experience.is_some()).unwrap().id;
        let x = base_offres::lire(c, lue).unwrap().unwrap();
        assert_eq!(x.lecture.experience.unwrap().par, jobmind_core::lecture::LuPar::Ia);
        // Une seule offre, sur un clic : celle qui reste
        let f = o.geste(&ctx, LIRE_IA, &json!({ "id": lue }), m(9, 4));
        assert_eq!(f.message.as_deref(), Some("Aucune offre floue à lire : Jobmind a déjà tout compris."));
        // Un résultat arrivé après l'abandon : laissé
        let f = o.geste(&ctx, LIRE_IA, &json!({}), m(9, 5));
        let jeton = f.ia.unwrap().jeton;
        o.ia_trop_longue();
        assert!(
            o.ia_finie(Some(&b), RetourIa { jeton, resultats: Vec::new(), erreur: None }, m(9, 6)).message.is_none()
        );
        assert_eq!(cout_texte(150), "moins d'un centime");
        assert_eq!(cout_texte(25_000), "environ 3 centimes");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Contre-épreuve : des gestes mal formés ne changent rien et ne plantent pas.
    #[test]
    fn gestes_mal_formes() {
        let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
        let mut o = Offres::default();
        let ctx = Contexte { base: Some(&b), cles: None };
        for (id, v) in [
            (OUVRIR, json!({ "id": "1" })),
            (FAVORI, json!({ "id": 99 })),
            (CACHER, json!(null)),
            (CHERCHER, json!({ "id": 42 })),
            (RECHERCHE, json!({ "mots": "", "lieu": "ville" })),
            (RECHERCHE, json!({ "mots": "cariste", "lieu": "ville", "ville": "<script>", "rayon": 10 })),
            (RECHERCHE_SUPPRIMER, json!({ "id": -3 })),
            (POSTULE, json!({ "id": 99 })),
            (VOIR, json!({ "id": 99 })),
            (FILTRES, json!({ "genre": "region", "region": "Lorraine" })),
            (DEPUIS_PROFIL, json!(null)),
            (ESSAYER, json!({ "service": "google" })),
        ] {
            let f = o.geste(&ctx, id, &v, m(9, 0));
            assert!(!f.bravo && f.reseau.is_none() && f.voir.is_none() && f.candidature.is_none(), "{id} {v} {f:?}");
        }
        assert_eq!(base_offres::nombre_recherches(b.connexion()).unwrap(), 0);
        assert!(est_geste(CHERCHER) && est_geste(ESSAYER) && !est_geste("c-ouvrir"));
        // Le site d'Adzuna : toujours le même lien
        assert_eq!(o.geste(&ctx, ADZUNA, &Value::Null, m(9, 0)).voir, Some("https://www.adzuna.fr".into()));
        // Sans base
        let f = o.geste(&Contexte { base: None, cles: None }, OUVRIR, &json!({ "id": 1 }), m(9, 0));
        assert_eq!(f.message.as_deref(), Some(crate::suivi::SANS_BASE));
    }
}
