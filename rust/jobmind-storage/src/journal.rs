//! Le journal de Jobmind (journal.txt, dans le dossier de données) : une ligne datée par événement (`Write-Log`,
//! modules\donnees.ps1). Il sert à aider à distance : chaque échec utile pour comprendre un problème y est noté.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::moment::Moment;

/// L'heure locale : donnée par l'appelant (la lecture de Windows), une horloge figée dans les tests.
pub type Horloge = Arc<dyn Fn() -> Moment + Send + Sync>;

/// Une horloge arrêtée sur ce moment (les tests).
pub fn horloge_figee(m: Moment) -> Horloge {
    Arc::new(move || m)
}

/// Au-delà de cette taille, le journal est raccourci au démarrage.
pub const TAILLE_MAX: u64 = 1024 * 1024;
/// Les lignes gardées quand il est raccourci.
pub const LIGNES_GARDEES: usize = 3000;
/// Au-delà de cette taille, le journal est raccourci tout de suite, au moment d'écrire (une longue analyse écrit
/// des centaines de lignes dans la journée, bien avant le prochain démarrage).
pub const OCTETS_MAX: u64 = 2 * 1024 * 1024;
/// Ce qui est gardé alors : la fin du journal, à partir d'un début de ligne.
pub const OCTETS_GARDES: u64 = 1024 * 1024;

/// Le journal : son fichier et l'horloge qui date ses lignes.
#[derive(Clone)]
pub struct Journal {
    fichier: PathBuf,
    horloge: Horloge,
    /// La taille du fichier, comptée en mémoire (la taille à l'ouverture, plus chaque ligne écrite) : le fichier n'est
    /// regardé que quand ce compte dépasse [`OCTETS_MAX`]. Partagée par les copies du journal ; elle sert aussi de
    /// verrou : deux fils n'écrivent ni ne raccourcissent en même temps.
    taille: Arc<Mutex<u64>>,
}

impl Journal {
    /// Le journal de ce fichier (journal.txt), raccourci s'il dépasse 1 Mo (seulement ses 3000 dernières lignes),
    /// comme au chargement de donnees.ps1, puis par sa taille s'il reste trop gros. Une erreur n'empêche rien.
    pub fn ouvrir(fichier: &Path, horloge: Horloge) -> Journal {
        raccourcir(fichier);
        raccourcir_fin(fichier);
        let taille = std::fs::metadata(fichier).map_or(0, |m| m.len());
        Journal { fichier: fichier.to_path_buf(), horloge, taille: Arc::new(Mutex::new(taille)) }
    }

    /// Le fichier du journal.
    pub fn fichier(&self) -> &Path {
        &self.fichier
    }

    /// L'heure qu'il est, selon l'horloge du journal.
    pub fn maintenant(&self) -> Moment {
        (self.horloge)()
    }

    /// `Write-Log` : une ligne « 2026-10-30T14:05:00 message », ajoutée à la fin (UTF-8 sans BOM). Une erreur est
    /// ignorée (le journal ne doit jamais rien bloquer).
    ///
    /// Le message tient toujours sur une seule ligne : chaque retour à la ligne et chaque caractère de contrôle
    /// devient une espace ([`sur_une_ligne`]), pour qu'un texte venu d'une page de site ne puisse pas glisser une
    /// fausse ligne datée. Quand le fichier dépasse [`OCTETS_MAX`], seule sa fin est gardée ([`OCTETS_GARDES`]).
    pub fn ecrire(&self, message: &str) {
        let ligne = format!("{} {}\r\n", self.maintenant().iso(), sur_une_ligne(message));
        let mut taille = self.taille.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let ecrit = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.fichier)
            .and_then(|mut f| f.write_all(ligne.as_bytes()));
        if ecrit.is_ok() {
            *taille = taille.saturating_add(ligne.len() as u64);
        }
        if *taille > OCTETS_MAX {
            // (le compte en mémoire peut être en avance : le journal a pu être raccourci par ailleurs)
            raccourcir_fin(&self.fichier);
            *taille = std::fs::metadata(&self.fichier).map_or(0, |m| m.len());
        }
    }
}

