//! « Ce que je cherche » (étape 3 du plan) : les postes ou les mots (n'importe quel métier), le lieu (autour d'une
//! ville, un département, une région, ou partout), les contrats, le télétravail, « aucun diplôme requis », « débutant
//! accepté » et un salaire minimum facultatif. La saisie de la page est vérifiée ici ([`verifier_recherche`]) ; les
//! sources d'offres (étape 4) et le tri (étape 6) s'en serviront.

use crate::candidature::{trop_long, une_ligne};
use crate::lieux::{self, Departement, Region};

/// Les rayons proposés autour d'une ville, en kilomètres.
pub const RAYONS: [u32; 6] = [5, 10, 20, 30, 50, 100];
/// Le rayon de départ.
pub const RAYON_DEFAUT: u32 = 30;
/// Le rayon le plus grand accepté.
pub const RAYON_MAX: u32 = 200;
/// Au plus ce nombre de postes ou de mots.
pub const POSTES_MAX: usize = 12;
/// La longueur d'un poste ou d'un mot (en caractères).
pub const POSTE_MAX: usize = 60;
/// La longueur de ce qui est tapé pour les postes (en caractères).
pub const POSTES_SAISIE_MAX: usize = 800;
/// La longueur d'un nom de ville.
pub const VILLE_MAX: usize = 60;
/// Le salaire minimum, en euros bruts par mois.
pub const SALAIRE_MIN: u32 = 300;
pub const SALAIRE_MAX: u32 = 20_000;

/// Un type de contrat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Contrat {
    Cdi,
    Cdd,
    Interim,
    Alternance,
    Stage,
    Saisonnier,
    Independant,
}

impl Contrat {
    pub const TOUS: [Contrat; 7] = [
        Contrat::Cdi,
        Contrat::Cdd,
        Contrat::Interim,
        Contrat::Alternance,
        Contrat::Stage,
        Contrat::Saisonnier,
        Contrat::Independant,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Contrat::Cdi => "cdi",
            Contrat::Cdd => "cdd",
            Contrat::Interim => "interim",
            Contrat::Alternance => "alternance",
            Contrat::Stage => "stage",
            Contrat::Saisonnier => "saisonnier",
            Contrat::Independant => "independant",
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            Contrat::Cdi => "CDI",
            Contrat::Cdd => "CDD",
            Contrat::Interim => "Intérim",
            Contrat::Alternance => "Alternance",
            Contrat::Stage => "Stage",
            Contrat::Saisonnier => "Saisonnier",
            Contrat::Independant => "Indépendant",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Contrat> {
        Contrat::TOUS.into_iter().find(|c| c.code() == code)
    }
}

/// Le télétravail voulu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Teletravail {
    PeuImporte,
    Possible,
    Complet,
}

impl Teletravail {
    pub const TOUS: [Teletravail; 3] = [Teletravail::PeuImporte, Teletravail::Possible, Teletravail::Complet];

    pub fn code(self) -> &'static str {
        match self {
            Teletravail::PeuImporte => "peu_importe",
            Teletravail::Possible => "possible",
            Teletravail::Complet => "complet",
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            Teletravail::PeuImporte => "Peu importe",
            Teletravail::Possible => "Possible de temps en temps",
            Teletravail::Complet => "Seulement en télétravail",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Teletravail> {
        Teletravail::TOUS.into_iter().find(|t| t.code() == code)
    }
}

/// Le genre de lieu choisi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenreLieu {
    Ville,
    Departement,
    Region,
    Partout,
}

impl GenreLieu {
    pub const TOUS: [GenreLieu; 4] = [GenreLieu::Ville, GenreLieu::Departement, GenreLieu::Region, GenreLieu::Partout];

    pub fn code(self) -> &'static str {
        match self {
            GenreLieu::Ville => "ville",
            GenreLieu::Departement => "departement",
            GenreLieu::Region => "region",
            GenreLieu::Partout => "partout",
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            GenreLieu::Ville => "Autour d'une ville",
            GenreLieu::Departement => "Un département",
            GenreLieu::Region => "Une région",
            GenreLieu::Partout => "Partout en France",
        }
    }

    pub fn depuis_code(code: &str) -> Option<GenreLieu> {
        GenreLieu::TOUS.into_iter().find(|g| g.code() == code)
    }
}

