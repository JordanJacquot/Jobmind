//! La lecture d'un CV (étape 3 du plan) : le texte d'un PDF ou d'un fichier Word (.docx), en Rust pur, à partir des
//! octets du fichier. Rien d'autre : ni écran, ni Windows, ni réseau, ni disque (l'appli lit le fichier et le copie).
//!
//! - [`lire`] : le format reconnu par ses premiers octets (jamais par le nom du fichier), puis le texte ; chaque
//!   lecture est protégée : un fichier que la bibliothèque PDF ne comprend pas, et qui la ferait s'arrêter net, donne
//!   seulement [`Erreur::Abime`] ;
//! - [`pdf`] : le texte d'un PDF (pdf-extract) ;
//! - [`docx`] : le texte d'un .docx (son zip lu par [`zip`], son XML par [`xml`]) : paragraphes, tableaux, en-têtes,
//!   zones de texte ;
//! - [`Erreur::message`] : la phrase simple à montrer pour chaque fichier qui ne se lit pas (trop gros, scanné,
//!   abîmé, protégé, ancien Word, autre chose).
//!
//! Le texte rendu est brut : jobmind-core (`profil::nettoyer_texte`) le remet au propre.

pub mod docx;
pub mod pdf;
pub mod xml;
pub mod zip;

/// Un fichier plus gros n'est pas lu (un CV pèse d'habitude moins de 2 Mo).
pub const TAILLE_MAX: u64 = 15 * 1024 * 1024;
/// Un texte qui a moins de lettres et de chiffres que cela est « sans texte » (un PDF scanné).
pub const LETTRES_MIN: usize = 20;

/// Le format d'un fichier lu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Pdf,
    Docx,
}

impl Format {
    /// Le code gardé dans la base (table `cv`, colonne `type_fichier`) et l'extension du fichier copié.
    pub fn code(self) -> &'static str {
        match self {
            Format::Pdf => "pdf",
            Format::Docx => "docx",
        }
    }

    /// « PDF », « Word ».
    pub fn nom(self) -> &'static str {
        match self {
            Format::Pdf => "PDF",
            Format::Docx => "Word",
        }
    }
}

/// Ce que la lecture a donné.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lu {
    pub format: Format,
    /// Le texte brut (à remettre au propre)
    pub texte: String,
}

/// Pourquoi un fichier ne se lit pas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Erreur {
    /// Plus de [`TAILLE_MAX`] octets
    TropGros,
    /// Un fichier vide
    Vide,
    /// Ni un PDF, ni un .docx
    FormatInconnu,
    /// Un ancien fichier Word (.doc), ou un .docx protégé par un mot de passe (les deux sont des fichiers « OLE »)
    AncienWord,
    /// Protégé par un mot de passe
    Protege,
    /// Abîmé ou incompris (la raison, pour le journal)
    Abime(String),
    /// Le fichier se lit, mais ne contient pas de texte (un PDF scanné, des images)
    SansTexte(Format),
}

impl Erreur {
    /// La phrase pour l'écran, en mots simples.
    pub fn message(&self) -> String {
        match self {
            Erreur::TropGros => {
                "Ce fichier est trop gros (plus de 15 Mo). Un CV pèse d'habitude moins de 2 Mo : choisis \
                                 un autre fichier, ou colle le texte de ton CV."
                    .into()
            }
            Erreur::Vide => "Ce fichier est vide. Choisis un autre fichier, ou colle le texte de ton CV.".into(),
            Erreur::FormatInconnu => {
                "Jobmind lit les CV en PDF ou en Word (.docx). Choisis un fichier de ce genre, ou \
                                      colle le texte de ton CV."
                    .into()
            }
            Erreur::AncienWord => {
                "Ce fichier Word est d'un ancien format, ou protégé par un mot de passe : Jobmind ne \
                                   peut pas le lire. Colle le texte de ton CV, ou choisis sa version PDF."
                    .into()
            }
            Erreur::Protege => {
                "Ce fichier est protégé par un mot de passe : Jobmind ne peut pas le lire. Colle le texte \
                                de ton CV, ou choisis une version sans mot de passe."
                    .into()
            }
            Erreur::Abime(_) => {
                "Ce fichier semble abîmé : Jobmind n'a pas pu le lire. Choisis un autre fichier, ou colle \
                                 le texte de ton CV."
                    .into()
            }
            Erreur::SansTexte(Format::Pdf) => {
                "Ce PDF ne contient que des images (un document scanné) : Jobmind ne peut \
                                               pas en lire le texte. Colle le texte de ton CV à la main."
                    .into()
            }
            Erreur::SansTexte(Format::Docx) => {
                "Ce fichier Word ne contient pas de texte (seulement des images) : colle \
                                                le texte de ton CV à la main."
                    .into()
            }
        }
    }

