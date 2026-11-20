//! L'annuaire officiel des entreprises (0.1.4, l'onglet « L'entreprise » d'une offre) : l'API publique et gratuite de
//! l'État, sans clé (<https://recherche-entreprises.api.gouv.fr>, `GET /search`).
//!
//! - [`Demande::depuis_offre`] : ce qu'on demande, d'après le nom de l'entreprise et le lieu de l'offre (le code postal
//!   si la commune n'en a qu'un, sinon le département) ; rien si l'offre ne dit pas assez pour être sûr ;
//! - [`lire_reponse`] : la réponse lue, sans réseau (les tests la lisent sur des réponses inventées). Un résultat n'est
//!   pris que s'il correspond clairement : le même nom (remis au propre, sans la forme juridique) ou un nom qui contient
//!   l'autre, et la même commune ou le même département ; deux entreprises différentes qui correspondent autant : rien ;
//! - [`chercher`] : l'appel lui-même (une seule requête, [`DELAI`] au plus), fait par l'appli sur un fil à part,
//!   jamais dans un test.
//!
//! Rien n'est inventé : chaque information rendue vient de la réponse de l'annuaire. L'annuaire ne donne pas le site
//! internet de l'entreprise ; [`adresse_recherche_web`] construit seulement l'adresse d'une recherche à ouvrir sur un
//! clic de Jordan.

use std::time::Duration;

use serde_json::Value;

use crate::communes;
use crate::texte;
use crate::texte_json;

/// L'adresse de la recherche de l'annuaire.
pub const ADRESSE: &str = "https://recherche-entreprises.api.gouv.fr/search";
/// Le temps laissé à l'annuaire pour répondre.
pub const DELAI: Duration = Duration::from_secs(8);
/// La réponse lue au plus (en octets).
const TAILLE_MAX: u64 = 2_000_000;
/// Les résultats demandés.
const PAR_PAGE: u32 = 5;

/// Ce qu'on demande à l'annuaire pour une offre.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Demande {
    /// Le nom de l'entreprise tel que l'offre l'écrit
    pub nom: String,
    /// Le code postal, si la commune de l'offre n'en a qu'un (sinon rien : le département)
    pub code_postal: Option<String>,
    /// Le département (« 57 », « 2A », « 974 »)
    pub departement: String,
    /// La commune de l'offre, remise au propre pour comparer (vide si l'offre ne donne que le département)
    pub commune: String,
}

/// Ce que l'annuaire dit d'une entreprise (chaque texte tel que l'annuaire le donne ; vide s'il ne le dit pas).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fiche {
    /// Son nom complet (« TRANSPORTS EXEMPLE (TE) »)
    pub nom: String,
    /// Son secteur, en mots (« Transports et entreposage »)
    pub secteur: String,
    /// Sa taille, en mots (« 50 à 99 salariés (chiffre de 2023) »)
    pub taille: String,
    /// L'adresse de son siège
    pub adresse: String,
    /// La commune de son siège (pour une recherche du site)
    pub commune: String,
}

/// Pourquoi l'annuaire n'a rien donné.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurAnnuaire {
    #[error("pas de réponse à temps")]
    Delai,
    #[error("réseau : {0}")]
    Reseau(String),
    #[error("trop de demandes (429)")]
    Occupe,
    #[error("réponse {0}")]
    Statut(u16),
    #[error("réponse illisible")]
    Illisible,
}

/// Les noms qui ne sont pas une entreprise (« Entreprise non précisée », « Confidentiel »).
const PAS_UN_NOM: [&str; 6] =
    ["confidenti", "non precise", "non communique", "anonyme", "entreprise non", "non renseigne"];

/// Les formes juridiques, retirées d'un nom pour comparer (« Transports Exemple SAS » : « transports exemple »).
const FORMES: [&str; 14] =
    ["sas", "sasu", "sarl", "eurl", "sa", "snc", "sci", "scop", "sca", "selarl", "eirl", "ei", "gie", "scp"];

/// Un nom d'entreprise remis au propre pour comparer : minuscules, sans accent ni ponctuation, sans la forme
/// juridique ni les points des sigles (« S.A.R.L. Dupont & Fils » : « dupont fils »).
pub fn nom_propre(t: &str) -> String {
    let sans_points: String = t.chars().filter(|c| *c != '.').collect();
    communes::cle_nom(&sans_points)
        .split_whitespace()
        .filter(|m| !FORMES.contains(m) && *m != "et")
        .collect::<Vec<_>>()
        .join(" ")
}

