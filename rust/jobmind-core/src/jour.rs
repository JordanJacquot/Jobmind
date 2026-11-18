//! Les jours et les heures du suivi des candidatures : un jour du calendrier (sans heure), compté en jours depuis le
//! 1er janvier 1970 pour les écarts (« envoyée il y a 16 jours »), écrit en français (« 2 nov. », « lundi 2
//! novembre ») ; une heure de rendez-vous (« 9 h 30 »). L'heure qu'il est est toujours donnée par l'appelant.

/// Les mois, en entier et en court (le point des abréviations compris).
const MOIS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];
const MOIS_COURTS: [&str; 12] =
    ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
/// Les jours de la semaine, du lundi au dimanche.
const SEMAINE: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];

/// Une année bissextile (calendrier grégorien).
fn bissextile(a: i32) -> bool {
    (a % 4 == 0 && a % 100 != 0) || a % 400 == 0
}

/// Le nombre de jours d'un mois.
fn jours_du_mois(a: i32, m: u32) -> u32 {
    match m {
        2 if bissextile(a) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Un jour du calendrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Jour {
    pub annee: i32,
    pub mois: u32,
    pub jour: u32,
}

impl Jour {
    /// Un jour, s'il existe (de l'an 1900 à l'an 9999 : une date plus ancienne est une faute de frappe).
    pub fn nouveau(annee: i32, mois: u32, jour: u32) -> Option<Jour> {
        let ok = (1900..=9999).contains(&annee)
            && (1..=12).contains(&mois)
            && jour >= 1
            && jour <= jours_du_mois(annee, mois);
        ok.then_some(Jour { annee, mois, jour })
    }

    /// Le nombre de jours depuis le 1er janvier 1970 (négatif avant).
    pub fn numero(self) -> i64 {
        // (l'algorithme « days_from_civil » de Howard Hinnant)
        let m = i64::from(self.mois);
        let d = i64::from(self.jour);
        let y = i64::from(self.annee) - i64::from(m <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Le jour de ce numéro ([`Jour::numero`]).
    pub fn depuis_numero(n: i64) -> Jour {
        let z = n + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        Jour { annee: y as i32, mois: m as u32, jour: d as u32 }
    }

    /// Ce jour plus `n` jours (`n` négatif : avant).
    pub fn plus(self, n: i64) -> Jour {
        Jour::depuis_numero(self.numero() + n)
    }

    /// Le nombre de jours d'ici à `autre` (négatif si `autre` est avant).
    pub fn jours_jusqua(self, autre: Jour) -> i64 {
        autre.numero() - self.numero()
    }

    /// Le jour de la semaine : 0 lundi, 6 dimanche.
    pub fn semaine(self) -> usize {
        // (le 1er janvier 1970 était un jeudi)
        (self.numero() + 3).rem_euclid(7) as usize
    }

    /// « 2026-11-02 » (ce que la base garde).
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.annee, self.mois, self.jour)
    }

    /// Un jour écrit « 2026-11-02 », suivi ou non d'une heure (« 2026-11-02T09:30:00 »).
    pub fn lire_iso(t: &str) -> Option<Jour> {
        let d = t.get(..10)?;
        if t.len() > 10 && !t[10..].starts_with('T') {
            return None;
        }
        let b = d.as_bytes();
        if b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        Jour::nouveau(d[..4].parse().ok()?, d[5..7].parse().ok()?, d[8..10].parse().ok()?)
    }

    /// Un jour tapé : « 2/11/2026 », « 02/11/26 », « 2-11-2026 », « 2.11.2026 », ou « 2026-11-02 ».
    pub fn lire_saisie(t: &str) -> Option<Jour> {
        let t = t.trim();
        if let Some(j) = Jour::lire_iso(t).filter(|_| t.len() == 10) {
            return Some(j);
        }
        let parts: Vec<&str> = t.split(['/', '-', '.']).map(str::trim).collect();
        let [j, m, a] = parts.as_slice() else { return None };
        let chiffres = |x: &str, max: usize| !x.is_empty() && x.len() <= max && x.bytes().all(|c| c.is_ascii_digit());
        if !chiffres(j, 2) || !chiffres(m, 2) || !(a.len() == 2 || a.len() == 4) || !chiffres(a, 4) {
            return None;
        }
        let annee: i32 = a.parse().ok()?;
        let annee = if a.len() == 2 { 2000 + annee } else { annee };
        Jour::nouveau(annee, m.parse().ok()?, j.parse().ok()?)
    }

    /// « 02/11/2026 » (ce que le champ montre).
    pub fn saisie(self) -> String {
        format!("{:02}/{:02}/{:04}", self.jour, self.mois, self.annee)
    }

    /// « 2 nov. » (l'année seulement si ce n'est pas celle de `aujourdhui`).
    pub fn court(self, aujourdhui: Jour) -> String {
        let m = MOIS_COURTS[(self.mois - 1) as usize];
        if self.annee == aujourdhui.annee {
            format!("{} {m}", self.jour)
        } else {
            format!("{} {m} {}", self.jour, self.annee)
        }
    }

    /// « lundi 2 novembre » (l'année seulement si ce n'est pas celle de `aujourdhui`).
    pub fn long(self, aujourdhui: Jour) -> String {
        let s = SEMAINE[self.semaine()];
        let m = MOIS[(self.mois - 1) as usize];
        let j = if self.jour == 1 { "1er".to_string() } else { self.jour.to_string() };
        if self.annee == aujourdhui.annee { format!("{s} {j} {m}") } else { format!("{s} {j} {m} {}", self.annee) }
    }

    /// « novembre ».
    pub fn nom_du_mois(self) -> &'static str {
        MOIS[(self.mois - 1) as usize]
    }
}

/// Une heure de rendez-vous.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Heure {
    pub heure: u32,
    pub minute: u32,
}

