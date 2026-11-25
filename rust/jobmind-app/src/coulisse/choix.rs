//! 0.2.0 : les choix de Jordan sur les offres d'un site (demandes du 25/11/2026) : les doublons cachés (« la lecture
//! peut te permettre de voir les doublons, et du coup ne pas les afficher »), les filtres de la colonne de gauche
//! (distance, contrat, date de publication) et le tri des cartes. Tout se décide ici, sur ce que Jobmind sait déjà de
//! chaque offre analysée ([`Faits`], calculés une fois quand l'offre est lue), sans fenêtre ni réseau :
//!
//! - les doublons : la même annonce publiée deux fois (jobmind_core::offre::cle_meme_annonce : le même poste, la même
//!   entreprise, le même lieu et le même texte, remis au propre). La première dans l'ordre de la liste garde sa
//!   carte ; l'offre ouverte en ce moment garde toujours la sienne (pour « Offre précédente » et « Offre suivante ») ;
//! - la distance : entre la ville de la recherche ([`Depart`]) et le lieu de l'offre ([`LieuOffre`]). Elle n'est
//!   jamais inventée : connue seulement quand l'offre donne sa position et que le centre de la ville est connu (à vol
//!   d'oiseau), ou quand l'offre est dans la ville cherchée elle-même (« sur place ») ;
//! - le contrat : la règle des offres gardées (jobmind_core::offre::contrat_lu) ;
//! - la date : le jour de publication donné par l'annonce.
//!
//! Avec un filtre coché, une offre dont la valeur n'est pas connue (pas de distance, contrat pas reconnu, date pas
//! donnée) est cachée ; sans filtre, elle reste montrée.

use std::cmp::Ordering;
use std::collections::HashMap;

use jobmind_core::Jour;
use jobmind_core::offre::{cle_meme_annonce, cle_ville, contrat_lu, departement_lu, distance_km};
use jobmind_core::recherche::Contrat;
use jobmind_sources::OffreTrouvee;
use jobmind_sources::communes;
use jobmind_sources::sites::LieuSite;

/// Les distances proposées (en km), de la plus courte à la plus longue.
pub const DISTANCES: [u32; 3] = [5, 15, 30];
/// Les contrats proposés.
pub const CONTRATS: [Contrat; 4] = [Contrat::Cdi, Contrat::Cdd, Contrat::Interim, Contrat::Alternance];
/// Les anciennetés proposées (en jours depuis la publication).
pub const ANCIENNETES: [u32; 2] = [3, 7];

/// Le tri des cartes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TriSite {
    /// Dans l'ordre du site
    #[default]
    Site,
    /// Les plus récentes d'abord (les offres sans date à la fin)
    Recentes,
    /// Les plus près d'abord (les offres sans distance connue à la fin)
    Pres,
}

impl TriSite {
    pub const TOUS: [TriSite; 3] = [TriSite::Site, TriSite::Recentes, TriSite::Pres];

    pub fn code(self) -> &'static str {
        match self {
            TriSite::Site => "site",
            TriSite::Recentes => "recentes",
            TriSite::Pres => "pres",
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            TriSite::Site => "Dans l'ordre du site",
            TriSite::Recentes => "Les plus récentes d'abord",
            TriSite::Pres => "Les plus près d'abord",
        }
    }

    pub fn depuis_code(c: &str) -> Option<TriSite> {
        TriSite::TOUS.into_iter().find(|t| t.code() == c)
    }
}

/// Ce que Jordan a choisi pour les offres d'un site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choix {
    /// « Cacher les doublons » est coché (d'office)
    pub cacher_doublons: bool,
    /// « Jusqu'à 15 km » (rien : toutes les distances)
    pub distance: Option<u32>,
    /// Les contrats cochés (aucun : tous les contrats)
    pub contrats: Vec<Contrat>,
    /// « Depuis 7 jours » (rien : toutes les dates)
    pub jours: Option<u32>,
    pub tri: TriSite,
}

impl Default for Choix {
    fn default() -> Choix {
        Choix { cacher_doublons: true, distance: None, contrats: Vec::new(), jours: None, tri: TriSite::Site }
    }
}

impl Choix {
    /// Un filtre de la colonne de gauche est coché (distance, contrat ou date).
    pub fn filtre(&self) -> bool {
        self.distance.is_some() || !self.contrats.is_empty() || self.jours.is_some()
    }

    /// Une nouvelle recherche : les filtres et le tri repartent de zéro (« Cacher les doublons » reste comme Jordan
    /// l'a laissé, comme « Seulement les pas encore vues »).
    pub fn remettre(&mut self) {
        *self = Choix { cacher_doublons: self.cacher_doublons, ..Choix::default() };
    }

    /// L'offre passe les filtres cochés. Une valeur pas connue ne passe pas un filtre coché.
    pub fn garde(&self, faits: Option<&Faits>, distance: Option<Distance>, aujourdhui: Jour) -> bool {
        // (la distance arrondie, celle que la carte écrit : une carte « à 15 km » passe « Jusqu'à 15 km »)
        self.distance.is_none_or(|km| distance.is_some_and(|d| d.km().round() <= f64::from(km)))
            && (self.contrats.is_empty() || faits.and_then(|f| f.contrat).is_some_and(|c| self.contrats.contains(&c)))
            && self.jours.is_none_or(|n| {
                faits.and_then(|f| f.publiee).is_some_and(|j| j.jours_jusqua(aujourdhui) <= i64::from(n))
            })
    }
}