    /// La raison pour le journal (jamais le contenu du fichier).
    pub fn raison(&self) -> String {
        match self {
            Erreur::TropGros => "trop gros".into(),
            Erreur::Vide => "vide".into(),
            Erreur::FormatInconnu => "format inconnu".into(),
            Erreur::AncienWord => "ancien Word ou protégé".into(),
            Erreur::Protege => "protégé par un mot de passe".into(),
            Erreur::Abime(r) => format!("abîmé ({})", r.chars().take(160).collect::<String>()),
            Erreur::SansTexte(f) => format!("{} sans texte", f.nom()),
        }
    }
}

/// Le format d'après les premiers octets.
pub fn format_de(octets: &[u8]) -> Result<Format, Erreur> {
    if octets.is_empty() {
        return Err(Erreur::Vide);
    }
    // (un PDF peut avoir quelques octets avant « %PDF- » : les lecteurs cherchent dans le premier kilo-octet)
    let debut = &octets[..octets.len().min(1024)];
    if debut.windows(5).any(|w| w == b"%PDF-") {
        return Ok(Format::Pdf);
    }
    if octets.starts_with(b"PK\x03\x04") || octets.starts_with(b"PK\x05\x06") {
        return Ok(Format::Docx);
    }
    if octets.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        return Err(Erreur::AncienWord);
    }
    Err(Erreur::FormatInconnu)
}

/// Lit le texte d'un fichier (ses octets). Ne s'arrête jamais net : une bibliothèque qui s'arrêterait sur un fichier
/// incompris donne [`Erreur::Abime`].
pub fn lire(octets: &[u8]) -> Result<Lu, Erreur> {
    if octets.len() as u64 > TAILLE_MAX {
        return Err(Erreur::TropGros);
    }
    let format = format_de(octets)?;
    let texte = std::panic::catch_unwind(|| match format {
        Format::Pdf => pdf::texte(octets),
        Format::Docx => docx::texte(octets),
    })
    .unwrap_or_else(|p| {
        let r = p
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "arrêt de la lecture".into());
        Err(Erreur::Abime(r))
    })?;
    let lettres = texte.chars().filter(|c| c.is_alphanumeric()).take(LETTRES_MIN).count();
    if lettres < LETTRES_MIN {
        return Err(Erreur::SansTexte(format));
    }
    Ok(Lu { format, texte })
}