/// Le lieu cherché.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lieu {
    Ville { nom: String, rayon: u32 },
    Departement(&'static Departement),
    Region(&'static Region),
    Partout,
}

impl Lieu {
    pub fn genre(&self) -> GenreLieu {
        match self {
            Lieu::Ville { .. } => GenreLieu::Ville,
            Lieu::Departement(_) => GenreLieu::Departement,
            Lieu::Region(_) => GenreLieu::Region,
            Lieu::Partout => GenreLieu::Partout,
        }
    }

    /// « Metz, 30 km autour », « 57 Moselle », « Grand Est », « Partout en France ».
    pub fn texte(&self) -> String {
        match self {
            Lieu::Ville { nom, rayon } => format!("{nom}, {rayon} km autour"),
            Lieu::Departement(d) => d.libelle(),
            Lieu::Region(r) => r.nom.to_string(),
            Lieu::Partout => "Partout en France".to_string(),
        }
    }
}

/// Ce que Jordan cherche, vérifié.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recherche {
    /// Les postes ou les mots, dans l'ordre tapé, sans doublon
    pub postes: Vec<String>,
    pub lieu: Lieu,
    /// Les contrats voulus, dans l'ordre de [`Contrat::TOUS`] (aucun : tous)
    pub contrats: Vec<Contrat>,
    pub teletravail: Teletravail,
    pub sans_diplome: bool,
    pub debutant: bool,
    /// Le salaire minimum, en euros bruts par mois
    pub salaire_min: Option<u32>,
}

/// Ce que la page a envoyé, tel quel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaisieRecherche {
    pub postes: String,
    pub lieu: String,
    pub ville: String,
    pub rayon: i64,
    pub departement: String,
    pub region: String,
    pub contrats: Vec<String>,
    pub teletravail: String,
    pub sans_diplome: bool,
    pub debutant: bool,
    pub salaire: String,
}

/// Les postes tapés : séparés par des virgules, des points-virgules ou des retours à la ligne ; chacun sur une ligne,
/// sans doublon (sans tenir compte des majuscules).
pub fn lire_postes(t: &str) -> Vec<String> {
    let mut l: Vec<String> = Vec::new();
    for p in t.split([',', ';', '\n', '\r', '/']) {
        let p = une_ligne(p);
        let p = p.trim_matches(|c: char| c == '.' || c == '-' || c.is_whitespace()).to_string();
        if !p.is_empty() && !l.iter().any(|x| x.to_lowercase() == p.to_lowercase()) {
            l.push(p);
        }
    }
    l
}

/// Un nom de ville acceptable : des lettres (accents compris), des espaces, des tirets, des apostrophes et des chiffres
/// (un code postal).
fn ville_valable(v: &str) -> bool {
    v.chars().any(char::is_alphabetic)
        && v.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '\'' | '\u{2019}' | '.' | '(' | ')'))
}

/// Un salaire tapé : « 1800 », « 1 800 », « 1 800 € », « 1800,50 » (les centimes sont laissés). Rien : aucun.
fn lire_salaire(t: &str) -> Result<Option<u32>, String> {
    let s: String =
        t.chars().filter(|c| !c.is_whitespace() && !matches!(c, '€' | '\u{a0}' | '\u{202f}' | '\'')).collect();
    // (« par mois », « brut », « euros » à la fin : laissés)
    let mut s = s.to_lowercase();
    for fin in ["/mois", "parmois", "mois", "/m", "brut", "net", "euros", "euro", "eur", "/"] {
        if let Some(x) = s.strip_suffix(fin) {
            s = x.to_string();
        }
    }
    if s.chars().any(char::is_alphabetic) {
        return Err("Le salaire s'écrit en chiffres, par exemple 1 800.".into());
    }
    if s.is_empty() {
        return Ok(None);
    }
    let entier = s.split([',', '.']).next().unwrap_or_default();
    // (un point des milliers, « 1.800 », compte comme les milliers)
    let entier = if s.contains('.') && s.split('.').nth(1).is_some_and(|x| x.len() == 3) {
        s.replace('.', "")
    } else {
        entier.to_string()
    };
    let n: u32 = entier.parse().map_err(|_| "Le salaire s'écrit en chiffres, par exemple 1 800.".to_string())?;
    if n > SALAIRE_MAX {
        return Err("Écris le salaire par mois (par exemple 1 800), pas par an.".into());
    }
    if n < SALAIRE_MIN {
        return Err(
            "Le salaire minimum semble trop petit : écris-le en euros bruts par mois, par exemple 1 800.".into()
        );
    }
    Ok(Some(n))
}