/// Le lieu d'une offre, comme il faut le connaître pour une distance.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LieuOffre {
    /// Sa ville remise au propre (jobmind_core::offre::cle_ville : « metz », « saint avold »)
    pub ville: String,
    /// Le code INSEE de sa commune, si le lieu la désigne sans doute possible (jobmind_sources::communes)
    pub insee: Option<&'static str>,
    /// Son département (« 57 »), s'il est dit
    pub departement: Option<&'static str>,
    /// Sa position (latitude, longitude) : celle que le site donne, sinon le centre de sa commune quand elle est
    /// reconnue sans doute possible (jobmind_sources::communes::centre)
    pub point: Option<(f64, f64)>,
}

/// La commune d'un lieu écrit par un site (« 57 - Metz », « Metz (57) », « 57000 Metz », « Metz, Grand Est, France ») :
/// le lieu entier, sinon ce qui vient avant la première virgule. Rien au moindre doute (plusieurs communes de ce nom).
fn commune_de(lieu: &str) -> Option<communes::Commune> {
    communes::trouver(lieu).or_else(|| communes::trouver(lieu.split(',').next().unwrap_or_default()))
}

impl LieuOffre {
    pub fn de(lieu: &str, zones: &[String], point: Option<(f64, f64)>) -> LieuOffre {
        let commune = commune_de(lieu);
        LieuOffre {
            ville: cle_ville(lieu),
            insee: commune.map(|c| c.insee),
            departement: departement_lu(lieu, zones).map(|d| d.code).or(commune.map(|c| c.departement())),
            point: point.or_else(|| commune.and_then(|c| communes::centre(c.insee))),
        }
    }
}

/// Ce que Jobmind retient d'une offre analysée pour les doublons, les filtres et le tri (calculé une fois, quand
/// l'offre est lue ou retrouvée dans la base).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Faits {
    /// Le contrat reconnu (rien : pas reconnu)
    pub contrat: Option<Contrat>,
    /// Le jour de publication (rien : pas donné)
    pub publiee: Option<Jour>,
    /// La clé de l'annonce pour reconnaître ses doublons (rien : jamais un doublon)
    pub annonce: Option<String>,
    pub lieu: LieuOffre,
}

impl Faits {
    pub fn de(o: &OffreTrouvee) -> Faits {
        Faits {
            contrat: contrat_lu(&o.contrat, &o.titre, &o.description),
            publiee: Jour::lire_iso(&o.publiee_le),
            annonce: cle_meme_annonce(&o.titre, &o.entreprise, &o.lieu, &o.description),
            lieu: LieuOffre::de(&o.lieu, &o.zones, o.latitude.zip(o.longitude)),
        }
    }
}

/// Le point de départ des distances : la ville de la recherche.
#[derive(Debug, Clone, PartialEq)]
pub struct Depart {
    /// La ville cherchée remise au propre (« metz »)
    pub ville: String,
    pub insee: Option<&'static str>,
    pub departement: Option<&'static str>,
    /// Le centre de la ville, s'il est connu (jobmind_core::offre::centre_ville : la position médiane des offres
    /// gardées qui y sont)
    pub centre: Option<(f64, f64)>,
}

impl Depart {
    /// Le départ d'une recherche : seulement autour d'une ville (rien pour un département, une région, partout).
    pub fn de(lieu: &LieuSite, centre: Option<(f64, f64)>) -> Option<Depart> {
        let LieuSite::Ville { nom, .. } = lieu else { return None };
        let l = LieuOffre::de(nom, &[], None);
        // (0.2.0 : le centre de la commune cherchée, quand les offres gardées ne le donnent pas)
        let centre = centre.or(l.point);
        (!l.ville.is_empty()).then_some(Depart { ville: l.ville, insee: l.insee, departement: l.departement, centre })
    }

    /// L'offre est dans la ville cherchée elle-même : la même commune quand les deux sont reconnues ; sinon le même
    /// nom de ville, sauf si les deux départements sont dits et différents (deux villes du même nom).
    fn meme_ville(&self, l: &LieuOffre) -> bool {
        match (self.insee, l.insee) {
            (Some(a), Some(b)) => a == b,
            _ => {
                !self.ville.is_empty()
                    && self.ville == l.ville
                    && !matches!((self.departement, l.departement), (Some(a), Some(b)) if a != b)
            }
        }
    }

    /// La distance jusqu'à ce lieu, si elle est connue : jamais inventée.
    pub fn distance(&self, l: &LieuOffre) -> Option<Distance> {
        // (dans la ville cherchée : « sur place », même si les deux positions sont connues)
        if self.meme_ville(l) {
            return Some(Distance::SurPlace);
        }
        match (self.centre, l.point) {
            (Some(c), Some(p)) => Some(Distance::Km(distance_km(c, p))),
            _ => None,
        }
    }
}

