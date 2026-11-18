//! Le texte d'un fichier Word (.docx) : un zip de parties XML. Le document principal est trouvé par `_rels/.rels`
//! (sinon `word/document.xml`) ; ses en-têtes viennent avant lui (le nom et les coordonnées y sont souvent), ses pieds
//! de page après, chacun une seule fois. Un zip qui n'est pas un document Word (un tableur, un document LibreOffice)
//! donne [`Erreur::FormatInconnu`].

use crate::Erreur;
use crate::xml::texte_word;
use crate::zip::Zip;

/// Une partie XML en texte (UTF-8, ou UTF-16 avec sa marque).
fn decoder(octets: &[u8]) -> String {
    match octets {
        [0xFF, 0xFE, reste @ ..] => String::from_utf16_lossy(
            &reste.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect::<Vec<_>>(),
        ),
        [0xFE, 0xFF, reste @ ..] => String::from_utf16_lossy(
            &reste.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect::<Vec<_>>(),
        ),
        [0xEF, 0xBB, 0xBF, reste @ ..] => String::from_utf8_lossy(reste).into_owned(),
        _ => String::from_utf8_lossy(octets).into_owned(),
    }
}

/// La valeur d'un attribut dans une balise (`Target="word/document.xml"`).
fn attribut<'a>(balise: &'a str, nom: &str) -> Option<&'a str> {
    let i = balise.find(&format!(" {nom}="))? + nom.len() + 2;
    let q = balise[i..].chars().next()?;
    let reste = &balise[i + 1..];
    reste.find(q).map(|f| &reste[..f])
}

/// Le chemin du document principal, d'après `_rels/.rels`.
fn document_principal(z: &Zip) -> Result<Option<String>, Erreur> {
    let Some(rels) = z.lire("_rels/.rels")? else { return Ok(None) };
    let rels = decoder(&rels);
    for balise in rels.split('<').filter(|b| b.starts_with("Relationship ") || b.contains(":Relationship ")) {
        let principal = attribut(balise, "Type").is_some_and(|t| t.ends_with("/officeDocument"));
        if principal && let Some(cible) = attribut(balise, "Target") {
            return Ok(Some(cible.trim_start_matches('/').to_string()));
        }
    }
    Ok(None)
}

/// Le texte brut d'un .docx.
pub fn texte(octets: &[u8]) -> Result<String, Erreur> {
    let z = Zip::ouvrir(octets)?;
    let chemin = document_principal(&z)?.unwrap_or_else(|| "word/document.xml".into());
    let Some(document) = z.lire(&chemin)? else {
        return Err(Erreur::FormatInconnu);
    };
    // Les en-têtes et les pieds de page, à côté du document (« word/header1.xml »...), dans l'ordre de leur numéro
    let dossier = chemin.rsplit_once('/').map_or(String::new(), |(d, _)| format!("{d}/"));
    let numero = |n: &str, debut: &str| -> Option<u32> {
        n.strip_prefix(&dossier)?.strip_prefix(debut)?.strip_suffix(".xml")?.parse().ok()
    };
    let mut entetes: Vec<(u32, String)> =
        z.noms().filter_map(|n| Some((numero(n, "header")?, n.to_string()))).collect();
    let mut pieds: Vec<(u32, String)> = z.noms().filter_map(|n| Some((numero(n, "footer")?, n.to_string()))).collect();
    entetes.sort();
    pieds.sort();
    let mut blocs: Vec<String> = Vec::new();
    let mut ajouter = |t: String| {
        let t = t.trim().to_string();
        if !t.is_empty() && !blocs.contains(&t) {
            blocs.push(t);
        }
    };
    for (_, n) in &entetes {
        if let Some(x) = z.lire(n)? {
            ajouter(texte_word(&decoder(&x)));
        }
    }
    ajouter(texte_word(&decoder(&document)));
    for (_, n) in &pieds {
        if let Some(x) = z.lire(n)? {
            ajouter(texte_word(&decoder(&x)));
        }
    }
    Ok(blocs.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabrique::{docx_parties, zip};

    #[test]
    fn document_avec_entete_tableau_et_pied() {
        let corps = r#"<w:document><w:body><w:p><w:r><w:t>Expérience</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>2023 à 2025</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Cariste en intérim</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#;
        let entete = r#"<w:hdr><w:p><w:r><w:t>Jordan Exemple, 06 00 00 00 00</w:t></w:r></w:p></w:hdr>"#;
        let d = docx_parties(&[
            ("word/document.xml", corps),
            ("word/header2.xml", entete),
            ("word/header1.xml", entete),
            ("word/footer1.xml", r#"<w:ftr><w:p><w:r><w:t>Page 1</w:t></w:r></w:p></w:ftr>"#),
        ]);
        assert_eq!(
            texte(&d).unwrap(),
            "Jordan Exemple, 06 00 00 00 00\n\nExpérience\n2023 à 2025\nCariste en intérim\n\nPage 1"
        );
    }

    #[test]
    fn document_principal_ailleurs_et_utf16() {
        let rels = r#"<?xml version="1.0"?><Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="/word/document2.xml"/></Relationships>"#;
        let mut xml16 = vec![0xFF, 0xFE];
        for u in "<w:p><w:r><w:t>Texte en UTF-16 é</w:t></w:r></w:p>".encode_utf16() {
            xml16.extend(u.to_le_bytes());
        }
        let z = zip(&[("_rels/.rels", rels.as_bytes(), true), ("word/document2.xml", &xml16, true)]);
        assert_eq!(texte(&z).unwrap(), "Texte en UTF-16 é");
    }

    /// Un zip qui n'est pas un document Word : un document LibreOffice, un tableur.
    #[test]
    fn autres_zips() {
        let odt = zip(&[
            ("mimetype", b"application/vnd.oasis.opendocument.text".as_slice(), false),
            ("content.xml", b"<x/>", true),
        ]);
        assert_eq!(texte(&odt), Err(Erreur::FormatInconnu));
        let xlsx = zip(&[("xl/workbook.xml", b"<x/>".as_slice(), true)]);
        assert_eq!(texte(&xlsx), Err(Erreur::FormatInconnu));
    }
}
