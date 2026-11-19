//! Les offres (étape 4 du plan) : ce que Jobmind lit dans une offre venue d'une source (le contrat, le télétravail, le
//! département), la règle des doublons (la même offre venue des deux sites, ou deux fois du même), et les filtres de
//! l'onglet Offres (lieu, contrat, télétravail, favoris et « plus tard », offres cachées). Le texte libre est cherché
//! par la base (FTS5) ; ici, seulement le résultat.
//!
//! La règle des doublons ([`empreinte`], [`meme_offre`]) : deux offres sont la même si l'entreprise, le titre et la
//! ville, remis au propre (sans accents ni majuscules, sans « H/F », sans « CDI » dans le titre, sans « SAS » dans le
//! nom, sans code postal), sont les mêmes. Jamais sans entreprise (deux « Cariste » à Metz d'entreprises cachées ne sont
//! pas forcément la même offre). Deux offres du même site, sous deux numéros, ne sont fusionnées que si leurs textes se
//! ressemblent aussi (une entreprise peut chercher deux caristes pour deux équipes).

use std::collections::HashSet;

use crate::Jour;
use crate::candidature::une_ligne;
use crate::lecture::{Diplome, Experience};
use crate::lieux::{self, Departement, Region};
use crate::pertinence::Niveau;
use crate::recherche::{Contrat, GenreLieu, RAYON_MAX, Teletravail};

