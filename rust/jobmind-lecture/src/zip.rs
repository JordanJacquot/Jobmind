//! Un lecteur de zip réduit à ce qu'un .docx demande : le répertoire central, puis une partie à la fois, rangée telle
//! quelle ou compressée (« deflate »), vérifiée par sa somme de contrôle. Une partie qui se décompresse en plus de
//! [`PARTIE_MAX`] octets est refusée (un zip piégé ne remplit pas la mémoire). Les zips chiffrés sont refusés
//! ([`Erreur::Protege`]) ; les zips de plus de 4 Go (« zip64 ») ne sont pas des CV ([`Erreur::Abime`]).

use crate::Erreur;

/// Une partie décompressée ne dépasse pas cette taille.
pub const PARTIE_MAX: usize = 32 * 1024 * 1024;
/// Un zip n'a pas plus de parties que cela.
pub const PARTIES_MAX: usize = 10_000;

const FIN_REPERTOIRE: u32 = 0x0605_4b50;
const REPERTOIRE: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;

/// Une partie, d'après le répertoire central.
#[derive(Debug, Clone)]
struct Partie {
    nom: String,
    drapeaux: u16,
    methode: u16,
    crc: u32,
    taille_compressee: u32,
    taille: u32,
    decalage: u32,
}

/// Un zip ouvert (ses octets restent ceux de l'appelant).
pub struct Zip<'a> {
    octets: &'a [u8],
    parties: Vec<Partie>,
}

fn abime(r: &str) -> Erreur {
    Erreur::Abime(format!("zip : {r}"))
}

fn u16_a(o: &[u8], i: usize) -> Result<u16, Erreur> {
    o.get(i..i + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| abime("coupé"))
}

fn u32_a(o: &[u8], i: usize) -> Result<u32, Erreur> {
    o.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| abime("coupé"))
}