/// Le département d'un code postal (« 57000 » : « 57 », « 97411 » : « 974 », la Corse : « 2A » ou « 2B »).
fn departement_de(cp: &str) -> Option<String> {
    let cp = cp.trim();
    if cp.len() != 5 || !cp.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(match &cp[..2] {
        "97" | "98" => cp[..3].to_string(),
        "20" => {
            if cp.as_bytes()[2] < b'2' {
                "2A".into()
            } else {
                "2B".into()
            }
        }
        d => d.to_string(),
    })
}

impl Demande {
    /// La demande pour une offre : le nom de l'entreprise et le lieu tel que l'offre l'écrit (« Metz (57) »,
    /// « Sarrebourg (57400) », « 57 - SARREBOURG », « Metz, Grand Est, France »). Rien sans nom, sans lieu reconnu, ou
    /// pour un nom qui n'en est pas un (« Entreprise non précisée »).
    pub fn depuis_offre(entreprise: &str, lieu: &str) -> Option<Demande> {
        let nom = texte::ligne(entreprise);
        let propre = nom_propre(&nom);
        if propre.chars().filter(|c| c.is_alphanumeric()).count() < 2 || PAS_UN_NOM.iter().any(|m| propre.contains(m)) {
            return None;
        }
        // (« 57 - SARREBOURG » à la façon de France Travail : « SARREBOURG (57) »)
        let lieu = lieu.trim();
        let lieu = match lieu.split_once(" - ") {
            Some((d, v))
                if (2..=3).contains(&d.trim().len()) && d.trim().chars().all(|c| c.is_ascii_alphanumeric()) =>
            {
                format!("{} ({})", v.trim(), d.trim())
            }
            _ => lieu.to_string(),
        };
        let premier = lieu.split(',').next().unwrap_or_default().trim().to_string();
        // (un code postal écrit dans le lieu)
        let cp_ecrit = lieu
            .split(|c: char| !c.is_ascii_digit())
            .find(|m| m.len() == 5)
            .filter(|m| departement_de(m).is_some())
            .map(str::to_string);
        let commune = communes::trouver(&lieu).or_else(|| communes::trouver(&premier));
        let (code_postal, departement, nom_commune) = match commune {
            Some(c) => {
                let cps: Vec<&str> = c.codes_postaux.split(',').filter(|x| !x.is_empty()).collect();
                let cp = cp_ecrit.clone().or_else(|| (cps.len() == 1).then(|| cps[0].to_string()));
                (cp, c.departement().to_string(), communes::cle_nom(c.nom))
            }
            None => {
                // (le département seul, écrit entre parenthèses : « Moselle (57) »)
                let dep = cp_ecrit.as_deref().and_then(departement_de).or_else(|| {
                    let i = premier.rfind('(')?;
                    let d = premier[i + 1..].strip_suffix(')')?.trim().to_ascii_uppercase();
                    let ok = (d.len() == 2 || d.len() == 3)
                        && (d.chars().all(|c| c.is_ascii_digit()) || d == "2A" || d == "2B");
                    ok.then_some(d)
                })?;
                // (une commune inconnue de la table, ou un nom porté par plusieurs : le département seul compte)
                (cp_ecrit, dep, String::new())
            }
        };
        Some(Demande { nom, code_postal, departement, commune: nom_commune })
    }

    /// La clé de cette demande pour la mémoire de l'appli (le nom remis au propre, et le lieu demandé).
    pub fn cle(&self) -> String {
        format!("{}|{}", nom_propre(&self.nom), self.code_postal.as_deref().unwrap_or(&self.departement))
    }

    /// L'adresse de la recherche (`base` : [`ADRESSE`], ou le faux serveur d'un essai).
    pub fn adresse(&self, base: &str) -> String {
        let lieu = match &self.code_postal {
            Some(cp) => format!("code_postal={}", encoder(cp)),
            None => format!("departement={}", encoder(&self.departement)),
        };
        format!("{base}?q={}&{lieu}&etat_administratif=A&per_page={PAR_PAGE}", encoder(&self.nom))
    }
}

