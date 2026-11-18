//! Le texte d'un PDF, par pdf-extract (licence MIT), page après page, dans l'ordre où il est dessiné. Un PDF protégé
//! par un mot de passe donne [`Erreur::Protege`] ; un PDF que pdf-extract ne comprend pas, [`Erreur::Abime`]. Un PDF
//! sans mot de passe d'ouverture, mais chiffré (seulement protégé contre la copie), se lit.
//!
//! Les caractères sont remis en texte ici ([`Sortie`], comme `PlainTextOutput` de pdf-extract) : un retour à la ligne
//! quand le texte descend, une espace quand deux morceaux de texte sont écartés. pdf-extract met une espace dès un
//! écart d'un dixième de la taille des lettres ; les PDF faits par un navigateur ou par Word coupent souvent leurs
//! mots en morceaux autour d'une lettre accentuée (« Compé tences », mesuré le 18/11/2026 sur un CV imprimé en PDF par
//! Edge). Ici, l'écart doit dépasser [`SEUIL_ESPACE`] de la taille des lettres (une vraie espace en fait 0,25 à 0,3).

use pdf_extract::{Document, MediaBox, OutputDev, OutputError, Transform};

use crate::Erreur;

/// L'écart (en part de la taille des lettres) au-delà duquel deux morceaux de texte sont deux mots.
pub const SEUIL_ESPACE: f64 = 0.18;
/// La largeur d'une lettre dont le PDF ne donne pas la largeur (en part de la taille des lettres).
pub const LARGEUR_INCONNUE: f64 = 0.5;

/// Les caractères d'un PDF remis en texte.
struct Sortie {
    texte: String,
    seuil: f64,
    fin: f64,
    y: f64,
    debut_morceau: bool,
    /// Les largeurs inconnues du morceau en cours (en part de la taille des lettres)
    manque: f64,
    /// (les mesures : l'écart de chaque début de morceau, en part de la taille des lettres)
    ecarts: Vec<f64>,
}

impl OutputDev for Sortie {
    fn begin_page(&mut self, _: u32, _: &MediaBox, _: Option<(f64, f64, f64, f64)>) -> Result<(), OutputError> {
        self.fin = 100_000.;
        Ok(())
    }
    fn end_page(&mut self) -> Result<(), OutputError> {
        if !self.texte.ends_with('\n') && !self.texte.is_empty() {
            self.texte.push('\n');
        }
        Ok(())
    }
    fn output_character(
        &mut self,
        trm: &Transform,
        largeur: f64,
        _: f64,
        taille: f64,
        c: &str,
    ) -> Result<(), OutputError> {
        // (la place sur la page, y vers le bas ; la taille des lettres après la transformation)
        let (x, y) = (trm.m31, -trm.m32);
        let (tx, ty) = (taille * (trm.m11 + trm.m21), taille * (trm.m12 + trm.m22));
        let t = (tx * ty).abs().sqrt().max(0.01);
        if self.debut_morceau {
            // (comme pdf-extract : une ligne vide quand le texte descend beaucoup, un retour quand il revient à gauche)
            let descend = (y - self.y).abs() > t * 1.5;
            if descend {
                self.texte.push('\n');
            }
            if x < self.fin && (y - self.y).abs() > t * 0.5 {
                self.texte.push('\n');
            } else if !descend {
                let ecart = (x - self.fin) / t;
                self.ecarts.push(ecart);
                if ecart > self.seuil && !self.texte.ends_with([' ', '\n']) && c != " " {
                    self.texte.push(' ');
                }
            }
        }
        self.texte.push_str(c);
        self.debut_morceau = false;
        self.y = y;
        // (une largeur inconnue, 0 : celle d'une lettre moyenne. pdf-extract place les lettres suivantes du morceau sans
        // elle : le manque compte jusqu'à la fin du morceau, sinon le mot serait coupé au morceau suivant)
        if largeur <= 0.001 {
            self.manque += LARGEUR_INCONNUE;
        }
        self.fin = x + (largeur + self.manque) * t;
        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> {
        self.debut_morceau = true;
        self.manque = 0.;
        Ok(())
    }
    fn end_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_line(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
}

/// Le texte brut d'un PDF, avec ce seuil d'espace ; et les écarts mesurés (les essais seulement).
#[doc(hidden)]
pub fn texte_mesure(octets: &[u8], seuil: f64) -> Result<(String, Vec<f64>), Erreur> {
    let r = (|| -> Result<Sortie, OutputError> {
        let mut doc = Document::load_mem(octets).map_err(OutputError::PdfError)?;
        if doc.is_encrypted() {
            doc.decrypt("").map_err(OutputError::PdfError)?;
        }
        let mut s = Sortie {
            texte: String::new(),
            seuil,
            fin: 100_000.,
            y: 0.,
            debut_morceau: false,
            manque: 0.,
            ecarts: Vec::new(),
        };
        pdf_extract::output_doc(&doc, &mut s)?;
        Ok(s)
    })();
    match r {
        Ok(s) => Ok((s.texte, s.ecarts)),
        Err(e) => {
            let r = e.to_string();
            let chiffre = octets.windows(8).any(|w| w == b"/Encrypt");
            if chiffre && (r.contains("ecrypt") || r.contains("assword")) {
                Err(Erreur::Protege)
            } else {
                Err(Erreur::Abime(r))
            }
        }
    }
}

/// Le texte brut d'un PDF.
pub fn texte(octets: &[u8]) -> Result<String, Erreur> {
    texte_mesure(octets, SEUIL_ESPACE).map(|(t, _)| t)
}