/// Un texte remis au propre pour comparer : minuscules, sans accents, des lettres et des chiffres séparés par une
/// espace.
pub fn normaliser(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    for c in t.chars().flat_map(char::to_lowercase) {
        let r: &str = match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => "a",
            'é' | 'è' | 'ê' | 'ë' => "e",
            'î' | 'ï' | 'í' | 'ì' => "i",
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => "o",
            'ù' | 'û' | 'ü' | 'ú' => "u",
            'ç' => "c",
            'ÿ' => "y",
            'ñ' => "n",
            'œ' => "oe",
            'æ' => "ae",
            c if c.is_ascii_alphanumeric() => {
                s.push(c);
                continue;
            }
            c if c.is_alphanumeric() => {
                s.push(c);
                continue;
            }
            _ => " ",
        };
        s.push_str(r);
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Les mots d'un titre qui ne disent rien du poste (« H/F », le contrat).
const MOTS_VIDES_TITRE: [&str; 11] = ["h", "f", "hf", "fh", "m", "x", "cdi", "cdd", "interim", "urgent", "nouveau"];
/// Les formes de société, retirées du nom.
const FORMES: [&str; 8] = ["sas", "sasu", "sarl", "sa", "eurl", "snc", "scop", "sca"];

fn sans(t: &str, mots: &[&str]) -> String {
    normaliser(t).split(' ').filter(|m| !m.is_empty() && !mots.contains(m)).collect::<Vec<_>>().join(" ")
}

/// Le titre pour comparer : « Agent de quai (H/F) - CDI » : « agent de quai ».
pub fn cle_titre(t: &str) -> String {
    sans(t, &MOTS_VIDES_TITRE)
}

/// L'entreprise pour comparer : « ENTREPÔTS DU VAL DE MOSELLE SAS » : « entrepots du val de moselle ».
pub fn cle_entreprise(e: &str) -> String {
    sans(e, &FORMES)
}

/// La ville d'un lieu (« Metz, Moselle », « Metz (57) », « 57000 Metz Cedex », « St-Avold ») : « metz », « saint avold ».
pub fn cle_ville(lieu: &str) -> String {
    let premier = lieu.split([',', '(']).next().unwrap_or_default();
    normaliser(premier)
        .split(' ')
        .filter(|m| !m.is_empty() && !m.chars().all(|c| c.is_ascii_digit()) && *m != "cedex")
        .map(|m| match m {
            "st" => "saint",
            "ste" => "sainte",
            m => m,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// L'empreinte d'une offre pour trouver ses doublons : rien sans entreprise, sans titre ou sans ville.
pub fn empreinte(entreprise: &str, titre: &str, lieu: &str) -> Option<String> {
    let (e, t, v) = (cle_entreprise(entreprise), cle_titre(titre), cle_ville(lieu));
    (!e.is_empty() && !t.is_empty() && !v.is_empty()).then(|| format!("{e}|{t}|{v}"))
}

/// Deux textes d'offre se ressemblent : la moitié de leurs mots (de 3 lettres ou plus) au moins en commun. Un texte trop
/// court pour en juger (moins de 6 mots) ressemble à tout.
pub fn textes_proches(a: &str, b: &str) -> bool {
    let mots = |t: &str| -> HashSet<String> {
        normaliser(t).split(' ').filter(|m| m.chars().count() >= 3).map(str::to_string).collect()
    };
    let (x, y) = (mots(a), mots(b));
    if x.len() < 6 || y.len() < 6 {
        return true;
    }
    let commun = x.intersection(&y).count();
    commun * 2 >= x.len().min(y.len())
}

/// Ce qu'il faut savoir d'une offre pour la comparer.
#[derive(Debug, Clone, Copy)]
pub struct Comparee<'a> {
    pub source: &'a str,
    pub url: &'a str,
    pub entreprise: &'a str,
    pub titre: &'a str,
    pub lieu: &'a str,
    pub description: &'a str,
}

/// La règle des doublons : la même adresse d'offre, ou la même empreinte (et, du même site, des textes proches).
pub fn meme_offre(a: &Comparee, b: &Comparee) -> bool {
    if !a.url.is_empty() && a.url == b.url {
        return true;
    }
    match (empreinte(a.entreprise, a.titre, a.lieu), empreinte(b.entreprise, b.titre, b.lieu)) {
        (Some(x), Some(y)) if x == y => a.source != b.source || textes_proches(a.description, b.description),
        _ => false,
    }
}

/// La position du premier de ces mots (ou suites de mots) dans un texte remis au propre.
fn premier(t: &str, mots: &[&str]) -> Option<usize> {
    let t = format!(" {t} ");
    mots.iter().filter_map(|m| t.find(&format!(" {m} "))).min()
}

/// Les contrats et leurs mots, dans l'ordre où ils l'emportent à égalité.
const MOTS_CONTRATS: [(Contrat, &[&str]); 7] = [
    (
        Contrat::Alternance,
        &["alternance", "apprentissage", "apprenti", "apprentie", "contrat pro", "contrat de professionnalisation"],
    ),
    (Contrat::Stage, &["stage", "stagiaire"]),
    (Contrat::Interim, &["interim", "interimaire", "mission d interim"]),
    (Contrat::Saisonnier, &["saisonnier", "saisonniere", "contrat saisonnier"]),
    (Contrat::Independant, &["freelance", "independant", "auto entrepreneur", "portage salarial"]),
    (Contrat::Cdi, &["cdi", "duree indeterminee", "permanent"]),
    (Contrat::Cdd, &["cdd", "duree determinee"]),
];

/// Le contrat d'après les mots d'un texte : celui dont un mot vient en premier.
fn contrat_dans(t: &str) -> Option<Contrat> {
    let t = normaliser(t);
    MOTS_CONTRATS
        .iter()
        .filter_map(|(c, mots)| premier(&t, mots).map(|p| (p, *c)))
        .min_by_key(|(p, c)| (*p, *c))
        .map(|(_, c)| c)
}

/// Le contrat d'une offre : d'après son titre, puis ce que la source en dit (« CDI », « Intérim », « permanent »), puis
/// son texte ; enfin « contract » d'Adzuna (un contrat à durée limitée : un CDD, faute de mieux).
pub fn contrat_lu(source_dit: &str, titre: &str, description: &str) -> Option<Contrat> {
    let brut = normaliser(source_dit).replace("full time", "").replace("part time", "");
    contrat_dans(titre)
        .or_else(|| contrat_dans(&brut.replace("contract", "")))
        .or_else(|| contrat_dans(description))
        .or_else(|| premier(&brut, &["contract"]).map(|_| Contrat::Cdd))
}

/// Le télétravail d'une offre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeleOffre {
    /// Une partie du temps (« deux jours par semaine », « hybride »)
    Partiel,
    /// Tout le temps
    Complet,
}

impl TeleOffre {
    pub fn code(self) -> &'static str {
        match self {
            TeleOffre::Partiel => "partiel",
            TeleOffre::Complet => "complet",
        }
    }

    pub fn depuis_code(c: &str) -> Option<TeleOffre> {
        match c {
            "partiel" => Some(TeleOffre::Partiel),
            "complet" => Some(TeleOffre::Complet),
            _ => None,
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            TeleOffre::Partiel => "Télétravail possible",
            TeleOffre::Complet => "En télétravail",
        }
    }
}

/// Le télétravail d'après le titre et le texte d'une offre (rien : pas dit, ou refusé).
pub fn teletravail_lu(titre: &str, description: &str) -> Option<TeleOffre> {
    let t = normaliser(&format!("{titre} . {description}"));
    if premier(
        &t,
        &[
            "pas de teletravail",
            "sans teletravail",
            "teletravail impossible",
            "teletravail non",
            "pas de travail a distance",
        ],
    )
    .is_some()
    {
        return None;
    }
    if premier(
        &t,
        &[
            "100 teletravail",
            "teletravail complet",
            "teletravail total",
            "full remote",
            "entierement en teletravail",
            "totalement en teletravail",
            "100 remote",
            "100 a distance",
        ],
    )
    .is_some()
    {
        return Some(TeleOffre::Complet);
    }
    premier(&t, &["teletravail", "remote", "hybride", "travail a distance", "a distance"]).map(|_| TeleOffre::Partiel)
}

/// Le département d'une offre : un numéro entre parenthèses (« Metz (57) »), un code postal (« 57000 Metz »), ou le nom
/// d'un département parmi ses zones (Adzuna) ou dans son lieu (« Metz, Moselle »).
pub fn departement_lu(lieu: &str, zones: &[String]) -> Option<&'static Departement> {
    // (57), (2A), (974)
    for morceau in lieu.split('(').skip(1) {
        let code: String = morceau.chars().take_while(|c| *c != ')').collect();
        if let Some(d) = lieux::departement(code.trim()).filter(|_| code.trim().len() <= 3) {
            return Some(d);
        }
    }
    // Un code postal de 5 chiffres
    for mot in lieu.split(|c: char| !c.is_ascii_digit()) {
        if mot.len() == 5 {
            let code = match &mot[..2] {
                "97" => mot[..3].to_string(),
                "20" if &mot[2..3] < "2" => "2A".to_string(),
                "20" => "2B".to_string(),
                d => d.to_string(),
            };
            if let Some(d) = lieux::departement(&code) {
                return Some(d);
            }
        }
    }
    // Un nom de département
    let noms: Vec<String> =
        zones.iter().cloned().chain(lieu.split(',').map(str::to_string)).map(|z| normaliser(&z)).collect();
    lieux::DEPARTEMENTS.iter().find(|d| noms.iter().any(|z| *z == normaliser(d.nom)))
}

