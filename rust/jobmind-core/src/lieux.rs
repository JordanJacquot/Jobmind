//! Les lieux de « Ce que je cherche » (étape 3 du plan) : les 18 régions et les 101 départements français, avec leur
//! code. L'écran les propose dans une liste ; la saisie est vérifiée contre ces listes ([`region`], [`departement`]).
//! Les sources d'offres (étape 4) s'en serviront pour chercher au bon endroit.

/// Une région : son code (gardé dans la base) et son nom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub code: &'static str,
    pub nom: &'static str,
}

/// Un département : son numéro (« 57 », « 2A », « 974 »), son nom et le code de sa région.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Departement {
    pub code: &'static str,
    pub nom: &'static str,
    pub region: &'static str,
}

const fn r(code: &'static str, nom: &'static str) -> Region {
    Region { code, nom }
}

/// Les régions, dans l'ordre alphabétique (les cinq d'outre-mer à la fin).
pub const REGIONS: [Region; 18] = [
    r("auvergne_rhone_alpes", "Auvergne-Rhône-Alpes"),
    r("bourgogne_franche_comte", "Bourgogne-Franche-Comté"),
    r("bretagne", "Bretagne"),
    r("centre_val_de_loire", "Centre-Val de Loire"),
    r("corse", "Corse"),
    r("grand_est", "Grand Est"),
    r("hauts_de_france", "Hauts-de-France"),
    r("ile_de_france", "Île-de-France"),
    r("normandie", "Normandie"),
    r("nouvelle_aquitaine", "Nouvelle-Aquitaine"),
    r("occitanie", "Occitanie"),
    r("pays_de_la_loire", "Pays de la Loire"),
    r("provence_alpes_cote_d_azur", "Provence-Alpes-Côte d'Azur"),
    r("guadeloupe", "Guadeloupe"),
    r("martinique", "Martinique"),
    r("guyane", "Guyane"),
    r("la_reunion", "La Réunion"),
    r("mayotte", "Mayotte"),
];

const ARA: &str = "auvergne_rhone_alpes";
const BFC: &str = "bourgogne_franche_comte";
const BRE: &str = "bretagne";
const CVL: &str = "centre_val_de_loire";
const COR: &str = "corse";
const GE: &str = "grand_est";
const HDF: &str = "hauts_de_france";
const IDF: &str = "ile_de_france";
const NOR: &str = "normandie";
const NAQ: &str = "nouvelle_aquitaine";
const OCC: &str = "occitanie";
const PDL: &str = "pays_de_la_loire";
const PACA: &str = "provence_alpes_cote_d_azur";

const fn d(code: &'static str, nom: &'static str, region: &'static str) -> Departement {
    Departement { code, nom, region }
}