impl Heure {
    pub fn nouvelle(heure: u32, minute: u32) -> Option<Heure> {
        (heure < 24 && minute < 60).then_some(Heure { heure, minute })
    }

    /// Une heure tapée : « 9 h 30 », « 9h30 », « 09:30 », « 9h », « 14 ».
    pub fn lire(t: &str) -> Option<Heure> {
        let t: String = t.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
        let (h, m) = match t.split_once(['h', ':']) {
            Some((h, m)) => (h, m),
            None => (t.as_str(), ""),
        };
        let chiffres = |x: &str| !x.is_empty() && x.len() <= 2 && x.bytes().all(|c| c.is_ascii_digit());
        if !chiffres(h) || !(m.is_empty() || (chiffres(m) && m.len() == 2)) {
            return None;
        }
        Heure::nouvelle(h.parse().ok()?, if m.is_empty() { 0 } else { m.parse().ok()? })
    }

    /// « 9 h 30 », « 14 h ».
    pub fn texte(self) -> String {
        if self.minute == 0 { format!("{} h", self.heure) } else { format!("{} h {:02}", self.heure, self.minute) }
    }

    /// « 09:30 » (ce que la base garde).
    pub fn hhmm(self) -> String {
        format!("{:02}:{:02}", self.heure, self.minute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j(a: i32, m: u32, d: u32) -> Jour {
        Jour::nouveau(a, m, d).unwrap()
    }

    #[test]
    fn numeros_et_retour() {
        assert_eq!(j(1970, 1, 1).numero(), 0);
        assert_eq!(j(1969, 12, 31).numero(), -1);
        assert_eq!(j(2026, 11, 18).numero(), 20_775);
        // Aller et retour sur plus de deux siècles, jour par jour
        let mut d = j(1900, 1, 1);
        let fin = j(2130, 12, 31);
        let mut n = d.numero();
        while d <= fin {
            assert_eq!(Jour::depuis_numero(n), d);
            n += 1;
            d = d.plus(1);
            assert_eq!(d.numero(), n);
        }
        assert_eq!(j(2026, 11, 2).jours_jusqua(j(2026, 11, 18)), 16);
        assert_eq!(j(2026, 12, 31).plus(1), j(2027, 1, 1));
        assert_eq!(j(2028, 2, 28).plus(1), j(2028, 2, 29));
        assert_eq!(j(2026, 3, 1).plus(-1), j(2026, 2, 28));
    }

    #[test]
    fn jours_de_la_semaine_et_textes() {
        let auj = j(2026, 11, 18);
        assert_eq!(auj.semaine(), 2); // un mercredi
        assert_eq!(auj.long(auj), "mercredi 18 novembre");
        assert_eq!(j(2026, 11, 2).long(auj), "lundi 2 novembre");
        assert_eq!(j(2026, 12, 1).long(auj), "mardi 1er décembre");
        assert_eq!(j(2027, 1, 4).long(auj), "lundi 4 janvier 2027");
        assert_eq!(j(2026, 11, 2).court(auj), "2 nov.");
        assert_eq!(j(2026, 8, 15).court(auj), "15 août");
        assert_eq!(j(2025, 2, 3).court(auj), "3 févr. 2025");
        assert_eq!(auj.iso(), "2026-11-18");
        assert_eq!(j(2026, 3, 4).saisie(), "04/03/2026");
        assert_eq!(auj.nom_du_mois(), "novembre");
    }

    #[test]
    fn jours_lus() {
        let d = Some(j(2026, 11, 2));
        for t in ["2/11/2026", "02/11/2026", " 2/11/26 ", "2-11-2026", "2.11.2026", "2026-11-02"] {
            assert_eq!(Jour::lire_saisie(t), d, "{t}");
        }
        for t in [
            "",
            "2/11",
            "31/11/2026",
            "2/13/2026",
            "29/02/2026",
            "a/11/2026",
            "2/11/202",
            "2026-11-02T09:00",
            "1/1/1899",
        ] {
            assert_eq!(Jour::lire_saisie(t), None, "{t}");
        }
        assert_eq!(Jour::lire_iso("2026-11-02T09:30:00"), d);
        assert_eq!(Jour::lire_iso("2026-11-02"), d);
        assert_eq!(Jour::lire_iso("2026-11-02 09"), None);
        assert_eq!(Jour::lire_iso("2026/11/02"), None);
        assert_eq!(Jour::lire_iso("é"), None);
    }

    #[test]
    fn heures() {
        let h = Some(Heure { heure: 9, minute: 30 });
        for t in ["9 h 30", "9h30", "09:30", " 9H30 "] {
            assert_eq!(Heure::lire(t), h, "{t}");
        }
        assert_eq!(Heure::lire("14"), Some(Heure { heure: 14, minute: 0 }));
        assert_eq!(Heure::lire("14h"), Some(Heure { heure: 14, minute: 0 }));
        for t in ["", "24h", "9h60", "9h5", "neuf", "9:30:00", "123"] {
            assert_eq!(Heure::lire(t), None, "{t}");
        }
        assert_eq!(Heure { heure: 9, minute: 30 }.texte(), "9 h 30");
        assert_eq!(Heure { heure: 14, minute: 0 }.texte(), "14 h");
        assert_eq!(Heure { heure: 9, minute: 5 }.hhmm(), "09:05");
    }
}