/// La distance entre deux points, en km (à vol d'oiseau).
pub fn distance_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let r = 6371.0;
    let (la1, la2) = (a.0.to_radians(), b.0.to_radians());
    let (dla, dlo) = ((b.0 - a.0).to_radians(), (b.1 - a.1).to_radians());
    let h = (dla / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dlo / 2.0).sin().powi(2);
    2.0 * r * h.sqrt().asin()
}

/// Une offre gardée, telle que la liste et les filtres la voient (sans son texte).
#[derive(Debug, Clone, PartialEq)]
pub struct Offre {
    pub id: i64,
    pub titre: String,
    pub entreprise: String,
    pub lieu: String,
    /// Le code du département et de la région, s'ils sont connus
    pub departement: Option<String>,
    pub region: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub contrat: Option<Contrat>,
    pub teletravail: Option<TeleOffre>,
    pub salaire: String,
    pub publiee_le: Option<Jour>,
    pub ajoutee_le: Jour,
    /// Les sources qui l'ont donnée (« adzuna », « linkedin »), la sienne d'abord
    pub sources: Vec<String>,
    pub favori: bool,
    pub plus_tard: bool,
    /// « Pas pour moi » : cachée de la liste
    pub cachee: bool,
    /// Ouverte au moins une fois
    pub vue: bool,
    /// Les recherches gardées qui l'ont trouvée
    pub recherches: Vec<i64>,
    /// La candidature faite pour elle
    pub candidature: Option<i64>,
    /// Étape 6 : le diplôme et l'expérience demandés, lus dans son texte (rien : non dit, ou pas encore lu)
    pub diplome: Option<Diplome>,
    pub experience: Option<Experience>,
    /// La note de pertinence, de 0 à 100 (rien : « Ce que je cherche » n'est pas rempli)
    pub pertinence: Option<u8>,
    /// Sa raison en une ligne
    pub raison: String,
    /// Hors sujet d'après la note
    pub hors_sujet: bool,
    /// Jordan a dit « Pas hors sujet »
    pub pas_hors_sujet: bool,
}

impl Default for Offre {
    fn default() -> Offre {
        Offre {
            id: 0,
            titre: String::new(),
            entreprise: String::new(),
            lieu: String::new(),
            departement: None,
            region: None,
            latitude: None,
            longitude: None,
            contrat: None,
            teletravail: None,
            salaire: String::new(),
            publiee_le: None,
            ajoutee_le: Jour { annee: 2026, mois: 1, jour: 1 },
            sources: Vec::new(),
            favori: false,
            plus_tard: false,
            cachee: false,
            vue: false,
            recherches: Vec::new(),
            candidature: None,
            diplome: None,
            experience: None,
            pertinence: None,
            raison: String::new(),
            hors_sujet: false,
            pas_hors_sujet: false,
        }
    }
}

impl Offre {
    pub fn point(&self) -> Option<(f64, f64)> {
        Some((self.latitude?, self.longitude?))
    }

    /// Rangée dans « Hors sujet » (hors sujet d'après la note, et Jordan n'a pas dit « Pas hors sujet »).
    pub fn rangee_hors_sujet(&self) -> bool {
        self.hors_sujet && !self.pas_hors_sujet
    }

    /// Le niveau montré (rien : pas de note).
    pub fn niveau(&self) -> Option<Niveau> {
        let s = self.pertinence?;
        Some(if self.rangee_hors_sujet() { Niveau::HorsSujet } else { Niveau::de(s) })
    }
}

/// Le lieu filtré.
#[derive(Debug, Clone, PartialEq)]
pub enum FiltreLieu {
    Tous,
    /// Autour d'une ville : son nom, sa clé ([`cle_ville`]) et le rayon
    Ville {
        nom: String,
        cle: String,
        rayon: u32,
    },
    Departement(&'static Departement),
    Region(&'static Region),
}

/// Les offres montrées.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liste {
    /// Toutes, sauf celles marquées « Pas pour moi » et les hors sujet
    Toutes,
    Favoris,
    PlusTard,
    /// Étape 6 : celles que la note range hors sujet (jamais effacées)
    HorsSujet,
    /// Celles marquées « Pas pour moi »
    Cachees,
}

/// L'ordre de la liste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tri {
    /// Les plus pertinentes d'abord (puis les plus récentes)
    Pertinence,
    Recentes,
}

impl Tri {
    pub fn code(self) -> &'static str {
        match self {
            Tri::Pertinence => "pertinence",
            Tri::Recentes => "recentes",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Tri> {
        match c {
            "pertinence" => Some(Tri::Pertinence),
            "recentes" => Some(Tri::Recentes),
            _ => None,
        }
    }
}

impl Liste {
    pub const TOUTES: [Liste; 5] = [Liste::Toutes, Liste::Favoris, Liste::PlusTard, Liste::HorsSujet, Liste::Cachees];