/// Un message sur une seule ligne : chaque caractère de contrôle (retour à la ligne, tabulation, échappement...) et
/// chaque séparateur de ligne ou de paragraphe (U+2028, U+2029) devient une espace.
pub fn sur_une_ligne(message: &str) -> String {
    message.chars().map(|c| if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') { ' ' } else { c }).collect()
}

/// Les lignes d'un texte comme `[IO.File]::ReadAllLines` : coupées à « \r\n », « \r » ou « \n », sans ligne vide
/// après un dernier retour à la ligne.
pub fn lignes(texte: &str) -> Vec<&str> {
    let mut l = Vec::new();
    let mut debut = 0;
    let o = texte.as_bytes();
    let mut i = 0;
    while i < o.len() {
        match o[i] {
            b'\r' => {
                l.push(&texte[debut..i]);
                i += if o.get(i + 1) == Some(&b'\n') { 2 } else { 1 };
                debut = i;
            }
            b'\n' => {
                l.push(&texte[debut..i]);
                i += 1;
                debut = i;
            }
            _ => i += 1,
        }
    }
    if debut < o.len() {
        l.push(&texte[debut..]);
    }
    l
}

/// Raccourcit le journal s'il dépasse 1 Mo : ses 3000 dernières lignes, réécrites en UTF-8 sans BOM.
pub fn raccourcir(fichier: &Path) {
    let Ok(m) = std::fs::metadata(fichier) else { return };
    if !m.is_file() || m.len() <= TAILLE_MAX {
        return;
    }
    let Ok(octets) = std::fs::read(fichier) else { return };
    let texte = suite_commun::json_sur::texte_du_fichier(&octets);
    let toutes = lignes(&texte);
    let gardees = &toutes[toutes.len().saturating_sub(LIGNES_GARDEES)..];
    let mut sortie = String::new();
    for l in gardees {
        sortie.push_str(l);
        sortie.push_str("\r\n");
    }
    let _ = std::fs::write(fichier, sortie.as_bytes());
}

/// Raccourcit le journal s'il dépasse [`OCTETS_MAX`] : seule sa fin est gardée ([`OCTETS_GARDES`] au plus), à partir
/// d'un début de ligne. Écrit à côté puis remplacé d'un coup ; une erreur n'empêche rien.
pub fn raccourcir_fin(fichier: &Path) {
    let Ok(m) = std::fs::metadata(fichier) else { return };
    if !m.is_file() || m.len() <= OCTETS_MAX {
        return;
    }
    let Ok(octets) = std::fs::read(fichier) else { return };
    let fin = fin_gardee(&octets, OCTETS_GARDES as usize);
    let Some(nom) = fichier.file_name() else { return };
    let mut a_cote = nom.to_os_string();
    a_cote.push(format!(".{}.court", std::process::id()));
    let a_cote = fichier.with_file_name(a_cote);
    if std::fs::write(&a_cote, fin).and_then(|()| std::fs::rename(&a_cote, fichier)).is_err() {
        let _ = std::fs::remove_file(&a_cote);
    }
}

/// La fin d'un journal : ses `gardes` derniers octets au plus, à partir du début d'une ligne (une ligne coupée au
/// milieu n'est pas gardée ; sans aucun retour à la ligne dans cette fin : rien).
fn fin_gardee(octets: &[u8], gardes: usize) -> &[u8] {
    if octets.len() <= gardes {
        return octets;
    }
    let coupe = octets.len() - gardes;
    // (la ligne d'avant finit juste là : la coupe tombe sur un début de ligne)
    if coupe > 0 && octets[coupe - 1] == b'\n' {
        return &octets[coupe..];
    }
    match octets[coupe..].iter().position(|o| *o == b'\n') {
        Some(i) => &octets[coupe + i + 1..],
        None => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lignes_comme_readalllines() {
        assert_eq!(lignes("a\r\nb\nc\rd"), vec!["a", "b", "c", "d"]);
        assert_eq!(lignes("a\r\n"), vec!["a"]);
        assert_eq!(lignes("a\r\n\r\n"), vec!["a", ""]);
        assert!(lignes("").is_empty());
    }

    fn essai(nom: &str) -> PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-journal-{nom}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn journal(dossier: &Path) -> Journal {
        Journal::ouvrir(&dossier.join("journal.txt"), horloge_figee(Moment::nouveau(2026, 11, 26, 9, 0, 0).unwrap()))
    }

    /// Un texte venu d'une page de site ne peut pas glisser une fausse ligne datée : tout reste sur une ligne.
    #[test]
    fn un_message_tient_sur_une_seule_ligne() {
        let d = essai("ligne");
        let j = journal(&d);
        j.ecrire(
            "Offre : titre (Vendeur\r\n2026-11-26T09:00:01 Clés : tout effacé\n\tfin\u{0}\u{1b}[31m\u{85}x\u{2028}y)",
        );
        j.ecrire("Un message simple, avec des accents : é à ç « » €");
        let t = std::fs::read_to_string(j.fichier()).unwrap();
        assert_eq!(
            t,
            "2026-11-26T09:00:00 Offre : titre (Vendeur  2026-11-26T09:00:01 Clés : tout effacé  fin  [31m x y)\r\n\
             2026-11-26T09:00:00 Un message simple, avec des accents : é à ç « » €\r\n"
        );
        assert_eq!(lignes(&t).len(), 2);
        assert_eq!(sur_une_ligne("a\u{7f}b\u{9f}c\u{2029}d"), "a b c d");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Une longue analyse dans la journée : le journal ne dépasse jamais la borne, garde ses dernières lignes entières,
    /// et le compte en mémoire suit le fichier.
    #[test]
    fn raccourci_par_taille_au_moment_d_ecrire() {
        let d = essai("taille");
        let j = journal(&d);
        let long = "x".repeat(1500);
        let mut plus_gros = 0;
        for i in 0..4000 {
            j.ecrire(&format!("ligne {i} {long}"));
            plus_gros = plus_gros.max(std::fs::metadata(j.fichier()).unwrap().len());
        }
        // (4000 lignes de 1500 caractères : 6 Mo écrits ; jamais plus que la borne sur le disque)
        assert!(plus_gros <= OCTETS_MAX, "{plus_gros}");
        let t = std::fs::read_to_string(j.fichier()).unwrap();
        assert!(t.len() as u64 >= OCTETS_GARDES / 2, "{}", t.len());
        let l = lignes(&t);
        // Chaque ligne est entière, et ce sont les dernières, dans l'ordre
        assert!(l.iter().all(|x| x.starts_with("2026-11-26T09:00:00 ligne ") && x.ends_with(&long)));
        assert!(l.last().unwrap().starts_with("2026-11-26T09:00:00 ligne 3999 "));
        let premier: usize = l[0].split(' ').nth(2).unwrap().parse().unwrap();
        assert_eq!(l.len(), 4000 - premier);
        // Le compte en mémoire suit le fichier, aussi pour une copie du journal
        assert_eq!(*j.taille.lock().unwrap(), t.len() as u64);
        j.clone().ecrire("fin");
        assert_eq!(*j.taille.lock().unwrap(), std::fs::metadata(j.fichier()).unwrap().len());
        // Aucun fichier laissé à côté
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Un journal déjà trop gros à l'ouverture, fait d'une ligne sans fin (ses 3000 dernières lignes ne le
    /// raccourcissent pas) : raccourci quand même. Un journal raccourci par ailleurs : le compte se remet d'accord.
    #[test]
    fn journal_enorme_a_l_ouverture_et_compte_remis_d_accord() {
        let d = essai("ouverture");
        let f = d.join("journal.txt");
        let mut gros = "a".repeat(OCTETS_MAX as usize + 10);
        gros.push_str("\r\n2026-11-25T08:00:00 dernière ligne\r\n");
        std::fs::write(&f, &gros).unwrap();
        let j = journal(&d);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "2026-11-25T08:00:00 dernière ligne\r\n");
        // Le compte en mémoire est en avance (le fichier a été raccourci par ailleurs) : rien n'est perdu
        *j.taille.lock().unwrap() = OCTETS_MAX + 1;
        j.ecrire("suite");
        let t = std::fs::read_to_string(&f).unwrap();
        assert_eq!(t, "2026-11-25T08:00:00 dernière ligne\r\n2026-11-26T09:00:00 suite\r\n");
        assert_eq!(*j.taille.lock().unwrap(), t.len() as u64);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn fin_gardee_a_un_debut_de_ligne() {
        assert_eq!(fin_gardee(b"abc\r\ndef\r\n", 100), b"abc\r\ndef\r\n");
        assert_eq!(fin_gardee(b"abc\r\ndef\r\n", 5), b"def\r\n");
        assert_eq!(fin_gardee(b"abc\r\ndef\r\n", 7), b"def\r\n");
        assert_eq!(fin_gardee(b"abc\r\ndef\r\n", 3), b"");
        assert_eq!(fin_gardee(b"abcdef", 3), b"");
    }
}