/// Une distance connue.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Distance {
    /// Dans la ville cherchée elle-même (sa position exacte n'est pas connue)
    SurPlace,
    /// À vol d'oiseau, du centre de la ville cherchée à la position donnée par l'offre
    Km(f64),
}

impl Distance {
    pub fn km(self) -> f64 {
        match self {
            Distance::SurPlace => 0.0,
            Distance::Km(d) => d,
        }
    }

    /// Ce que la carte écrit : « sur place », « à 12 km ».
    pub fn texte(self) -> String {
        match self {
            Distance::SurPlace => "sur place".to_string(),
            Distance::Km(d) if d < 1.0 => "à moins de 1 km".to_string(),
            Distance::Km(d) => format!("à {} km", d.round() as i64),
        }
    }
}

/// Où en est l'analyse d'une offre de la liste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Analyse {
    /// Analysée : elle passe, ou non, « Aucun diplôme requis » et « Débutant accepté »
    Faite { passe: bool },
    /// Pas analysée, mais montrée pour que Jordan l'ouvre lui-même (illisible, ou lecture arrêtée par le site)
    AOuvrir,
    /// Pas encore analysée : pas de carte
    Attendue,
}

/// Ce qu'il faut savoir d'une offre de la liste pour décider de sa carte (une par résultat, dans l'ordre de la liste).
#[derive(Debug, Clone)]
pub struct Ligne<'a> {
    pub retiree: bool,
    /// C'est l'offre ouverte en ce moment
    pub ouverte: bool,
    /// Jordan a déjà ouvert cette offre lui-même
    pub vue: bool,
    pub analyse: Analyse,
    pub faits: Option<&'a Faits>,
    pub distance: Option<Distance>,
    /// Son groupe de doublons : le rang de la première annonce identique dans la liste (elle-même si c'est elle) ;
    /// rien : elle n'a pas de doublon possible (posé par [`grouper`])
    pub groupe: Option<usize>,
    /// Le nombre d'annonces de son groupe (1 : pas de doublon)
    pub taille: usize,
    /// Jordan a déjà ouvert une annonce de son groupe (« Déjà vue » se partage entre doublons)
    pub vue_groupe: bool,
}

impl<'a> Ligne<'a> {
    pub fn nouvelle(
        retiree: bool,
        ouverte: bool,
        vue: bool,
        analyse: Analyse,
        faits: Option<&'a Faits>,
        distance: Option<Distance>,
    ) -> Ligne<'a> {
        Ligne { retiree, ouverte, vue, analyse, faits, distance, groupe: None, taille: 1, vue_groupe: vue }
    }

    /// Une copie : une annonce identique à une autre, placée avant elle dans la liste.
    pub fn copie(&self, rang: usize) -> bool {
        self.groupe.is_some_and(|g| g != rang)
    }
}

/// Pose les groupes de doublons : les offres analysées, pas retirées, dont la clé d'annonce est la même.
pub fn grouper(lignes: &mut [Ligne]) {
    let mut premiers: HashMap<&str, usize> = HashMap::new();
    for (i, x) in lignes.iter_mut().enumerate() {
        if x.retiree || !matches!(x.analyse, Analyse::Faite { .. }) {
            continue;
        }
        let Some(k) = x.faits.and_then(|f| f.annonce.as_deref()) else { continue };
        x.groupe = Some(*premiers.entry(k).or_insert(i));
    }
    let mut tailles: HashMap<usize, (usize, bool)> = HashMap::new();
    for x in lignes.iter() {
        if let Some(g) = x.groupe {
            let t = tailles.entry(g).or_insert((0, false));
            t.0 += 1;
            t.1 |= x.vue;
        }
    }
    for x in lignes.iter_mut() {
        if let Some((n, vue)) = x.groupe.and_then(|g| tailles.get(&g)) {
            x.taille = *n;
            x.vue_groupe = *vue;
        }
    }
}

/// Les réglages qui décident des cartes, en plus des [`Choix`].
#[derive(Debug, Clone, Copy)]
pub struct Regles<'a> {
    pub choix: &'a Choix,
    /// « Seulement les pas encore vues »
    pub pas_vues: bool,
    /// « Revoir les offres retirées »
    pub voir_retirees: bool,
    /// « Aucun diplôme requis » ou « Débutant accepté » est coché
    pub filtre_lecture: bool,
    pub aujourdhui: Jour,
}

/// Les cartes montrées, et le nombre de doublons cachés parmi les offres qui passent les filtres.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cartes {
    /// Les rangs dans les résultats, dans l'ordre des cartes
    pub rangs: Vec<usize>,
    pub doublons_caches: usize,
}