    pub fn code(self) -> &'static str {
        match self {
            Liste::Toutes => "toutes",
            Liste::Favoris => "favoris",
            Liste::PlusTard => "plus_tard",
            Liste::HorsSujet => "hors_sujet",
            Liste::Cachees => "cachees",
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            Liste::Toutes => "Toutes",
            Liste::Favoris => "Favoris",
            Liste::PlusTard => "Plus tard",
            Liste::HorsSujet => "Hors sujet",
            Liste::Cachees => "Cachées",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Liste> {
        Liste::TOUTES.into_iter().find(|l| l.code() == c)
    }
}

/// Les filtres de l'onglet Offres, vérifiés.
#[derive(Debug, Clone, PartialEq)]
pub struct Filtres {
    pub lieu: FiltreLieu,
    pub contrat: Option<Contrat>,
    pub teletravail: Teletravail,
    /// Le texte cherché dans tout le texte des offres (vide : rien)
    pub texte: String,
    pub liste: Liste,
    /// Étape 6 : seulement « aucun diplôme demandé », seulement « débutant accepté »
    pub sans_diplome: bool,
    pub debutant: bool,
    pub tri: Tri,
}

impl Default for Filtres {
    fn default() -> Filtres {
        Filtres::tous()
    }
}

impl Filtres {
    /// Aucun filtre.
    pub const fn tous() -> Filtres {
        Filtres {
            lieu: FiltreLieu::Tous,
            contrat: None,
            teletravail: Teletravail::PeuImporte,
            texte: String::new(),
            liste: Liste::Toutes,
            sans_diplome: false,
            debutant: false,
            tri: Tri::Pertinence,
        }
    }

    /// Un filtre est posé (en plus de la liste).
    pub fn actifs(&self) -> bool {
        self.lieu != FiltreLieu::Tous
            || self.contrat.is_some()
            || self.teletravail != Teletravail::PeuImporte
            || !self.texte.is_empty()
            || self.sans_diplome
            || self.debutant
    }
}

/// Ce que la page a envoyé pour les filtres, tel quel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaisieFiltres {
    pub genre: String,
    pub ville: String,
    pub rayon: i64,
    pub departement: String,
    pub region: String,
    pub contrat: String,
    pub teletravail: String,
    pub texte: String,
    pub liste: String,
    pub sans_diplome: bool,
    pub debutant: bool,
    pub tri: String,
}

/// La longueur du texte cherché.
pub const TEXTE_MAX: usize = 100;

/// Vérifie les filtres envoyés par la page. Une ville vide ou un choix vide : pas de filtre de lieu (on est en train de
/// choisir). L'erreur est une phrase pour la bulle de l'écran.
pub fn verifier_filtres(s: &SaisieFiltres) -> Result<Filtres, String> {
    let lieu = match GenreLieu::depuis_code(&s.genre).unwrap_or(GenreLieu::Partout) {
        GenreLieu::Partout => FiltreLieu::Tous,
        GenreLieu::Ville => {
            let nom = une_ligne(&s.ville);
            let cle = cle_ville(&nom);
            if cle.is_empty() {
                FiltreLieu::Tous
            } else {
                if nom.chars().count() > crate::recherche::VILLE_MAX {
                    return Err("Le nom de la ville est trop long.".into());
                }
                let rayon = u32::try_from(s.rayon).ok().filter(|r| (1..=RAYON_MAX).contains(r));
                let rayon =
                    rayon.ok_or(format!("Choisis une distance autour de la ville ({RAYON_MAX} km au plus)."))?;
                FiltreLieu::Ville { nom, cle, rayon }
            }
        }
        GenreLieu::Departement if s.departement.is_empty() => FiltreLieu::Tous,
        GenreLieu::Departement => {
            FiltreLieu::Departement(lieux::departement(&s.departement).ok_or("Choisis un département dans la liste.")?)
        }
        GenreLieu::Region if s.region.is_empty() => FiltreLieu::Tous,
        GenreLieu::Region => FiltreLieu::Region(lieux::region(&s.region).ok_or("Choisis une région dans la liste.")?),
    };
    let contrat = match s.contrat.as_str() {
        "" | "tous" => None,
        c => Some(Contrat::depuis_code(c).ok_or("Ce type de contrat n'existe pas.")?),
    };
    let teletravail = match s.teletravail.as_str() {
        "" => Teletravail::PeuImporte,
        t => Teletravail::depuis_code(t).ok_or("Choisis ce que tu veux pour le télétravail.")?,
    };
    let texte = une_ligne(&s.texte);
    if texte.chars().count() > TEXTE_MAX {
        return Err(format!("Le texte cherché est trop long ({TEXTE_MAX} caractères au plus)."));
    }
    let liste = match s.liste.as_str() {
        "" => Liste::Toutes,
        l => Liste::depuis_code(l).ok_or("Cette liste n'existe pas.")?,
    };
    let tri = match s.tri.as_str() {
        "" => Tri::Pertinence,
        t => Tri::depuis_code(t).ok_or("Ce tri n'existe pas.")?,
    };
    Ok(Filtres { lieu, contrat, teletravail, texte, liste, sans_diplome: s.sans_diplome, debutant: s.debutant, tri })
}

/// Le centre d'une ville d'après les offres qui y sont et dont le site donne la position (la médiane de leurs
/// positions) : rien si aucune n'en donne.
pub fn centre_ville(offres: &[Offre], cle: &str) -> Option<(f64, f64)> {
    let mut la: Vec<f64> = Vec::new();
    let mut lo: Vec<f64> = Vec::new();
    for (a, b) in offres.iter().filter(|o| cle_ville(&o.lieu) == cle).filter_map(Offre::point) {
        la.push(a);
        lo.push(b);
    }
    if la.is_empty() {
        return None;
    }
    la.sort_by(f64::total_cmp);
    lo.sort_by(f64::total_cmp);
    Some((la[la.len() / 2], lo[lo.len() / 2]))
}

/// Ce que les filtres demandent en plus de l'offre elle-même.
#[derive(Debug, Clone, Default)]
pub struct Appui<'a> {
    /// Les offres qui contiennent le texte cherché (rien : aucun texte cherché)
    pub textes: Option<&'a HashSet<i64>>,
    /// Le centre de la ville filtrée, s'il est connu ([`centre_ville`])
    pub centre: Option<(f64, f64)>,
    /// Les recherches gardées faites autour de cette ville, dans ce rayon ou moins
    pub recherches_ville: &'a [i64],
}