/// Un texte écrit pour une adresse : lettres et chiffres ASCII, « - », « _ », « . », « ~ » gardés, le reste en
/// `%XX` (les octets UTF-8 ; une espace : `%20`).
pub fn encoder(t: &str) -> String {
    let mut s = String::with_capacity(t.len() * 3);
    for b in t.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

/// L'adresse d'une recherche web du site d'une entreprise (DuckDuckGo) : « Transports Exemple Metz ».
pub fn adresse_recherche_web(nom: &str, commune: &str) -> String {
    let q = format!("{} {}", texte::ligne(nom), texte::ligne(commune));
    format!("https://duckduckgo.com/?q={}", encoder(q.trim()))
}

/// La taille d'une entreprise d'après sa tranche d'effectif (codes de l'INSEE) ; rien pour un code inconnu ou « NN »
/// (pas de salarié déclaré).
pub fn tranche_effectif(code: &str) -> Option<&'static str> {
    Some(match code.trim() {
        "00" => "Aucun salarié",
        "01" => "1 ou 2 salariés",
        "02" => "3 à 5 salariés",
        "03" => "6 à 9 salariés",
        "11" => "10 à 19 salariés",
        "12" => "20 à 49 salariés",
        "21" => "50 à 99 salariés",
        "22" => "100 à 199 salariés",
        "31" => "200 à 249 salariés",
        "32" => "250 à 499 salariés",
        "41" => "500 à 999 salariés",
        "42" => "1 000 à 1 999 salariés",
        "51" => "2 000 à 4 999 salariés",
        "52" => "5 000 à 9 999 salariés",
        "53" => "10 000 salariés et plus",
        _ => return None,
    })
}

/// Le secteur d'après la section de l'activité principale (nomenclature NAF de l'INSEE, A à U).
pub fn secteur(section: &str) -> Option<&'static str> {
    Some(match section.trim().to_ascii_uppercase().as_str() {
        "A" => "Agriculture, forêt et pêche",
        "B" => "Industries extractives",
        "C" => "Industrie manufacturière",
        "D" => "Électricité et gaz",
        "E" => "Eau, assainissement et déchets",
        "F" => "Construction",
        "G" => "Commerce et réparation d'automobiles",
        "H" => "Transports et entreposage",
        "I" => "Hébergement et restauration",
        "J" => "Information et communication",
        "K" => "Banque et assurance",
        "L" => "Immobilier",
        "M" => "Activités spécialisées, scientifiques et techniques",
        "N" => "Services administratifs et de soutien",
        "O" => "Administration publique",
        "P" => "Enseignement",
        "Q" => "Santé humaine et action sociale",
        "R" => "Arts, spectacles et loisirs",
        "S" => "Autres services",
        "T" => "Activités des ménages",
        "U" => "Organisations internationales",
        _ => return None,
    })
}

/// Les noms d'un résultat : nom complet, raison sociale, sigle, enseignes et noms commerciaux (du siège et des
/// établissements trouvés).
fn noms_de(r: &Value) -> Vec<String> {
    let mut l =
        vec![texte_json(r.get("nom_complet")), texte_json(r.get("nom_raison_sociale")), texte_json(r.get("sigle"))];
    let mut lieux: Vec<&Value> = r.get("siege").into_iter().collect();
    if let Some(Value::Array(a)) = r.get("matching_etablissements") {
        lieux.extend(a.iter().take(20));
    }
    for e in lieux {
        l.push(texte_json(e.get("nom_commercial")));
        if let Some(Value::Array(a)) = e.get("liste_enseignes") {
            l.extend(a.iter().take(10).map(|x| texte_json(Some(x))));
        }
    }
    l.into_iter().map(|n| nom_propre(&n)).filter(|n| !n.is_empty()).collect()
}

