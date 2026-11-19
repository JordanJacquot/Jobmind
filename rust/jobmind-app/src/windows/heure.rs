//! L'heure locale du PC, pour le journal et etat.json (`Get-Date`).

use std::sync::Arc;

use jobmind_storage::{Horloge, Moment};
use windows::Win32::System::SystemInformation::{GetLocalTime, GetSystemTime};

/// Le 1er janvier 2000 : seulement si Windows rendait une date impossible.
const SECOURS: Moment = Moment { annee: 2000, mois: 1, jour: 1, heure: 0, minute: 0, seconde: 0 };

/// L'heure locale, à la seconde.
pub fn maintenant() -> Moment {
    // SAFETY : simple lecture de l'heure, structure rendue par valeur
    let t = unsafe { GetLocalTime() };
    Moment::nouveau(
        i32::from(t.wYear),
        u32::from(t.wMonth),
        u32::from(t.wDay),
        u32::from(t.wHour),
        u32::from(t.wMinute),
        u32::from(t.wSecond),
    )
    .unwrap_or(SECOURS)
}

/// Le décalage de l'heure locale avec l'heure universelle, en minutes (+60 en hiver à Paris), arrondi au quart
/// d'heure : la date d'un mail envoyé (étape 10).
pub fn decalage_minutes() -> i32 {
    // SAFETY : simples lectures de l'heure, structures rendues par valeur
    let (l, u) = unsafe { (GetLocalTime(), GetSystemTime()) };
    let m = |t: windows::Win32::Foundation::SYSTEMTIME| {
        Moment::nouveau(
            i32::from(t.wYear),
            u32::from(t.wMonth),
            u32::from(t.wDay),
            u32::from(t.wHour),
            u32::from(t.wMinute),
            u32::from(t.wSecond),
        )
    };
    match (m(l), m(u)) {
        (Some(a), Some(b)) => {
            let d = (a.secondes() - b.secondes()) as f64 / 60.0;
            ((d / 15.0).round() * 15.0) as i32
        }
        _ => 0,
    }
}

/// L'horloge du journal.
pub fn horloge() -> Horloge {
    Arc::new(maintenant)
}

#[cfg(test)]
mod tests {
    #[test]
    fn heure_plausible() {
        let m = super::maintenant();
        assert!(m.annee >= 2026, "{m:?}");
        let d = super::decalage_minutes();
        assert!((-14 * 60..=14 * 60).contains(&d) && d % 15 == 0, "{d}");
    }
}