/// L'offre passe les filtres.
pub fn garder(o: &Offre, f: &Filtres, appui: &Appui) -> bool {
    let liste = match f.liste {
        Liste::Toutes => !o.cachee && !o.rangee_hors_sujet(),
        Liste::Favoris => o.favori,
        Liste::PlusTard => o.plus_tard,
        Liste::HorsSujet => !o.cachee && o.rangee_hors_sujet(),
        Liste::Cachees => o.cachee,
    };
    if !liste {
        return false;
    }
    if (f.sans_diplome && o.diplome != Some(Diplome::Aucun))
        || (f.debutant && o.experience != Some(Experience::Debutant))
    {
        return false;
    }
    if let Some(t) = appui.textes
        && !t.contains(&o.id)
    {
        return false;
    }
    if let Some(c) = f.contrat
        && o.contrat != Some(c)
    {
        return false;
    }
    let tele = match f.teletravail {
        Teletravail::PeuImporte => true,
        Teletravail::Possible => o.teletravail.is_some(),
        Teletravail::Complet => o.teletravail == Some(TeleOffre::Complet),
    };
    if !tele {
        return false;
    }
    match &f.lieu {
        FiltreLieu::Tous => true,
        FiltreLieu::Departement(d) => o.departement.as_deref() == Some(d.code),
        FiltreLieu::Region(r) => o.region.as_deref() == Some(r.code),
        FiltreLieu::Ville { cle, rayon, .. } => {
            // La ville même ; sinon, à moins du rayon du centre connu ; sinon (pas de position), trouvée par une
            // recherche faite autour de cette ville dans ce rayon
            cle_ville(&o.lieu) == *cle
                || match (appui.centre, o.point()) {
                    (Some(c), Some(p)) => distance_km(c, p) <= f64::from(*rayon),
                    _ => o.recherches.iter().any(|r| appui.recherches_ville.contains(r)),
                }
        }
    }
}

/// Les offres dans l'ordre de la liste : les plus récentes d'abord (publiées, sinon gardées), puis par numéro.
pub fn trier(l: &mut [Offre]) {
    l.sort_by(|a, b| {
        let da = a.publiee_le.unwrap_or(a.ajoutee_le);
        let db = b.publiee_le.unwrap_or(b.ajoutee_le);
        db.cmp(&da).then(b.id.cmp(&a.id))
    });
}

/// Les offres dans l'ordre choisi : les plus pertinentes d'abord (une offre sans note après celles qui en ont), puis
/// les plus récentes ; ou seulement les plus récentes.
pub fn trier_selon(l: &mut [Offre], t: Tri) {
    trier(l);
    if t == Tri::Pertinence {
        // (un tri stable : à note égale, l'ordre des plus récentes reste)
        l.sort_by_key(|o| std::cmp::Reverse(o.pertinence.map_or(-1, i16::from)));
    }
}

/// Une adresse qu'on peut ouvrir dans le navigateur : http ou https seulement, sans espace, guillemet ni caractère de
/// contrôle.
pub fn lien_ouvrable(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() <= 2000
        && !url.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"' | '\\' | '^' | '`'))
        && url.split("://").nth(1).is_some_and(|r| !r.is_empty() && !r.starts_with('/'))
}

/// L'hôte d'une adresse (pour le journal : jamais l'adresse entière).
pub fn hote(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or_default()
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .chars()
        .take(80)
        .collect()
}