/// Vérifie ce que la page a envoyé. L'erreur est une phrase pour la bulle de l'écran.
pub fn verifier_recherche(s: &SaisieRecherche) -> Result<Recherche, String> {
    if trop_long(&s.postes, POSTES_SAISIE_MAX) {
        return Err(format!("La liste des postes est trop longue ({POSTES_SAISIE_MAX} caractères au plus)."));
    }
    let postes = lire_postes(&s.postes);
    if postes.is_empty() {
        return Err("Écris au moins un poste ou un mot, par exemple : agent de quai.".into());
    }
    if postes.len() > POSTES_MAX {
        return Err(format!("Garde {POSTES_MAX} postes ou mots au plus."));
    }
    if let Some(p) = postes.iter().find(|p| trop_long(p, POSTE_MAX)) {
        let debut: String = p.chars().take(20).collect();
        return Err(format!(
            "« {debut}... » est trop long pour un poste ({POSTE_MAX} caractères au plus). Sépare les postes par des virgules."
        ));
    }
    let lieu = match GenreLieu::depuis_code(&s.lieu).ok_or("Choisis où tu cherches.")? {
        GenreLieu::Ville => {
            let nom = une_ligne(&s.ville);
            if nom.is_empty() {
                return Err("Écris la ville autour de laquelle tu cherches.".into());
            }
            if trop_long(&nom, VILLE_MAX) || !ville_valable(&nom) {
                return Err("Le nom de la ville semble faux.".into());
            }
            let rayon = u32::try_from(s.rayon).ok().filter(|r| (1..=RAYON_MAX).contains(r));
            let rayon = rayon.ok_or(format!("Choisis une distance autour de la ville ({RAYON_MAX} km au plus)."))?;
            Lieu::Ville { nom, rayon }
        }
        GenreLieu::Departement => {
            Lieu::Departement(lieux::departement(&s.departement).ok_or("Choisis un département dans la liste.")?)
        }
        GenreLieu::Region => Lieu::Region(lieux::region(&s.region).ok_or("Choisis une région dans la liste.")?),
        GenreLieu::Partout => Lieu::Partout,
    };
    let mut contrats = Vec::new();
    for c in &s.contrats {
        let c = Contrat::depuis_code(c).ok_or("Ce type de contrat n'existe pas.")?;
        if !contrats.contains(&c) {
            contrats.push(c);
        }
    }
    contrats.sort();
    let teletravail = Teletravail::depuis_code(&s.teletravail).ok_or("Choisis ce que tu veux pour le télétravail.")?;
    let salaire_min = lire_salaire(&s.salaire)?;
    Ok(Recherche {
        postes,
        lieu,
        contrats,
        teletravail,
        sans_diplome: s.sans_diplome,
        debutant: s.debutant,
        salaire_min,
    })
}

impl Recherche {
    /// La ligne de la liste de Mon profil : les postes, coupés (« Agent de quai, cariste et 2 autres »).
    pub fn resume(&self) -> String {
        let mut t = self.postes.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
        if self.postes.len() > 2 {
            let n = self.postes.len() - 2;
            t.push_str(&format!(" et {n} autre{}", if n > 1 { "s" } else { "" }));
        }
        t
    }

    /// Les contrats en mots (« CDI, CDD », « Tous les contrats »).
    pub fn contrats_texte(&self) -> String {
        if self.contrats.is_empty() {
            "Tous les contrats".into()
        } else {
            self.contrats.iter().map(|c| c.nom()).collect::<Vec<_>>().join(", ")
        }
    }

    /// Le salaire en mots (« 1 800 € brut par mois au moins »), ou rien.
    pub fn salaire_texte(&self) -> Option<String> {
        self.salaire_min.map(|n| format!("{} € brut par mois au moins", milliers(n)))
    }
}

