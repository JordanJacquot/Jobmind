//! Le zip d'une sauvegarde : écrit ici (en-têtes locaux, répertoire central, fin), et relu avec méfiance.
//!
//! L'écriture : chaque partie compressée (« deflate », niveau 6) ou rangée telle quelle si la compression ne gagne rien,
//! sa somme de contrôle (crc32), son nom en UTF-8 (drapeau 0x0800), la date et l'heure du moment donné (format DOS).
//!
//! La lecture : seulement ce que l'écriture produit, en mieux vérifié. Méthodes 0 (rangée) et 8 (deflate) seulement ;
//! chiffrement et zip64 refusés ; chaque partie doit tenir dans le fichier, avant le répertoire central, et porter le
//! même nom dans son en-tête local et dans le répertoire. La décompression est bornée par la taille annoncée : une
//! partie qui se décompresse en plus que ce qu'elle annonce est arrêtée net (un zip piégé ne remplit pas la mémoire).
//! Les limites de taille et de nombre sont vérifiées par l'appelant, avant toute décompression.

use super::ErreurSauvegarde;
use crate::moment::Moment;

const FIN: u32 = 0x0605_4b50;
const REPERTOIRE: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;
/// (le localisateur de la fin zip64, juste avant la fin ordinaire)
const LOCALISATEUR_ZIP64: u32 = 0x0706_4b50;
/// Noms en UTF-8.
const DRAPEAU_UTF8: u16 = 0x0800;
/// Chiffrement (simple ou « fort »).
const DRAPEAUX_CHIFFRE: u16 = 0x0001 | 0x0040;
/// La version du format demandée pour lire (2.0 : deflate).
const VERSION: u16 = 20;

/// Une partie, d'après le répertoire central (le début de ses données vérifié d'après l'en-tête local).
#[derive(Debug, Clone)]
pub(super) struct Partie {
    pub nom: String,
    pub methode: u16,
    pub crc: u32,
    pub taille_compressee: u32,
    pub taille: u32,
    debut: usize,
}

fn abime(r: &str) -> ErreurSauvegarde {
    ErreurSauvegarde::Abimee(format!("zip : {r}"))
}

fn u16_a(o: &[u8], i: usize) -> Result<u16, ErreurSauvegarde> {
    o.get(i..i.saturating_add(2)).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| abime("coupé"))
}

fn u32_a(o: &[u8], i: usize) -> Result<u32, ErreurSauvegarde> {
    o.get(i..i.saturating_add(4)).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| abime("coupé"))
}

/// La date et l'heure au format DOS (de 1980 à 2107, à deux secondes près).
fn date_dos(m: Moment) -> (u16, u16) {
    let annee = m.annee.clamp(1980, 2107) as u32 - 1980;
    let date = (annee << 9) | (m.mois << 5) | m.jour;
    let heure = (m.heure << 11) | (m.minute << 5) | (m.seconde / 2);
    (date as u16, heure as u16)
}