/// Un nom contenu dans l'autre, mot pour mot (« transports exemple » dans « transports exemple lorraine »), d'au
/// moins 4 lettres.
fn contenu(a: &str, b: &str) -> bool {
    let (court, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    court.chars().filter(|c| c.is_alphanumeric()).count() >= 4 && format!(" {long} ").contains(&format!(" {court} "))
}

/// Le lieu d'un résultat correspond-il (la même commune, ou le même département) ? Le siège ou un établissement.
fn meme_lieu(d: &Demande, r: &Value) -> bool {
    let mut lieux: Vec<&Value> = r.get("siege").into_iter().collect();
    if let Some(Value::Array(a)) = r.get("matching_etablissements") {
        lieux.extend(a.iter().take(20));
    }
    lieux.iter().any(|e| {
        let commune = communes::cle_nom(&texte_json(e.get("libelle_commune")));
        let dep = Some(texte_json(e.get("departement")))
            .filter(|x| !x.is_empty())
            .or_else(|| departement_de(&texte_json(e.get("code_postal"))))
            .unwrap_or_default();
        (!d.commune.is_empty() && commune == d.commune) || (!dep.is_empty() && dep.eq_ignore_ascii_case(&d.departement))
    })
}

/// Lit la réponse de l'annuaire pour cette demande : la fiche de l'entreprise si un résultat correspond clairement,
/// sinon rien ; une réponse qui n'est pas celle de l'annuaire est illisible.
pub fn lire_reponse(d: &Demande, json: &str) -> Result<Option<Fiche>, ErreurAnnuaire> {
    let v: Value = serde_json::from_str(json).map_err(|_| ErreurAnnuaire::Illisible)?;
    let Some(Value::Array(resultats)) = v.get("results") else { return Err(ErreurAnnuaire::Illisible) };
    let cherche = nom_propre(&d.nom);
    // (2 : le même nom ; 1 : un nom contenu dans l'autre)
    let mut retenus: Vec<(u8, &Value)> = Vec::new();
    for r in resultats.iter().take(25) {
        if texte_json(r.get("etat_administratif")).eq_ignore_ascii_case("C") || !meme_lieu(d, r) {
            continue;
        }
        let noms = noms_de(r);
        let note = if noms.contains(&cherche) {
            2
        } else if noms.iter().any(|n| contenu(n, &cherche)) {
            1
        } else {
            continue;
        };
        retenus.push((note, r));
    }
    let Some(meilleure) = retenus.iter().map(|(n, _)| *n).max() else { return Ok(None) };
    let mut siren: Vec<String> = Vec::new();
    let mut choisi = None;
    for (n, r) in retenus.iter().filter(|(n, _)| *n == meilleure) {
        let s = texte_json(r.get("siren"));
        if !siren.contains(&s) {
            siren.push(s);
        }
        choisi.get_or_insert((*n, *r));
    }
    // (deux entreprises différentes qui correspondent autant : pas de certitude, rien)
    if siren.len() != 1 {
        return Ok(None);
    }
    let Some((_, r)) = choisi else { return Ok(None) };
    let nom = texte::ligne(&texte_json(r.get("nom_complet")));
    let nom = if nom.is_empty() { texte::ligne(&texte_json(r.get("nom_raison_sociale"))) } else { nom };
    let taille = match tranche_effectif(&texte_json(r.get("tranche_effectif_salarie"))) {
        Some(t) => {
            let annee = texte_json(r.get("annee_tranche_effectif_salarie"));
            if annee.len() == 4 && annee.bytes().all(|b| b.is_ascii_digit()) {
                format!("{t} (chiffre de {annee})")
            } else {
                t.to_string()
            }
        }
        None => String::new(),
    };
    let siege = r.get("siege");
    Ok(Some(Fiche {
        nom,
        secteur: secteur(&texte_json(r.get("section_activite_principale"))).unwrap_or_default().to_string(),
        taille,
        adresse: texte::ligne(&texte_json(siege.and_then(|s| s.get("adresse")))),
        commune: texte::ligne(&texte_json(siege.and_then(|s| s.get("libelle_commune")))),
    }))
}

/// Demande l'entreprise à l'annuaire (une seule requête, [`DELAI`] au plus, aucune redirection) et lit sa réponse.
/// `base` : [`ADRESSE`] (la construction d'essai donne une adresse locale) ; `https_seulement` : refuse le http.
/// Bloquant : jamais sur le fil de l'écran, jamais dans un test.
pub fn chercher(d: &Demande, base: &str, https_seulement: bool) -> Result<Option<Fiche>, ErreurAnnuaire> {
    let tls = ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::NativeTls)
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(DELAI))
        .timeout_global(Some(DELAI))
        .http_status_as_error(false)
        .https_only(https_seulement)
        .max_redirects(0)
        .user_agent("Jobmind")
        .tls_config(tls)
        .build()
        .into();
    let mut r = agent.get(&d.adresse(base)).header("accept", "application/json").call().map_err(|e| raison(&e))?;
    let statut = r.status().as_u16();
    let texte = r.body_mut().with_config().limit(TAILLE_MAX).read_to_string().map_err(|e| raison(&e))?;
    match statut {
        200 => lire_reponse(d, &texte),
        429 => Err(ErreurAnnuaire::Occupe),
        s => Err(ErreurAnnuaire::Statut(s)),
    }
}

fn raison(e: &ureq::Error) -> ErreurAnnuaire {
    match e {
        ureq::Error::Timeout(_) => ErreurAnnuaire::Delai,
        ureq::Error::Io(io) if io.kind() == std::io::ErrorKind::TimedOut => ErreurAnnuaire::Delai,
        ureq::Error::HostNotFound => ErreurAnnuaire::Reseau("nom du site introuvable".into()),
        ureq::Error::ConnectionFailed => ErreurAnnuaire::Reseau("connexion impossible".into()),
        ureq::Error::Io(io) => ErreurAnnuaire::Reseau(format!("{}", io.kind())),
        ureq::Error::BodyExceedsLimit(_) | ureq::Error::Protocol(_) => ErreurAnnuaire::Illisible,
        _ => ErreurAnnuaire::Reseau("erreur de connexion".into()),
    }
}

#[cfg(test)]
mod tests;