/// « 1 800 » (une espace pour les milliers).
pub fn milliers(n: u32) -> String {
    let s = n.to_string();
    let mut t = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            t.push(' ');
        }
        t.push(c);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saisie() -> SaisieRecherche {
        SaisieRecherche {
            postes: "Agent de quai, cariste ;  préparateur de commandes\nAgent de quai".into(),
            lieu: "ville".into(),
            ville: "  Metz ".into(),
            rayon: 30,
            departement: String::new(),
            region: String::new(),
            contrats: vec!["cdd".into(), "cdi".into(), "cdi".into()],
            teletravail: "peu_importe".into(),
            sans_diplome: true,
            debutant: false,
            salaire: "1 800 €".into(),
        }
    }

    #[test]
    fn une_recherche_complete() {
        let r = verifier_recherche(&saisie()).unwrap();
        assert_eq!(r.postes, ["Agent de quai", "cariste", "préparateur de commandes"]);
        assert_eq!(r.lieu, Lieu::Ville { nom: "Metz".into(), rayon: 30 });
        assert_eq!(r.lieu.texte(), "Metz, 30 km autour");
        assert_eq!(r.contrats, [Contrat::Cdi, Contrat::Cdd]);
        assert_eq!(r.contrats_texte(), "CDI, CDD");
        assert_eq!(r.salaire_min, Some(1800));
        assert_eq!(r.salaire_texte().unwrap(), "1 800 € brut par mois au moins");
        assert_eq!(r.resume(), "Agent de quai, cariste et 1 autre");
        assert!(r.sans_diplome && !r.debutant);
    }

    #[test]
    fn les_autres_lieux() {
        let mut s = saisie();
        s.lieu = "departement".into();
        s.departement = "57".into();
        assert_eq!(verifier_recherche(&s).unwrap().lieu.texte(), "57 Moselle");
        s.lieu = "region".into();
        s.region = "grand_est".into();
        assert_eq!(verifier_recherche(&s).unwrap().lieu.texte(), "Grand Est");
        s.lieu = "partout".into();
        s.ville = String::new();
        assert_eq!(verifier_recherche(&s).unwrap().lieu, Lieu::Partout);
    }

    #[test]
    fn les_salaires_tapes() {
        for (t, n) in [
            ("", None),
            ("1800", Some(1800)),
            ("1 800,50", Some(1800)),
            ("1.800", Some(1800)),
            ("2000 €/mois", Some(2000)),
        ] {
            assert_eq!(lire_salaire(t), Ok(n), "{t}");
        }
        assert!(lire_salaire("24 000").unwrap_err().contains("par mois"));
        assert!(lire_salaire("12").is_err());
        assert!(lire_salaire("beaucoup").is_err());
        assert_eq!(milliers(950), "950");
        assert_eq!(milliers(12_345), "12 345");
    }

    /// Contre-épreuve : chaque saisie fausse est refusée avec une phrase simple.
    #[test]
    fn saisies_refusees() {
        type Changer = Box<dyn Fn(&mut SaisieRecherche)>;
        let cas: Vec<(Changer, &str)> = vec![
            (Box::new(|s| s.postes = " , ; ".into()), "au moins un poste"),
            (Box::new(|s| s.postes = (0..13).map(|i| format!("poste {i}")).collect::<Vec<_>>().join(",")), "12 postes"),
            (Box::new(|s| s.postes = "x".repeat(70)), "trop long pour un poste"),
            (Box::new(|s| s.postes = "x,".repeat(500)), "trop longue"),
            (Box::new(|s| s.lieu = "lune".into()), "où tu cherches"),
            (Box::new(|s| s.ville = String::new()), "Écris la ville"),
            (Box::new(|s| s.ville = "<script>".into()), "semble faux"),
            (Box::new(|s| s.ville = "12345".into()), "semble faux"),
            (Box::new(|s| s.rayon = 0), "distance"),
            (Box::new(|s| s.rayon = 5000), "distance"),
            (
                Box::new(|s| {
                    s.lieu = "departement".into();
                    s.departement = "20".into()
                }),
                "département",
            ),
            (
                Box::new(|s| {
                    s.lieu = "region".into();
                    s.region = "Lorraine".into()
                }),
                "région",
            ),
            (Box::new(|s| s.contrats = vec!["cdi".into(), "esclavage".into()]), "contrat"),
            (Box::new(|s| s.teletravail = String::new()), "télétravail"),
            (Box::new(|s| s.salaire = "36000".into()), "par mois"),
        ];
        for (changer, attendu) in cas {
            let mut s = saisie();
            changer(&mut s);
            let e = verifier_recherche(&s).unwrap_err();
            assert!(e.contains(attendu), "{e} / {attendu}");
            assert!(!e.contains(['\u{2013}', '\u{2014}']));
        }
    }
}