/// « Adzuna », « Adzuna et LinkedIn », « Adzuna, Indeed et LinkedIn ».
pub fn noms_sources(noms: &[&str]) -> String {
    match noms {
        [] => String::new(),
        [a] => (*a).to_string(),
        [debut @ .., fin] => format!("{} et {fin}", debut.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j(d: u32) -> Jour {
        Jour::nouveau(2026, 11, d).unwrap()
    }

    #[test]
    fn remis_au_propre() {
        assert_eq!(normaliser("  Préparateur-Cariste (H/F) : ÉQUIPE d'été "), "preparateur cariste h f equipe d ete");
        assert_eq!(cle_titre("Agent de quai (H/F) - CDI"), "agent de quai");
        assert_eq!(cle_titre("AGENT DE QUAI F/H"), "agent de quai");
        assert_eq!(cle_titre("Cariste CACES 3 - CDI"), "cariste caces 3");
        assert_eq!(cle_entreprise("ENTREPÔTS DU VAL DE MOSELLE SAS"), "entrepots du val de moselle");
        assert_eq!(cle_ville("Metz, Moselle"), "metz");
        assert_eq!(cle_ville("Metz (57)"), "metz");
        assert_eq!(cle_ville("57000 Metz Cedex 1"), "metz");
        assert_eq!(cle_ville("St-Avold"), "saint avold");
        assert_eq!(cle_ville("Saint-Avold, Moselle"), "saint avold");
        assert_eq!(empreinte("", "Cariste", "Metz"), None);
        assert_eq!(empreinte("Sarl", "Cariste", "Metz"), None);
    }

    fn c<'a>(
        source: &'a str,
        entreprise: &'a str,
        titre: &'a str,
        lieu: &'a str,
        description: &'a str,
    ) -> Comparee<'a> {
        Comparee { source, url: "", entreprise, titre, lieu, description }
    }

    /// La règle des doublons, et sa contre-épreuve : des offres différentes ne sont jamais fusionnées.
    #[test]
    fn doublons() {
        let a = c(
            "adzuna",
            "Entrepôts du Val de Moselle",
            "Agent de quai H/F",
            "Ennery, Moselle",
            "Chargement et déchargement des camions, contrôle des palettes à l'arrivée, rangement en zone de stockage.",
        );
        let b = c(
            "indeed",
            "ENTREPOTS DU VAL DE MOSELLE",
            "Agent de quai (H/F)",
            "Ennery (57)",
            "...chargement et déchargement...",
        );
        assert!(meme_offre(&a, &b));
        // La même adresse
        let u1 = Comparee { url: "https://x.fr/1", ..c("indeed", "", "Cariste", "Metz", "") };
        let u2 = Comparee { url: "https://x.fr/1", ..c("indeed", "", "Cariste", "Metz", "") };
        assert!(meme_offre(&u1, &u2));
        // Différentes : autre entreprise, autre ville, autre poste, sans entreprise
        for x in [
            c("indeed", "Transports Seille Express", "Agent de quai", "Ennery", ""),
            c("indeed", "Entrepôts du Val de Moselle", "Agent de quai", "Woippy (57)", ""),
            c("indeed", "Entrepôts du Val de Moselle", "Opérateur logistique de nuit", "Ennery", ""),
            c("indeed", "", "Agent de quai H/F", "Ennery", ""),
        ] {
            assert!(!meme_offre(&a, &x), "{x:?}");
        }
        assert!(!meme_offre(&c("adzuna", "", "Cariste", "Metz", ""), &c("indeed", "", "Cariste", "Metz", "")));
        // Du même site, sous deux numéros : seulement si les textes se ressemblent
        let jour = c(
            "adzuna",
            "Logipôle Lorraine",
            "Cariste",
            "Metz",
            "Conduite de chariot en équipe de jour, réception des livraisons, rangement des palettes, inventaires tournants.",
        );
        let nuit = c(
            "adzuna",
            "Logipôle Lorraine",
            "Cariste",
            "Metz",
            "Poste de nuit : chargement des semi-remorques, préparation des expéditions urgentes, filmage, contrôle qualité.",
        );
        let encore = c(
            "adzuna",
            "Logipôle Lorraine",
            "Cariste (H/F)",
            "Metz",
            "Conduite de chariot en équipe de jour, réception des livraisons, rangement des palettes et inventaires.",
        );
        assert!(!meme_offre(&jour, &nuit));
        assert!(meme_offre(&jour, &encore));
    }

    #[test]
    fn contrats_lus() {
        assert_eq!(contrat_lu("permanent full_time", "Agent de quai H/F", ""), Some(Contrat::Cdi));
        assert_eq!(
            contrat_lu("contract full_time", "Préparateur de commandes", "Mission d'intérim de 3 mois renouvelable."),
            Some(Contrat::Interim)
        );
        assert_eq!(contrat_lu("contract", "Magasinier", "CDD de 6 mois."), Some(Contrat::Cdd));
        assert_eq!(contrat_lu("contract", "Magasinier", "Réception des pièces."), Some(Contrat::Cdd));
        assert_eq!(contrat_lu("contract", "Apprenti agent logistique", "en alternance"), Some(Contrat::Alternance));
        assert_eq!(
            contrat_lu("contract", "Employé libre-service", "Contrat saisonnier pour les fêtes."),
            Some(Contrat::Saisonnier)
        );
        assert_eq!(contrat_lu("CDI", "Vendeur", ""), Some(Contrat::Cdi));
        assert_eq!(contrat_lu("Intérim", "Agent de quai", ""), Some(Contrat::Interim));
        assert_eq!(contrat_lu("Temps plein", "Cariste CACES 3 - CDI", "CDD possible"), Some(Contrat::Cdi));
        assert_eq!(contrat_lu("", "Stage assistant logistique", ""), Some(Contrat::Stage));
        assert_eq!(contrat_lu("", "Agent de tri", "Tri de colis."), None);
        // (un mot dans un autre ne compte pas : « stagnation » n'est pas un stage, « cdiscount » pas un CDI)
        assert_eq!(contrat_lu("", "Vendeur", "Ventes en stagnation chez Cdiscount."), None);
    }

    #[test]
    fn teletravail_lu_dans_le_texte() {
        assert_eq!(
            teletravail_lu("Technicien support", "Télétravail possible deux jours par semaine."),
            Some(TeleOffre::Partiel)
        );
        assert_eq!(teletravail_lu("Développeur web", "Poste 100 % télétravail."), Some(TeleOffre::Complet));
        assert_eq!(teletravail_lu("Développeur (full remote)", ""), Some(TeleOffre::Complet));
        assert_eq!(teletravail_lu("Cariste", "Pas de télétravail."), None);
        assert_eq!(teletravail_lu("Cariste", "Chargement des camions."), None);
    }

    #[test]
    fn departements_lus() {
        let z = |l: &[&str]| l.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(departement_lu("Metz (57)", &[]).unwrap().code, "57");
        assert_eq!(departement_lu("Ajaccio (2A)", &[]).unwrap().code, "2A");
        assert_eq!(departement_lu("57000 Metz", &[]).unwrap().code, "57");
        assert_eq!(departement_lu("20200 Bastia", &[]).unwrap().code, "2B");
        assert_eq!(departement_lu("97400 Saint-Denis", &[]).unwrap().code, "974");
        assert_eq!(departement_lu("Metz, Moselle", &[]).unwrap().code, "57");
        assert_eq!(
            departement_lu("Nancy", &z(&["France", "Grand Est", "Meurthe-et-Moselle", "Nancy"])).unwrap().code,
            "54"
        );
        assert_eq!(departement_lu("Nancy", &[]), None);
        assert_eq!(departement_lu("Paris (Île-de-France)", &[]), None);
        let d = distance_km((49.1193, 6.1757), (48.692, 6.184));
        assert!((46.0..49.0).contains(&d), "{d}");
    }

    fn offre(id: i64, lieu: &str, point: Option<(f64, f64)>) -> Offre {
        Offre {
            id,
            titre: "Cariste".into(),
            lieu: lieu.into(),
            departement: departement_lu(lieu, &[]).map(|d| d.code.to_string()),
            region: departement_lu(lieu, &[]).map(|d| d.region.to_string()),
            latitude: point.map(|p| p.0),
            longitude: point.map(|p| p.1),
            ajoutee_le: j(18),
            ..Offre::default()
        }
    }

    fn saisie() -> SaisieFiltres {
        SaisieFiltres { genre: "partout".into(), rayon: 30, ..SaisieFiltres::default() }
    }

    #[test]
    fn les_filtres() {
        let metz = offre(1, "Metz, Moselle", Some((49.1193, 6.1757)));
        let woippy = offre(2, "Woippy, Moselle", Some((49.151, 6.151)));
        let nancy = offre(3, "Nancy (54)", Some((48.692, 6.184)));
        let ennery_sans_point = offre(4, "Ennery (57)", None);
        let mut toutes = vec![metz.clone(), woippy.clone(), nancy.clone(), ennery_sans_point.clone()];
        toutes[1].contrat = Some(Contrat::Cdi);
        toutes[2].teletravail = Some(TeleOffre::Complet);
        toutes[3].recherches = vec![7];
        toutes[0].favori = true;
        toutes[1].cachee = true;
        let garde = |s: &SaisieFiltres, appui: &Appui| -> Vec<i64> {
            let f = verifier_filtres(s).unwrap();
            toutes.iter().filter(|o| garder(o, &f, appui)).map(|o| o.id).collect()
        };
        // Toutes sauf la cachée ; les cachées seules
        assert_eq!(garde(&saisie(), &Appui::default()), [1, 3, 4]);
        assert_eq!(garde(&SaisieFiltres { liste: "cachees".into(), ..saisie() }, &Appui::default()), [2]);
        assert_eq!(garde(&SaisieFiltres { liste: "favoris".into(), ..saisie() }, &Appui::default()), [1]);
        // Département, région
        let s = SaisieFiltres { genre: "departement".into(), departement: "54".into(), ..saisie() };
        assert_eq!(garde(&s, &Appui::default()), [3]);
        let s = SaisieFiltres { genre: "region".into(), region: "grand_est".into(), ..saisie() };
        assert_eq!(garde(&s, &Appui::default()), [1, 3, 4]);
        // Autour de Metz, 20 km : Metz par son nom, Woippy (cachée) et Nancy par la distance, Ennery par la recherche
        let s = SaisieFiltres { genre: "ville".into(), ville: "Metz".into(), rayon: 20, ..saisie() };
        let centre = centre_ville(&toutes, "metz");
        assert_eq!(centre, Some((49.1193, 6.1757)));
        assert_eq!(garde(&s, &Appui { centre, recherches_ville: &[7], ..Appui::default() }), [1, 4]);
        assert_eq!(garde(&s, &Appui { centre, recherches_ville: &[], ..Appui::default() }), [1]);
        let s = SaisieFiltres { rayon: 60, ..s };
        assert_eq!(garde(&s, &Appui { centre, ..Appui::default() }), [1, 3]);
        // Contrat, télétravail, texte
        let s = SaisieFiltres { liste: "cachees".into(), contrat: "cdi".into(), ..saisie() };
        assert_eq!(garde(&s, &Appui::default()), [2]);
        assert_eq!(garde(&SaisieFiltres { teletravail: "complet".into(), ..saisie() }, &Appui::default()), [3]);
        let t: HashSet<i64> = [4, 2].into();
        assert_eq!(garde(&saisie(), &Appui { textes: Some(&t), ..Appui::default() }), [4]);
        // Une ville pas encore écrite : pas de filtre
        assert_eq!(
            verifier_filtres(&SaisieFiltres { genre: "ville".into(), ..saisie() }).unwrap().lieu,
            FiltreLieu::Tous
        );
        assert!(!verifier_filtres(&saisie()).unwrap().actifs());
    }

    /// Contre-épreuve : des filtres faux sont refusés avec une phrase simple.
    #[test]
    fn filtres_refuses() {
        for (s, attendu) in [
            (SaisieFiltres { genre: "ville".into(), ville: "Metz".into(), rayon: 0, ..saisie() }, "distance"),
            (SaisieFiltres { genre: "ville".into(), ville: "M".repeat(70), rayon: 10, ..saisie() }, "trop long"),
            (SaisieFiltres { genre: "departement".into(), departement: "99".into(), ..saisie() }, "département"),
            (SaisieFiltres { genre: "region".into(), region: "Lorraine".into(), ..saisie() }, "région"),
            (SaisieFiltres { contrat: "esclavage".into(), ..saisie() }, "contrat"),
            (SaisieFiltres { teletravail: "jamais".into(), ..saisie() }, "télétravail"),
            (SaisieFiltres { texte: "x".repeat(101), ..saisie() }, "trop long"),
            (SaisieFiltres { liste: "corbeille".into(), ..saisie() }, "liste"),
        ] {
            let e = verifier_filtres(&s).unwrap_err();
            assert!(e.contains(attendu), "{e} / {attendu}");
            assert!(!e.contains(['\u{2013}', '\u{2014}']));
        }
    }

    #[test]
    fn tri_liens_et_noms() {
        let mut l = vec![
            Offre { id: 1, publiee_le: Some(j(10)), ajoutee_le: j(18), ..Offre::default() },
            Offre { id: 2, publiee_le: None, ajoutee_le: j(12), ..Offre::default() },
            Offre { id: 3, publiee_le: Some(j(15)), ajoutee_le: j(18), ..Offre::default() },
        ];
        trier(&mut l);
        assert_eq!(l.iter().map(|o| o.id).collect::<Vec<_>>(), [3, 2, 1]);
        assert!(lien_ouvrable("https://www.adzuna.fr/land/ad/1?se=x"));
        for faux in
            ["javascript:alert(1)", "file:///C:/Windows", "https://a b", "http://", "ms-settings:", "https://x\"&calc"]
        {
            assert!(!lien_ouvrable(faux), "{faux}");
        }
        assert_eq!(hote("https://www.adzuna.fr/land/ad/1?app_key=x"), "www.adzuna.fr");
        assert_eq!(noms_sources(&["Adzuna"]), "Adzuna");
        assert_eq!(noms_sources(&["Adzuna", "LinkedIn"]), "Adzuna et LinkedIn");
        assert_eq!(noms_sources(&["Adzuna", "Indeed", "LinkedIn"]), "Adzuna, Indeed et LinkedIn");
    }

    /// Étape 6 : « Aucun diplôme requis », « Débutant accepté », la rubrique Hors sujet et le tri par pertinence.
    #[test]
    fn filtres_de_la_lecture_et_tri() {
        let o = |id, d, e, p: Option<u8>, hors| Offre {
            id,
            diplome: d,
            experience: e,
            pertinence: p,
            hors_sujet: hors,
            publiee_le: Some(j(id as u32)),
            ..Offre::default()
        };
        let mut l = vec![
            o(1, Some(Diplome::Aucun), Some(Experience::Debutant), Some(60), false),
            o(2, Some(Diplome::Bac2), Some(Experience::Debutant), Some(90), false),
            o(3, Some(Diplome::Aucun), Some(Experience::Duree(24)), Some(75), false),
            o(4, None, None, None, false),
            o(5, Some(Diplome::Aucun), Some(Experience::Debutant), Some(0), true),
        ];
        let garde = |s: SaisieFiltres, l: &[Offre]| -> Vec<i64> {
            let f = verifier_filtres(&s).unwrap();
            l.iter().filter(|x| garder(x, &f, &Appui::default())).map(|x| x.id).collect()
        };
        assert_eq!(garde(saisie(), &l), [1, 2, 3, 4]);
        assert_eq!(garde(SaisieFiltres { sans_diplome: true, ..saisie() }, &l), [1, 3]);
        assert_eq!(garde(SaisieFiltres { debutant: true, ..saisie() }, &l), [1, 2]);
        assert_eq!(garde(SaisieFiltres { sans_diplome: true, debutant: true, ..saisie() }, &l), [1]);
        assert_eq!(garde(SaisieFiltres { liste: "hors_sujet".into(), ..saisie() }, &l), [5]);
        // « Pas hors sujet » : de retour dans la liste
        l[4].pas_hors_sujet = true;
        assert_eq!(garde(saisie(), &l), [1, 2, 3, 4, 5]);
        assert_eq!(l[4].niveau(), Some(Niveau::UnPeu));
        assert_eq!(l[1].niveau(), Some(Niveau::TresProche));
        assert!(verifier_filtres(&SaisieFiltres { sans_diplome: true, ..saisie() }).unwrap().actifs());
        assert!(verifier_filtres(&SaisieFiltres { tri: "hasard".into(), ..saisie() }).is_err());
        // Le tri : par note (sans note à la fin), ou par date
        let mut x = l.clone();
        trier_selon(&mut x, Tri::Pertinence);
        assert_eq!(x.iter().map(|o| o.id).collect::<Vec<_>>(), [2, 3, 1, 5, 4]);
        trier_selon(&mut x, Tri::Recentes);
        assert_eq!(x.iter().map(|o| o.id).collect::<Vec<_>>(), [5, 4, 3, 2, 1]);
    }
}