#[cfg(test)]
pub(crate) mod fabrique;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabrique::{docx_simple, pdf_avec_image, pdf_texte};

    #[test]
    fn formats_reconnus_par_leurs_octets() {
        assert_eq!(format_de(b"%PDF-1.7\n"), Ok(Format::Pdf));
        assert_eq!(format_de(b"\xEF\xBB\xBF%PDF-1.4"), Ok(Format::Pdf));
        assert_eq!(format_de(b"PK\x03\x04..."), Ok(Format::Docx));
        assert_eq!(format_de(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0]), Err(Erreur::AncienWord));
        assert_eq!(format_de(b""), Err(Erreur::Vide));
        assert_eq!(format_de(b"{\\rtf1 Jordan}"), Err(Erreur::FormatInconnu));
        assert_eq!(format_de(b"\x89PNG\r\n"), Err(Erreur::FormatInconnu));
    }

    #[test]
    fn un_pdf_et_un_docx_de_bout_en_bout() {
        let lu = lire(&pdf_texte(&["Jordan Exemple", "Cariste, CACES 1, 3 et 5", "Préparateur de commandes à Woippy"]))
            .unwrap();
        assert_eq!(lu.format, Format::Pdf);
        assert!(lu.texte.contains("Jordan Exemple"), "{:?}", lu.texte);
        assert!(lu.texte.contains("Préparateur de commandes à Woippy"), "{:?}", lu.texte);
        let lu = lire(&docx_simple(&["Jordan Exemple", "Vendeur à Metz, 2021 à 2022"])).unwrap();
        assert_eq!(lu.format, Format::Docx);
        assert_eq!(lu.texte.trim(), "Jordan Exemple\nVendeur à Metz, 2021 à 2022");
    }

    /// Un mot coupé en deux morceaux juste après une lettre de largeur inconnue (« Compé » puis « tences ») reste un mot ;
    /// deux mots écartés d'une vraie espace (0,3 de la taille des lettres) restent deux mots. Mesuré sur un CV imprimé
    /// en PDF par Edge : pdf-extract seul écrivait « Compé tences », « Expér ience ».
    #[test]
    fn mots_coupes_apres_une_lettre_accentuee() {
        // (12 points, 0,6 par lettre : 7,2 points par lettre ; « é » en fait autant en vrai, 0 selon la police)
        let l = 7.2;
        let pdf = fabrique::pdf_morceaux(&[
            (72.0, "Compé"),
            (72.0 + 5.0 * l, "tences"),
            (72.0 + 11.0 * l + 3.6, "Agent"),
            (72.0 + 16.0 * l + 3.6 + 3.6, "de"),
        ]);
        let t = pdf::texte(&pdf).unwrap();
        assert_eq!(t.trim(), "Compétences Agent de", "{t:?}");
        // (pdf-extract seul : le mot coupé)
        assert!(pdf_extract::extract_text_from_mem(&pdf).unwrap().contains("Compé tences"));
    }
    /// Un PDF scanné (une image, aucun texte) : un message qui dit de coller le texte.
    #[test]
    fn un_pdf_scanne() {
        let e = lire(&pdf_avec_image()).unwrap_err();
        assert_eq!(e, Erreur::SansTexte(Format::Pdf));
        assert!(e.message().contains("Colle le texte"));
    }

    /// Contre-épreuve : des fichiers abîmés ou piégés donnent une erreur simple, jamais un arrêt net.
    #[test]
    fn fichiers_abimes() {
        let pdf = pdf_texte(&["Jordan Exemple, cariste depuis 2023 à Woippy"]);
        let docx = docx_simple(&["Jordan Exemple, cariste depuis 2023 à Woippy"]);
        // Coupés en deux, à moitié remplis de zéros, un octet sur trois changé
        for f in [&pdf, &docx] {
            let moitie = &f[..f.len() / 2];
            assert!(lire(moitie).is_err());
            let mut zeros = f.clone();
            let n = zeros.len();
            zeros[n / 3..].fill(0);
            assert!(lire(&zeros).is_err());
            let mut bruit = f.clone();
            for (i, o) in bruit.iter_mut().enumerate().skip(16) {
                if i % 3 == 0 {
                    *o = o.wrapping_mul(31).wrapping_add(7);
                }
            }
            let _ = lire(&bruit);
        }
        // Un « PDF » qui n'en a que le début
        assert!(lire(b"%PDF-1.4\n1 0 obj << >> endobj\n%%EOF").is_err());
        // Trop gros : refusé sans être lu
        assert_eq!(lire(&vec![b'%'; TAILLE_MAX as usize + 1]), Err(Erreur::TropGros));
        for e in [Erreur::TropGros, Erreur::Vide, Erreur::FormatInconnu, Erreur::AncienWord, Erreur::Protege] {
            assert!(!e.message().contains(['\u{2013}', '\u{2014}']));
            assert!(e.message().contains("colle") || e.message().contains("Colle"), "{}", e.message());
        }
    }
}