/// Les départements, dans l'ordre de leur numéro.
pub const DEPARTEMENTS: [Departement; 101] = [
    d("01", "Ain", ARA),
    d("02", "Aisne", HDF),
    d("03", "Allier", ARA),
    d("04", "Alpes-de-Haute-Provence", PACA),
    d("05", "Hautes-Alpes", PACA),
    d("06", "Alpes-Maritimes", PACA),
    d("07", "Ardèche", ARA),
    d("08", "Ardennes", GE),
    d("09", "Ariège", OCC),
    d("10", "Aube", GE),
    d("11", "Aude", OCC),
    d("12", "Aveyron", OCC),
    d("13", "Bouches-du-Rhône", PACA),
    d("14", "Calvados", NOR),
    d("15", "Cantal", ARA),
    d("16", "Charente", NAQ),
    d("17", "Charente-Maritime", NAQ),
    d("18", "Cher", CVL),
    d("19", "Corrèze", NAQ),
    d("2A", "Corse-du-Sud", COR),
    d("2B", "Haute-Corse", COR),
    d("21", "Côte-d'Or", BFC),
    d("22", "Côtes-d'Armor", BRE),
    d("23", "Creuse", NAQ),
    d("24", "Dordogne", NAQ),
    d("25", "Doubs", BFC),
    d("26", "Drôme", ARA),
    d("27", "Eure", NOR),
    d("28", "Eure-et-Loir", CVL),
    d("29", "Finistère", BRE),
    d("30", "Gard", OCC),
    d("31", "Haute-Garonne", OCC),
    d("32", "Gers", OCC),
    d("33", "Gironde", NAQ),
    d("34", "Hérault", OCC),
    d("35", "Ille-et-Vilaine", BRE),
    d("36", "Indre", CVL),
    d("37", "Indre-et-Loire", CVL),
    d("38", "Isère", ARA),
    d("39", "Jura", BFC),
    d("40", "Landes", NAQ),
    d("41", "Loir-et-Cher", CVL),
    d("42", "Loire", ARA),
    d("43", "Haute-Loire", ARA),
    d("44", "Loire-Atlantique", PDL),
    d("45", "Loiret", CVL),
    d("46", "Lot", OCC),
    d("47", "Lot-et-Garonne", NAQ),
    d("48", "Lozère", OCC),
    d("49", "Maine-et-Loire", PDL),
    d("50", "Manche", NOR),
    d("51", "Marne", GE),
    d("52", "Haute-Marne", GE),
    d("53", "Mayenne", PDL),
    d("54", "Meurthe-et-Moselle", GE),
    d("55", "Meuse", GE),
    d("56", "Morbihan", BRE),
    d("57", "Moselle", GE),
    d("58", "Nièvre", BFC),
    d("59", "Nord", HDF),
    d("60", "Oise", HDF),
    d("61", "Orne", NOR),
    d("62", "Pas-de-Calais", HDF),
    d("63", "Puy-de-Dôme", ARA),
    d("64", "Pyrénées-Atlantiques", NAQ),
    d("65", "Hautes-Pyrénées", OCC),
    d("66", "Pyrénées-Orientales", OCC),
    d("67", "Bas-Rhin", GE),
    d("68", "Haut-Rhin", GE),
    d("69", "Rhône", ARA),
    d("70", "Haute-Saône", BFC),
    d("71", "Saône-et-Loire", BFC),
    d("72", "Sarthe", PDL),
    d("73", "Savoie", ARA),
    d("74", "Haute-Savoie", ARA),
    d("75", "Paris", IDF),
    d("76", "Seine-Maritime", NOR),
    d("77", "Seine-et-Marne", IDF),
    d("78", "Yvelines", IDF),
    d("79", "Deux-Sèvres", NAQ),
    d("80", "Somme", HDF),
    d("81", "Tarn", OCC),
    d("82", "Tarn-et-Garonne", OCC),
    d("83", "Var", PACA),
    d("84", "Vaucluse", PACA),
    d("85", "Vendée", PDL),
    d("86", "Vienne", NAQ),
    d("87", "Haute-Vienne", NAQ),
    d("88", "Vosges", GE),
    d("89", "Yonne", BFC),
    d("90", "Territoire de Belfort", BFC),
    d("91", "Essonne", IDF),
    d("92", "Hauts-de-Seine", IDF),
    d("93", "Seine-Saint-Denis", IDF),
    d("94", "Val-de-Marne", IDF),
    d("95", "Val-d'Oise", IDF),
    d("971", "Guadeloupe", "guadeloupe"),
    d("972", "Martinique", "martinique"),
    d("973", "Guyane", "guyane"),
    d("974", "La Réunion", "la_reunion"),
    d("976", "Mayotte", "mayotte"),
];

/// Une région d'après son code.
pub fn region(code: &str) -> Option<&'static Region> {
    REGIONS.iter().find(|r| r.code == code)
}

/// Un département d'après son numéro (« 57 », « 2a » ou « 2A », « 974 » ; « 1 » pour « 01 »).
pub fn departement(code: &str) -> Option<&'static Departement> {
    let c = code.trim().to_ascii_uppercase();
    let c = if c.len() == 1 && c.bytes().all(|b| b.is_ascii_digit()) { format!("0{c}") } else { c };
    DEPARTEMENTS.iter().find(|d| d.code == c)
}

impl Departement {
    /// « 57 Moselle », comme dans la liste de l'écran.
    pub fn libelle(&self) -> String {
        format!("{} {}", self.code, self.nom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listes_completes_et_coherentes() {
        // 96 départements en métropole (Corse : 2A et 2B), 5 outre-mer
        assert_eq!(DEPARTEMENTS.len(), 101);
        let mut codes: Vec<&str> = DEPARTEMENTS.iter().map(|d| d.code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 101, "un numéro en double");
        // Chaque département est dans une région connue, chaque région a au moins un département
        for d in DEPARTEMENTS {
            assert!(region(d.region).is_some(), "{}", d.code);
        }
        for r in REGIONS {
            assert!(DEPARTEMENTS.iter().any(|d| d.region == r.code), "{}", r.code);
            assert!(r.code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'), "{}", r.code);
        }
        assert_eq!(DEPARTEMENTS.iter().filter(|d| d.region == GE).count(), 10);
        assert_eq!(DEPARTEMENTS.iter().filter(|d| d.region == IDF).count(), 8);
        // Aucun tiret long dans les noms
        for n in REGIONS.iter().map(|r| r.nom).chain(DEPARTEMENTS.iter().map(|d| d.nom)) {
            assert!(!n.contains(['\u{2013}', '\u{2014}']), "{n}");
        }
    }

    #[test]
    fn lecture_d_un_code() {
        assert_eq!(departement("57").unwrap().nom, "Moselle");
        assert_eq!(departement(" 2a ").unwrap().nom, "Corse-du-Sud");
        assert_eq!(departement("1").unwrap().code, "01");
        assert_eq!(departement("974").unwrap().libelle(), "974 La Réunion");
        for faux in ["20", "00", "96", "975", "", "Moselle", "5 7"] {
            assert!(departement(faux).is_none(), "{faux}");
        }
        assert_eq!(region("grand_est").unwrap().nom, "Grand Est");
        assert!(region("Grand Est").is_none());
    }
}