impl<'a> Zip<'a> {
    /// Lit le répertoire central (à la fin du fichier).
    pub fn ouvrir(octets: &'a [u8]) -> Result<Zip<'a>, Erreur> {
        if octets.len() < 22 {
            return Err(abime("trop court"));
        }
        // (la fin du répertoire : 22 octets, plus un commentaire de 65 535 octets au plus)
        let plus_tot = octets.len().saturating_sub(22 + 65_535);
        let fin = (plus_tot..=octets.len() - 22)
            .rev()
            .find(|&i| u32_a(octets, i).is_ok_and(|s| s == FIN_REPERTOIRE))
            .ok_or_else(|| abime("fin du répertoire introuvable"))?;
        let nombre = u16_a(octets, fin + 10)? as usize;
        let taille = u32_a(octets, fin + 12)?;
        let debut = u32_a(octets, fin + 16)?;
        if nombre == 0xFFFF || taille == u32::MAX || debut == u32::MAX {
            return Err(abime("zip64"));
        }
        if nombre > PARTIES_MAX {
            return Err(abime("trop de parties"));
        }
        let mut parties = Vec::with_capacity(nombre);
        let mut i = debut as usize;
        for _ in 0..nombre {
            if u32_a(octets, i)? != REPERTOIRE {
                return Err(abime("répertoire central faux"));
            }
            let drapeaux = u16_a(octets, i + 8)?;
            let methode = u16_a(octets, i + 10)?;
            let crc = u32_a(octets, i + 16)?;
            let taille_compressee = u32_a(octets, i + 20)?;
            let taille = u32_a(octets, i + 24)?;
            let n_nom = u16_a(octets, i + 28)? as usize;
            let n_extra = u16_a(octets, i + 30)? as usize;
            let n_comm = u16_a(octets, i + 32)? as usize;
            let decalage = u32_a(octets, i + 42)?;
            let nom = octets.get(i + 46..i + 46 + n_nom).ok_or_else(|| abime("nom coupé"))?;
            let nom = String::from_utf8_lossy(nom).replace('\\', "/");
            parties.push(Partie { nom, drapeaux, methode, crc, taille_compressee, taille, decalage });
            i += 46 + n_nom + n_extra + n_comm;
        }
        Ok(Zip { octets, parties })
    }

    /// Les noms des parties.
    pub fn noms(&self) -> impl Iterator<Item = &str> {
        self.parties.iter().map(|p| p.nom.as_str())
    }

    /// Une partie, décompressée et vérifiée (rien si elle n'existe pas). Le nom sans tenir compte des majuscules.
    pub fn lire(&self, nom: &str) -> Result<Option<Vec<u8>>, Erreur> {
        let Some(p) = self.parties.iter().find(|p| p.nom.eq_ignore_ascii_case(nom)) else { return Ok(None) };
        if p.drapeaux & 1 != 0 {
            return Err(Erreur::Protege);
        }
        if p.taille == u32::MAX || p.taille_compressee == u32::MAX || p.decalage == u32::MAX {
            return Err(abime("zip64"));
        }
        let o = self.octets;
        let l = p.decalage as usize;
        if u32_a(o, l)? != LOCAL {
            return Err(abime("en-tête de partie faux"));
        }
        let n_nom = u16_a(o, l + 26)? as usize;
        let n_extra = u16_a(o, l + 28)? as usize;
        let debut = l + 30 + n_nom + n_extra;
        let donnees = o.get(debut..debut + p.taille_compressee as usize).ok_or_else(|| abime("partie coupée"))?;
        let clair = match p.methode {
            0 => donnees.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(donnees, PARTIE_MAX)
                .map_err(|e| abime(&format!("décompression ({:?})", e.status)))?,
            m => return Err(abime(&format!("compression {m} inconnue"))),
        };
        if clair.len() != p.taille as usize {
            return Err(abime("taille fausse"));
        }
        if crc32fast::hash(&clair) != p.crc {
            return Err(abime("somme de contrôle fausse"));
        }
        Ok(Some(clair))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabrique::zip;

    #[test]
    fn parties_rangees_et_compressees() {
        let z = zip(&[("a.txt", b"bonjour".as_slice(), false), ("dossier/b.xml", "é".repeat(5000).as_bytes(), true)]);
        let z = Zip::ouvrir(&z).unwrap();
        assert_eq!(z.noms().collect::<Vec<_>>(), ["a.txt", "dossier/b.xml"]);
        assert_eq!(z.lire("a.txt").unwrap().unwrap(), b"bonjour");
        assert_eq!(z.lire("DOSSIER/B.XML").unwrap().unwrap(), "é".repeat(5000).as_bytes());
        assert!(z.lire("absent").unwrap().is_none());
    }

    /// Contre-épreuve : une somme de contrôle fausse, un zip piégé (un petit fichier qui se décompresse en énorme),
    /// une partie chiffrée.
    #[test]
    fn zips_refuses() {
        let mut z = zip(&[("a.txt", b"bonjour tout le monde".as_slice(), false)]);
        let i = z.windows(7).position(|w| w == b"bonjour").unwrap();
        z[i] = b'B';
        assert!(matches!(Zip::ouvrir(&z).unwrap().lire("a.txt"), Err(Erreur::Abime(r)) if r.contains("contrôle")));
        // 40 Mo de zéros : quelques dizaines de kilo-octets compressés
        let gros = vec![0u8; 40 * 1024 * 1024];
        let z = zip(&[("bombe.xml", gros.as_slice(), true)]);
        assert!(z.len() < 200_000, "{}", z.len());
        assert!(matches!(Zip::ouvrir(&z).unwrap().lire("bombe.xml"), Err(Erreur::Abime(_))));
        let mut z = zip(&[("a.txt", b"secret".as_slice(), false)]);
        // (le drapeau « chiffré » dans le répertoire central)
        let rep = z.windows(4).position(|w| w == REPERTOIRE.to_le_bytes()).unwrap();
        z[rep + 8] |= 1;
        assert_eq!(Zip::ouvrir(&z).unwrap().lire("a.txt").unwrap_err(), Erreur::Protege);
        assert!(Zip::ouvrir(b"PK\x03\x04 pas un zip").is_err());
    }
}