/// Écrit un zip de ces parties (nom, octets), dans l'ordre donné.
pub(super) fn ecrire(parties: &[(&str, &[u8])], maintenant: Moment) -> Result<Vec<u8>, ErreurSauvegarde> {
    let trop = || ErreurSauvegarde::DonneesTropGrosses;
    if parties.len() >= 0xFFFF {
        return Err(trop());
    }
    let (date, heure) = date_dos(maintenant);
    let mut zip = Vec::new();
    let mut central = Vec::new();
    for (nom, clair) in parties {
        let compresse = miniz_oxide::deflate::compress_to_vec(clair, 6);
        let (methode, donnees): (u16, &[u8]) = if compresse.len() < clair.len() { (8, &compresse) } else { (0, clair) };
        let crc = crc32fast::hash(clair);
        let taille = u32::try_from(clair.len()).map_err(|_| trop())?;
        let taille_compressee = u32::try_from(donnees.len()).map_err(|_| trop())?;
        let decalage = u32::try_from(zip.len()).map_err(|_| trop())?;
        let n_nom = u16::try_from(nom.len()).map_err(|_| abime("nom trop long"))?;
        // L'en-tête local
        zip.extend_from_slice(&LOCAL.to_le_bytes());
        zip.extend_from_slice(&VERSION.to_le_bytes());
        zip.extend_from_slice(&DRAPEAU_UTF8.to_le_bytes());
        zip.extend_from_slice(&methode.to_le_bytes());
        zip.extend_from_slice(&heure.to_le_bytes());
        zip.extend_from_slice(&date.to_le_bytes());
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&taille_compressee.to_le_bytes());
        zip.extend_from_slice(&taille.to_le_bytes());
        zip.extend_from_slice(&n_nom.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(nom.as_bytes());
        zip.extend_from_slice(donnees);
        // Sa ligne dans le répertoire central
        central.extend_from_slice(&REPERTOIRE.to_le_bytes());
        central.extend_from_slice(&VERSION.to_le_bytes());
        central.extend_from_slice(&VERSION.to_le_bytes());
        central.extend_from_slice(&DRAPEAU_UTF8.to_le_bytes());
        central.extend_from_slice(&methode.to_le_bytes());
        central.extend_from_slice(&heure.to_le_bytes());
        central.extend_from_slice(&date.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&taille_compressee.to_le_bytes());
        central.extend_from_slice(&taille.to_le_bytes());
        central.extend_from_slice(&n_nom.to_le_bytes());
        // (extra, commentaire, disque, attributs internes, attributs externes)
        central.extend_from_slice(&[0u8; 2 + 2 + 2 + 2 + 4]);
        central.extend_from_slice(&decalage.to_le_bytes());
        central.extend_from_slice(nom.as_bytes());
    }
    let debut_central = u32::try_from(zip.len()).map_err(|_| trop())?;
    let taille_central = u32::try_from(central.len()).map_err(|_| trop())?;
    zip.extend_from_slice(&central);
    let nombre = parties.len() as u16;
    zip.extend_from_slice(&FIN.to_le_bytes());
    zip.extend_from_slice(&[0u8; 4]);
    zip.extend_from_slice(&nombre.to_le_bytes());
    zip.extend_from_slice(&nombre.to_le_bytes());
    zip.extend_from_slice(&taille_central.to_le_bytes());
    zip.extend_from_slice(&debut_central.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    if zip.len() > u32::MAX as usize {
        return Err(trop());
    }
    Ok(zip)
}

/// Lit le répertoire central et vérifie la place de chaque partie (sans rien décompresser). Au plus `parties_max`
/// parties.
pub(super) fn ouvrir(o: &[u8], parties_max: usize) -> Result<Vec<Partie>, ErreurSauvegarde> {
    if !o.starts_with(b"PK") {
        return Err(ErreurSauvegarde::PasUnZip);
    }
    if o.len() < 22 {
        return Err(abime("trop court"));
    }
    // La fin du répertoire : 22 octets, plus un commentaire de 65 535 octets au plus, qui va jusqu'au bout du fichier
    let plus_tot = o.len().saturating_sub(22 + 65_535);
    let fin = (plus_tot..=o.len() - 22)
        .rev()
        .find(|&i| {
            u32_a(o, i).is_ok_and(|s| s == FIN) && u16_a(o, i + 20).is_ok_and(|n| i + 22 + n as usize == o.len())
        })
        .ok_or_else(|| abime("fin du répertoire introuvable"))?;
    if fin >= 20 && u32_a(o, fin - 20)? == LOCALISATEUR_ZIP64 {
        return Err(abime("zip64"));
    }
    let disque = u16_a(o, fin + 4)?;
    let disque_central = u16_a(o, fin + 6)?;
    let nombre_ici = u16_a(o, fin + 8)?;
    let nombre = u16_a(o, fin + 10)?;
    let taille = u32_a(o, fin + 12)?;
    let debut = u32_a(o, fin + 16)?;
    if nombre == 0xFFFF || taille == u32::MAX || debut == u32::MAX {
        return Err(abime("zip64"));
    }
    if disque != 0 || disque_central != 0 || nombre_ici != nombre {
        return Err(abime("zip en plusieurs morceaux"));
    }
    let nombre = nombre as usize;
    if nombre > parties_max {
        return Err(ErreurSauvegarde::Piegee("trop de parties".into()));
    }
    let debut = debut as usize;
    let fin_central =
        debut.checked_add(taille as usize).filter(|&f| f <= fin).ok_or_else(|| abime("répertoire coupé"))?;
    let mut parties = Vec::with_capacity(nombre);
    let mut i = debut;
    for _ in 0..nombre {
        if i.saturating_add(46) > fin_central || u32_a(o, i)? != REPERTOIRE {
            return Err(abime("répertoire central faux"));
        }
        let drapeaux = u16_a(o, i + 8)?;
        let methode = u16_a(o, i + 10)?;
        let crc = u32_a(o, i + 16)?;
        let taille_compressee = u32_a(o, i + 20)?;
        let taille = u32_a(o, i + 24)?;
        let n_nom = u16_a(o, i + 28)? as usize;
        let n_extra = u16_a(o, i + 30)? as usize;
        let n_comm = u16_a(o, i + 32)? as usize;
        let disque = u16_a(o, i + 34)?;
        let decalage = u32_a(o, i + 42)?;
        let suivant = i + 46 + n_nom + n_extra + n_comm;
        if suivant > fin_central {
            return Err(abime("répertoire central coupé"));
        }
        if drapeaux & DRAPEAUX_CHIFFRE != 0 {
            return Err(ErreurSauvegarde::Piegee("partie chiffrée".into()));
        }
        if taille == u32::MAX || taille_compressee == u32::MAX || decalage == u32::MAX || disque == 0xFFFF {
            return Err(abime("zip64"));
        }
        if methode != 0 && methode != 8 {
            return Err(abime(&format!("compression {methode} inconnue")));
        }
        let nom_brut = &o[i + 46..i + 46 + n_nom];
        let nom = std::str::from_utf8(nom_brut)
            .map_err(|_| ErreurSauvegarde::Piegee("nom de partie illisible".into()))?
            .to_string();
        // L'en-tête local : à sa place, avec le même nom, sans chiffrement ; les données avant le répertoire central
        let l = decalage as usize;
        if u32_a(o, l)? != LOCAL {
            return Err(abime("en-tête de partie faux"));
        }
        if u16_a(o, l + 6)? & DRAPEAUX_CHIFFRE != 0 {
            return Err(ErreurSauvegarde::Piegee("partie chiffrée".into()));
        }
        let n_nom_local = u16_a(o, l + 26)? as usize;
        let n_extra_local = u16_a(o, l + 28)? as usize;
        let nom_local = o.get(l + 30..l + 30 + n_nom_local).ok_or_else(|| abime("en-tête de partie coupé"))?;
        if nom_local != nom_brut {
            return Err(ErreurSauvegarde::Piegee("deux noms pour une partie".into()));
        }
        let donnees = l + 30 + n_nom_local + n_extra_local;
        if donnees.checked_add(taille_compressee as usize).is_none_or(|f| f > debut) {
            return Err(abime("partie coupée"));
        }
        parties.push(Partie { nom, methode, crc, taille_compressee, taille, debut: donnees });
        i = suivant;
    }
    Ok(parties)
}

/// Une partie, décompressée (jamais plus que sa taille annoncée) et vérifiée par sa somme de contrôle.
pub(super) fn extraire(o: &[u8], p: &Partie) -> Result<Vec<u8>, ErreurSauvegarde> {
    let donnees =
        o.get(p.debut..p.debut + p.taille_compressee as usize).ok_or_else(|| abime(&format!("{} coupée", p.nom)))?;
    let clair = match p.methode {
        0 => donnees.to_vec(),
        // (un octet de plus que la taille annoncée : une partie qui en donne plus est arrêtée là)
        _ => miniz_oxide::inflate::decompress_to_vec_with_limit(donnees, p.taille as usize + 1)
            .map_err(|_| abime(&format!("{} ne se décompresse pas comme annoncé", p.nom)))?,
    };
    if clair.len() != p.taille as usize {
        return Err(abime(&format!("{} n'a pas la taille annoncée", p.nom)));
    }
    if crc32fast::hash(&clair) != p.crc {
        return Err(abime(&format!("somme de contrôle fausse ({})", p.nom)));
    }
    Ok(clair)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> Moment {
        Moment::nouveau(2026, 11, 19, 14, 5, 7).unwrap()
    }

    #[test]
    fn aller_retour() {
        let gros = "é".repeat(5000);
        let z = ecrire(&[("a.txt", b"bonjour"), ("cv/b.pdf", gros.as_bytes()), ("vide", b"")], m()).unwrap();
        let p = ouvrir(&z, 10).unwrap();
        assert_eq!(p.iter().map(|p| p.nom.as_str()).collect::<Vec<_>>(), ["a.txt", "cv/b.pdf", "vide"]);
        assert_eq!(p[0].methode, 0);
        assert_eq!(p[1].methode, 8);
        assert_eq!(extraire(&z, &p[0]).unwrap(), b"bonjour");
        assert_eq!(extraire(&z, &p[1]).unwrap(), gros.as_bytes());
        assert!(extraire(&z, &p[2]).unwrap().is_empty());
        assert_eq!(date_dos(m()), (((2026 - 1980) << 9 | 11 << 5 | 19) as u16, (14 << 11 | 5 << 5 | 3) as u16));
        assert!(matches!(ouvrir(&z, 2), Err(ErreurSauvegarde::Piegee(_))));
        assert!(matches!(ouvrir(b"", 10), Err(ErreurSauvegarde::PasUnZip)));
    }
}