/// Compare deux valeurs dont certaines manquent : celles qui manquent à la fin.
fn manquantes_a_la_fin<T>(a: Option<T>, b: Option<T>, ordre: impl Fn(&T, &T) -> Ordering) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => ordre(&x, &y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Les cartes à montrer pour ces offres (déjà groupées par [`grouper`]) : les offres analysées qui passent tous les
/// filtres ; celles à ouvrir soi-même quand aucun filtre n'est coché ; les retirées seulement avec « Revoir les offres
/// retirées » ; sans les offres déjà vues avec « Seulement les pas encore vues » (sauf celle ouverte en ce moment) ;
/// un seul doublon par groupe avec « Cacher les doublons » (l'offre ouverte, sinon la première de la liste qui passe
/// les filtres) ; puis le tri (à égalité, l'ordre du site).
pub fn cartes(lignes: &[Ligne], r: &Regles) -> Cartes {
    let passe = |x: &Ligne| {
        if x.retiree {
            return r.voir_retirees;
        }
        if r.pas_vues && !x.ouverte && x.vue_groupe {
            return false;
        }
        match x.analyse {
            Analyse::Faite { passe } => passe && r.choix.garde(x.faits, x.distance, r.aujourdhui),
            Analyse::AOuvrir => !r.filtre_lecture && !r.choix.filtre(),
            Analyse::Attendue => false,
        }
    };
    let passent: Vec<usize> = (0..lignes.len()).filter(|i| passe(&lignes[*i])).collect();
    let mut rangs = passent.clone();
    if r.choix.cacher_doublons {
        // (la carte gardée de chaque groupe : l'offre ouverte si elle en fait partie, sinon la première qui passe)
        let mut gardee: HashMap<usize, usize> = HashMap::new();
        for &i in &passent {
            if let Some(g) = lignes[i].groupe {
                let e = gardee.entry(g).or_insert(i);
                if lignes[i].ouverte {
                    *e = i;
                }
            }
        }
        rangs.retain(|i| lignes[*i].groupe.is_none_or(|g| gardee.get(&g) == Some(i)));
    }
    let doublons_caches = passent.len() - rangs.len();
    match r.choix.tri {
        TriSite::Site => {}
        TriSite::Recentes => rangs.sort_by(|a, b| {
            let jour = |i: &usize| lignes[*i].faits.and_then(|f| f.publiee);
            manquantes_a_la_fin(jour(a), jour(b), |x, y| y.cmp(x))
        }),
        TriSite::Pres => rangs.sort_by(|a, b| {
            let km = |i: &usize| lignes[*i].distance.map(Distance::km);
            manquantes_a_la_fin(km(a), km(b), f64::total_cmp)
        }),
    }
    Cartes { rangs, doublons_caches }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auj() -> Jour {
        Jour::nouveau(2026, 11, 19).unwrap()
    }

    const TEXTE: &str = "Conduite de chariot en équipe de jour, réception des livraisons, rangement des palettes, \
                         inventaires tournants. Débutant accepté.";

    fn offre(titre: &str, lieu: &str, contrat: &str, publiee: &str) -> OffreTrouvee {
        OffreTrouvee {
            titre: titre.into(),
            entreprise: "Logipôle Lorraine".into(),
            lieu: lieu.into(),
            description: TEXTE.into(),
            contrat: contrat.into(),
            publiee_le: publiee.into(),
            ..OffreTrouvee::default()
        }
    }

    fn metz(centre: Option<(f64, f64)>) -> Depart {
        Depart::de(&LieuSite::Ville { nom: "Metz (57000)".into(), rayon_km: 30 }, centre).unwrap()
    }

    /// Ce que Jobmind retient d'une offre : le contrat par la règle des offres gardées, le jour, la clé de l'annonce.
    #[test]
    fn faits_d_une_offre() {
        let f = Faits::de(&offre("Cariste H/F", "57 - Metz", "Contrat à durée indéterminée", "2026-11-17"));
        assert_eq!(f.contrat, Some(Contrat::Cdi));
        assert_eq!(f.publiee, Jour::nouveau(2026, 11, 17));
        assert!(f.annonce.is_some());
        assert_eq!((f.lieu.ville.as_str(), f.lieu.insee, f.lieu.departement), ("metz", Some("57463"), Some("57")));
        // (le site ne donne pas de position : c'est le centre de la commune reconnue, ici Metz)
        assert_eq!(f.lieu.point, Some((49.1196, 6.1764)));
        assert_eq!(f.lieu.point, communes::centre("57463"));
        // (un lieu qui n'est pas une commune reconnue sans doute possible : pas de position, jamais inventée)
        for lieu in ["", "Moselle", "Grand Est", "France", "Télétravail", "Saint-Denis", "Ennery"] {
            let l = Faits::de(&offre("Cariste H/F", lieu, "CDI", "2026-11-17")).lieu;
            assert_eq!((l.insee, l.point), (None, None), "{lieu}");
        }
        for (dit, titre, attendu) in [
            ("CDI", "Cariste", Some(Contrat::Cdi)),
            ("permanent", "Cariste", Some(Contrat::Cdi)),
            ("CDD", "Cariste", Some(Contrat::Cdd)),
            ("Mission intérimaire - 3 Mois", "Cariste", Some(Contrat::Interim)),
            ("", "Cariste en alternance", Some(Contrat::Alternance)),
            ("Stage", "Assistant logistique", Some(Contrat::Stage)),
            ("Temps plein", "Cariste", None),
        ] {
            let mut o = offre(titre, "57 - Metz", dit, "");
            o.description = "Poste à pourvoir vite.".into();
            let f = Faits::de(&o);
            assert_eq!(f.contrat, attendu, "{dit} / {titre}");
            assert_eq!((f.publiee, f.annonce), (None, None), "pas de date, un texte trop court");
        }
        // (une position donnée à moitié n'en est pas une : sans commune reconnue, pas de position du tout)
        let mut o = offre("Cariste", "Moselle", "CDI", "pas une date");
        o.latitude = Some(49.1);
        assert_eq!((Faits::de(&o).lieu.point, Faits::de(&o).publiee), (None, None));
        // (et dans une commune reconnue, c'est le centre de la commune, pas la moitié de position)
        o.lieu = "Metz".into();
        assert_eq!((Faits::de(&o).lieu.point, Faits::de(&o).publiee), (Some((49.1196, 6.1764)), None));
        // (la position entière donnée par le site passe avant le centre de la commune)
        o.longitude = Some(6.2);
        assert_eq!(Faits::de(&o).lieu.point, Some((49.1, 6.2)));
    }

    /// La distance n'est jamais inventée : à vol d'oiseau entre le centre de la ville cherchée et la position de
    /// l'offre (celle du site, sinon le centre de sa commune reconnue), « sur place » dans la ville cherchée, rien
    /// quand le lieu n'est pas une commune reconnue.
    #[test]
    fn distance_connue_ou_pas() {
        let (centre, woippy) = ((49.1193, 6.1757), (49.1519, 6.1514));
        let lieu = |t: &str, point| LieuOffre::de(t, &[], point);
        // Le centre du départ : celui tiré des offres gardées, sinon le centre de la commune cherchée
        assert_eq!(metz(Some(centre)).centre, Some(centre));
        assert_eq!(metz(None).centre, Some((49.1196, 6.1764)));
        assert_eq!(metz(None).centre, communes::centre("57463"));
        // Le centre et la position donnée par le site : la vraie distance (Woippy est à 4 km de Metz)
        let d = metz(Some(centre)).distance(&lieu("57 - Woippy", Some(woippy))).unwrap();
        assert!(matches!(d, Distance::Km(x) if (3.0..5.0).contains(&x)), "{d:?}");
        assert_eq!(d.texte(), "à 4 km");
        // Dans la ville cherchée : toujours « sur place », même quand les deux positions sont connues (au centre
        // même, ou à l'autre bout de la commune)
        assert_eq!(metz(Some(centre)).distance(&lieu("57 - Metz", Some(centre))), Some(Distance::SurPlace));
        assert_eq!(metz(Some(centre)).distance(&lieu("57 - Metz", Some((49.09, 6.23)))), Some(Distance::SurPlace));
        assert_eq!(metz(None).distance(&lieu("57 - Metz", Some((49.09, 6.23)))), Some(Distance::SurPlace));
        assert_eq!(Distance::Km(0.4).texte(), "à moins de 1 km");
        assert_eq!(Distance::Km(12.4).texte(), "à 12 km");
        // Sans position : « sur place », quelle que soit l'écriture du site
        for t in
            ["57 - METZ", "Metz (57)", "57000 Metz", "Metz, Grand Est, France", "Metz", "Metz - 57", "metz cedex 1"]
        {
            assert_eq!(metz(None).distance(&lieu(t, None)), Some(Distance::SurPlace), "{t}");
            assert_eq!(metz(Some(centre)).distance(&lieu(t, None)), Some(Distance::SurPlace), "{t}");
        }
        assert_eq!((Distance::SurPlace.km(), Distance::SurPlace.texte().as_str()), (0.0, "sur place"));
        // Ailleurs sans position, dans une commune reconnue : la distance entre les deux centres, à vol d'oiseau
        // (kilomètres depuis la mairie de Metz, puis depuis le centre tiré des offres gardées, à 60 m de là)
        for (t, de_la_commune, des_offres, ecrit) in [
            ("57 - Woippy", 3.97, 3.97, "à 4 km"),
            ("54 - Nancy", 47.45, 47.42, "à 47 km"),
            ("Montigny-lès-Metz", 2.77, 2.71, "à 3 km"),
            ("Metzervisse (57)", 22.77, 22.82, "à 23 km"),
            ("57 - Thionville", 26.48, 26.51, "à 26 km"),
            ("Sarrebourg (57400)", 77.28, 77.30, "à 77 km"),
        ] {
            let a = metz(None).distance(&lieu(t, None)).unwrap();
            let b = metz(Some(centre)).distance(&lieu(t, None)).unwrap();
            assert!(matches!(a, Distance::Km(x) if (x - de_la_commune).abs() < 0.05), "{t} : {a:?}");
            assert!(matches!(b, Distance::Km(x) if (x - des_offres).abs() < 0.05), "{t} : {b:?}");
            assert_eq!(a.texte(), ecrit, "{t}");
        }
        // (le centre tiré des offres gardées passe avant celui de la commune cherchée : d'un centre placé à 15 km au
        // sud-ouest de Metz, Woippy est à 20 km, pas à 4)
        let d = metz(Some((49.0, 6.0))).distance(&lieu("57 - Woippy", None)).unwrap();
        assert!(matches!(d, Distance::Km(x) if (x - 20.07).abs() < 0.05), "{d:?}");
        // (la position donnée par le site passe avant le centre de la commune de l'offre : 7 km, pas 4)
        let d = metz(None).distance(&lieu("57 - Woippy", Some((49.18, 6.15)))).unwrap();
        assert!(matches!(d, Distance::Km(x) if (x - 6.99).abs() < 0.05), "{d:?}");
        assert_eq!(d.texte(), "à 7 km");
        let d = metz(None).distance(&lieu("57 - Woippy", Some(woippy))).unwrap();
        assert!(matches!(d, Distance::Km(x) if (x - 4.03).abs() < 0.05), "{d:?}");
        // Un lieu qui n'est pas une commune reconnue sans doute possible (une région, un département, rien, du
        // télétravail, deux communes du même nom sans leur département) : pas de distance
        for t in ["Grand Est", "France", "", "Moselle", "Télétravail", "Saint-Denis", "Ennery"] {
            assert_eq!(metz(None).distance(&lieu(t, None)), None, "{t}");
            assert_eq!(metz(Some(centre)).distance(&lieu(t, None)), None, "{t}");
        }
        // Une position mais pas de centre connu (la ville cherchée n'est pas reconnue) : pas de distance
        let sans_centre = Depart::de(&LieuSite::Ville { nom: "Saint-Denis".into(), rayon_km: 10 }, None).unwrap();
        assert_eq!((sans_centre.insee, sans_centre.centre), (None, None));
        assert_eq!(sans_centre.distance(&lieu("57 - Woippy", Some(woippy))), None);
        assert_eq!(sans_centre.distance(&lieu("57 - Woippy", None)), None);
        assert_eq!(sans_centre.distance(&lieu("Saint-Denis", None)), Some(Distance::SurPlace));
        // Deux villes du même nom : jamais « sur place » quand les départements sont dits et différents (les deux
        // communes sont reconnues : c'est la vraie distance, de la Seine-Saint-Denis à La Réunion)
        let saint_denis =
            Depart::de(&LieuSite::Ville { nom: "Saint-Denis (93200)".into(), rayon_km: 10 }, None).unwrap();
        assert_eq!((saint_denis.insee, saint_denis.departement), (Some("93066"), Some("93")));
        assert_eq!(saint_denis.centre, Some((48.9362, 2.3584)));
        assert_eq!(saint_denis.distance(&lieu("93 - Saint-Denis", None)), Some(Distance::SurPlace));
        assert_eq!(saint_denis.distance(&lieu("St-Denis (93)", None)), Some(Distance::SurPlace));
        for t in ["974 - Saint-Denis", "Saint-Denis (974)"] {
            let d = saint_denis.distance(&lieu(t, None)).unwrap();
            assert!(matches!(d, Distance::Km(x) if (x - 9370.1).abs() < 1.0), "{t} : {d:?}");
        }
        // (le lieu ne dit pas lequel : le site a cherché autour de la ville demandée, c'est elle)
        assert_eq!(saint_denis.distance(&lieu("Saint-Denis", None)), Some(Distance::SurPlace));
        // Une recherche sans ville n'a pas de départ
        assert_eq!(Depart::de(&LieuSite::Partout, None), None);
        assert_eq!(Depart::de(&LieuSite::Departement { code: "57".into(), nom: "Moselle".into() }, None), None);
        assert_eq!(Depart::de(&LieuSite::Region { code: "grand_est".into(), nom: "Grand Est".into() }, None), None);
        assert_eq!(Depart::de(&LieuSite::Ville { nom: " ".into(), rayon_km: 30 }, None), None);
    }

    fn regles(choix: &Choix) -> Regles<'_> {
        Regles { choix, pas_vues: false, voir_retirees: false, filtre_lecture: false, aujourdhui: auj() }
    }

    fn analysee(f: &Faits, distance: Option<Distance>) -> Ligne<'_> {
        Ligne::nouvelle(false, false, false, Analyse::Faite { passe: true }, Some(f), distance)
    }

    /// Les filtres : chacun seul, croisés, et les valeurs pas connues (cachées seulement par un filtre coché).
    #[test]
    fn filtres_et_valeurs_inconnues() {
        let cdi = Faits::de(&offre("Cariste", "57 - Metz", "CDI", "2026-11-18"));
        let cdd = Faits::de(&offre("Agent de quai", "57 - Woippy", "CDD", "2026-11-14"));
        let interim = Faits::de(&offre("Manutentionnaire", "57 - Ennery", "Intérim", "2026-11-01"));
        let mut inconnue = Faits::de(&offre("Magasinier", "Grand Est", "", ""));
        inconnue.contrat = None;
        let lignes = vec![
            analysee(&cdi, Some(Distance::SurPlace)),
            analysee(&cdd, Some(Distance::Km(4.2))),
            analysee(&interim, Some(Distance::Km(14.0))),
            analysee(&inconnue, None),
            // (analysée, mais sans ses faits : rien n'est connu)
            Ligne::nouvelle(false, false, false, Analyse::Faite { passe: true }, None, None),
        ];
        let avec = |c: Choix| cartes(&lignes, &regles(&c)).rangs;
        assert_eq!(avec(Choix::default()), [0, 1, 2, 3, 4], "sans filtre, tout est montré");
        assert!(!Choix::default().filtre());
        // La distance
        assert_eq!(avec(Choix { distance: Some(5), ..Choix::default() }), [0, 1]);
        assert_eq!(avec(Choix { distance: Some(15), ..Choix::default() }), [0, 1, 2]);
        assert_eq!(avec(Choix { distance: Some(30), ..Choix::default() }), [0, 1, 2]);
        // (à la limite, c'est la distance écrite sur la carte, arrondie, qui décide : « à 15 km » passe « Jusqu'à
        // 15 km », « à 16 km » non)
        let jusqua = |limite: u32, d: Distance| {
            Choix { distance: Some(limite), ..Choix::default() }.garde(Some(&cdi), Some(d), auj())
        };
        for (limite, km, ecrit, gardee) in [
            (15, 15.0, "à 15 km", true),
            (15, 15.4, "à 15 km", true),
            (15, 15.49, "à 15 km", true),
            (15, 15.5, "à 16 km", false),
            (15, 15.6, "à 16 km", false),
            (5, 5.4, "à 5 km", true),
            (5, 5.6, "à 6 km", false),
            (30, 30.4, "à 30 km", true),
            (30, 30.6, "à 31 km", false),
            (5, 0.4, "à moins de 1 km", true),
        ] {
            assert_eq!(Distance::Km(km).texte(), ecrit, "{km}");
            assert_eq!(jusqua(limite, Distance::Km(km)), gardee, "{km} km, jusqu'à {limite} km");
        }
        assert!(DISTANCES.into_iter().all(|limite| jusqua(limite, Distance::SurPlace)));
        // Le contrat (plusieurs à la fois)
        assert_eq!(avec(Choix { contrats: vec![Contrat::Cdi], ..Choix::default() }), [0]);
        assert_eq!(avec(Choix { contrats: vec![Contrat::Cdd, Contrat::Interim], ..Choix::default() }), [1, 2]);
        assert!(avec(Choix { contrats: vec![Contrat::Alternance], ..Choix::default() }).is_empty());
        // La date (le 19 : depuis 3 jours, à partir du 16)
        assert_eq!(avec(Choix { jours: Some(3), ..Choix::default() }), [0]);
        assert_eq!(avec(Choix { jours: Some(7), ..Choix::default() }), [0, 1]);
        // Croisés
        let c = Choix { distance: Some(15), contrats: vec![Contrat::Cdd, Contrat::Interim], ..Choix::default() };
        assert!(c.filtre());
        assert_eq!(avec(c.clone()), [1, 2]);
        assert_eq!(avec(Choix { jours: Some(7), ..c }), [1]);
        // « Aucun diplôme requis » qui ne passe pas, une offre pas encore analysée : pas de carte
        let mut l2 = lignes.clone();
        l2[0].analyse = Analyse::Faite { passe: false };
        l2[1].analyse = Analyse::Attendue;
        assert_eq!(cartes(&l2, &regles(&Choix::default())).rangs, [2, 3, 4]);
        // Une offre à ouvrir soi-même : montrée seulement sans aucun filtre
        l2[1].analyse = Analyse::AOuvrir;
        assert_eq!(cartes(&l2, &regles(&Choix::default())).rangs, [1, 2, 3, 4]);
        let c = Choix { distance: Some(30), ..Choix::default() };
        assert_eq!(cartes(&l2, &regles(&c)).rangs, [2]);
        let sans = Choix::default();
        let r = Regles { filtre_lecture: true, ..regles(&sans) };
        assert_eq!(cartes(&l2, &r).rangs, [2, 3, 4]);
        // Une nouvelle recherche remet les filtres et le tri, pas « Cacher les doublons »
        let mut c = Choix {
            cacher_doublons: false,
            distance: Some(5),
            contrats: vec![Contrat::Cdi],
            jours: Some(3),
            tri: TriSite::Pres,
        };
        c.remettre();
        assert_eq!(c, Choix { cacher_doublons: false, ..Choix::default() });
    }

    /// Le tri : à égalité l'ordre du site, les valeurs pas connues à la fin.
    #[test]
    fn tri_des_cartes() {
        let f = |jour: &str| Faits::de(&offre("Cariste", "57 - Metz", "CDI", jour));
        let (a, b, c, d) = (f("2026-11-10"), f(""), f("2026-11-18"), f("2026-11-10"));
        let mut lignes = vec![
            analysee(&a, Some(Distance::Km(12.0))),
            analysee(&b, Some(Distance::SurPlace)),
            analysee(&c, None),
            analysee(&d, Some(Distance::Km(3.0))),
        ];
        // (les mêmes textes : sans les groupes de doublons, chaque offre garde sa carte)
        let avec = |l: &[Ligne], tri| cartes(l, &regles(&Choix { tri, ..Choix::default() })).rangs;
        assert_eq!(avec(&lignes, TriSite::Site), [0, 1, 2, 3]);
        assert_eq!(avec(&lignes, TriSite::Recentes), [2, 0, 3, 1]);
        assert_eq!(avec(&lignes, TriSite::Pres), [1, 3, 0, 2]);
        lignes[1].distance = None;
        assert_eq!(avec(&lignes, TriSite::Pres), [3, 0, 1, 2]);
        assert_eq!(TriSite::depuis_code("pres"), Some(TriSite::Pres));
        assert_eq!(TriSite::depuis_code("recentes"), Some(TriSite::Recentes));
        assert_eq!(TriSite::depuis_code("site"), Some(TriSite::Site));
        assert_eq!(TriSite::depuis_code("autre"), None);
        assert_eq!(TriSite::default(), TriSite::Site);
    }

    /// Les doublons : la première de la liste garde sa carte, les copies sont cachées (ou montrées, décoché) ;
    /// l'offre ouverte garde toujours la sienne ; « Déjà vue » se partage ; les retirées restent à part.
    #[test]
    fn doublons_caches_ou_montres() {
        let a = Faits::de(&offre("Cariste", "57 - Metz", "CDI", "2026-11-18"));
        let b = Faits::de(&offre("Agent de quai", "57 - Metz", "CDI", "2026-11-10"));
        // (la même annonce que `a`, publiée un autre jour)
        let a2 = Faits::de(&offre("CARISTE", "57 - METZ", "CDI", "2026-11-11"));
        assert_eq!(a.annonce, a2.annonce);
        assert_ne!(a.annonce, b.annonce);
        let neuves = || {
            vec![
                analysee(&a, None),
                analysee(&b, None),
                analysee(&a2, None),
                analysee(&a, None),
                // (retirée : jamais dans un groupe)
                Ligne::nouvelle(true, false, false, Analyse::Faite { passe: true }, Some(&a), None),
                // (pas analysée : jamais dans un groupe)
                Ligne::nouvelle(false, false, false, Analyse::AOuvrir, None, None),
            ]
        };
        let mut l = neuves();
        grouper(&mut l);
        assert_eq!(l.iter().map(|x| x.groupe).collect::<Vec<_>>(), [Some(0), Some(1), Some(0), Some(0), None, None]);
        assert_eq!(l.iter().map(|x| x.taille).collect::<Vec<_>>(), [3, 1, 3, 3, 1, 1]);
        assert_eq!((0..6).map(|i| l[i].copie(i)).collect::<Vec<_>>(), [false, false, true, true, false, false]);
        let cachees = Choix::default();
        let c = cartes(&l, &regles(&cachees));
        assert_eq!((c.rangs, c.doublons_caches), (vec![0, 1, 5], 2));
        let montrees = Choix { cacher_doublons: false, ..Choix::default() };
        let c = cartes(&l, &regles(&montrees));
        assert_eq!((c.rangs, c.doublons_caches), (vec![0, 1, 2, 3, 5], 0));
        // Les retirées remontrées : chacune sa carte, sans compter dans les doublons
        let c = cartes(&l, &Regles { voir_retirees: true, ..regles(&cachees) });
        assert_eq!((c.rangs, c.doublons_caches), (vec![0, 1, 4, 5], 2));
        // Un filtre cache la première : la copie qui passe garde la carte (une annonce n'est jamais perdue)
        let depuis_7 = Choix { jours: Some(7), ..Choix::default() };
        let c = cartes(&l, &regles(&depuis_7));
        assert_eq!(
            (c.rangs, c.doublons_caches),
            (vec![0], 1),
            "la première et sa copie du 18 passent, pas celle du 11"
        );
        let mut l2 = neuves();
        l2.swap(0, 2);
        grouper(&mut l2);
        let c = cartes(&l2, &regles(&Choix { jours: Some(3), ..Choix::default() }));
        assert_eq!((c.rangs, c.doublons_caches), (vec![2], 1), "la première (du 11) ne passe pas : sa copie du 18");
        // L'offre ouverte garde sa carte, même si c'est une copie
        let mut l = neuves();
        l[3].ouverte = true;
        l[3].vue = true;
        l[3].vue_groupe = true;
        grouper(&mut l);
        let c = cartes(&l, &regles(&cachees));
        assert_eq!((c.rangs, c.doublons_caches), (vec![1, 3, 5], 2));
        // « Déjà vue » est partagé par tout le groupe
        assert_eq!(l.iter().map(|x| x.vue_groupe).collect::<Vec<_>>(), [true, false, true, true, false, false]);
        // « Seulement les pas encore vues » : tout le groupe s'en va, sauf l'offre ouverte tant qu'elle l'est
        let c = cartes(&l, &Regles { pas_vues: true, ..regles(&cachees) });
        assert_eq!((c.rangs, c.doublons_caches), (vec![1, 3, 5], 0));
        let c = cartes(&l, &Regles { pas_vues: true, ..regles(&montrees) });
        assert_eq!(c.rangs, [1, 3, 5]);
        l[3].ouverte = false;
        let c = cartes(&l, &Regles { pas_vues: true, ..regles(&cachees) });
        assert_eq!((c.rangs, c.doublons_caches), (vec![1, 5], 0));
        // Le tri s'applique aux cartes gardées
        let mut l = neuves();
        grouper(&mut l);
        let c = cartes(&l, &regles(&Choix { tri: TriSite::Recentes, ..Choix::default() }));
        assert_eq!(c.rangs, [0, 1, 5]);
    }
}
