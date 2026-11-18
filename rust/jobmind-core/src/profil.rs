//! Le profil de Jordan (étape 3 du plan) : ses CV (collés, ou lus dans un PDF ou un fichier Word) et ses textes pour
//! le style. La saisie de la page est vérifiée ici ; le texte lu dans un fichier est remis au propre
//! ([`nettoyer_texte`]) avant d'être montré, puis Jordan le corrige s'il le faut.

use crate::candidature::{trop_long, une_ligne};

/// Au plus ce nombre de CV gardés.
pub const CV_NOMBRE_MAX: usize = 5;
/// Le nom d'un CV (en caractères).
pub const CV_NOM_MAX: usize = 80;
/// Le texte d'un CV (en caractères) : un CV de deux pages en fait 3 000 à 6 000.
pub const CV_TEXTE_MAX: usize = 30_000;
/// Au plus ce nombre de textes pour le style.
pub const STYLE_NOMBRE_MAX: usize = 20;
/// Le titre d'un texte pour le style.
pub const STYLE_TITRE_MAX: usize = 80;
/// Un texte pour le style : quelques phrases au moins, une longue lettre au plus.
pub const STYLE_TEXTE_MIN: usize = 80;
pub const STYLE_TEXTE_MAX: usize = 8_000;

/// Remet au propre un texte lu dans un fichier ou collé : fins de ligne « \n », espaces insécables et tabulations en
/// espaces, ligatures (« ﬁ ») en lettres, tirets longs en tirets simples, puces de police en « • », caractères
/// invisibles retirés, espaces en trop retirées au bout des lignes, jamais plus d'une ligne vide de suite.
pub fn nettoyer_texte(t: &str) -> String {
    let t = t.replace("\r\n", "\n").replace('\r', "\n");
    let mut propre = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '\n' => propre.push('\n'),
            '\t' | '\u{a0}' | '\u{202f}' | '\u{2007}' | '\u{2009}' | '\u{2002}' | '\u{2003}' | '\u{3000}' => {
                propre.push(' ')
            }
            '\u{fb00}' => propre.push_str("ff"),
            '\u{fb01}' => propre.push_str("fi"),
            '\u{fb02}' => propre.push_str("fl"),
            '\u{fb03}' => propre.push_str("ffi"),
            '\u{fb04}' => propre.push_str("ffl"),
            '\u{fb05}' | '\u{fb06}' => propre.push_str("st"),
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}' | '\u{2212}' => {
                propre.push('-')
            }
            // (les puces des polices de symboles, dans la zone privée d'Unicode)
            '\u{e000}'..='\u{f8ff}' => propre.push('•'),
            '\u{ad}' | '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}' | '\u{feff}' | '\u{fffd}' => {}
            c if c.is_control() => {}
            c => propre.push(c),
        }
    }
    let mut lignes: Vec<String> = Vec::new();
    for l in propre.split('\n') {
        // (les espaces en trop dans la ligne : une seule)
        let l = l.split(' ').filter(|m| !m.is_empty()).collect::<Vec<_>>().join(" ");
        if l.is_empty() && lignes.last().is_none_or(|x| x.is_empty()) {
            continue;
        }
        lignes.push(l);
    }
    while lignes.last().is_some_and(String::is_empty) {
        lignes.pop();
    }
    lignes.join("\n")
}

/// Un texte assez long pour être un vrai texte : au moins `n` lettres ou chiffres.
pub fn a_du_texte(t: &str, n: usize) -> bool {
    t.chars().filter(|c| c.is_alphanumeric()).take(n).count() >= n
}

/// Coupe un texte trop long à `max` caractères (au dernier retour à la ligne avant, s'il y en a un pas trop loin).
/// Rend aussi vrai s'il a été coupé.
pub fn couper(t: &str, max: usize) -> (String, bool) {
    if !trop_long(t, max) {
        return (t.to_string(), false);
    }
    let debut: String = t.chars().take(max).collect();
    let coupe = match debut.rfind('\n') {
        Some(i) if debut[..i].chars().count() > max * 9 / 10 => debut[..i].to_string(),
        _ => debut,
    };
    (coupe.trim_end().to_string(), true)
}

/// Le nom d'un CV tiré du nom de son fichier (« CV Jordan 2026.pdf » donne « CV Jordan 2026 »).
pub fn nom_depuis_fichier(fichier: &str) -> String {
    let base = fichier.rsplit(['\\', '/']).next().unwrap_or(fichier);
    let sans = match base.rfind('.') {
        Some(i) if i > 0 => &base[..i],
        _ => base,
    };
    let n = une_ligne(&sans.replace('_', " "));
    let n: String = n.chars().take(CV_NOM_MAX).collect();
    if n.is_empty() { "Mon CV".into() } else { n }
}

/// Vérifie le nom d'un CV.
pub fn verifier_nom_cv(nom: &str) -> Result<String, String> {
    let n = une_ligne(nom);
    if n.is_empty() {
        return Err("Donne un nom à ce CV, par exemple : CV 2026.".into());
    }
    if trop_long(&n, CV_NOM_MAX) {
        return Err(format!("Le nom du CV est trop long ({CV_NOM_MAX} caractères au plus)."));
    }
    Ok(n)
}

/// Vérifie le texte d'un CV collé ou corrigé (il ne peut pas être vide).
pub fn verifier_texte_cv(texte: &str) -> Result<String, String> {
    let t = nettoyer_texte(texte);
    if !a_du_texte(&t, 20) {
        return Err("Colle le texte de ton CV : il est vide ou presque.".into());
    }
    if trop_long(&t, CV_TEXTE_MAX) {
        return Err(format!(
            "Ce texte est trop long pour un CV ({} caractères au plus). Garde l'essentiel.",
            crate::recherche::milliers(CV_TEXTE_MAX as u32)
        ));
    }
    Ok(t)
}

/// Un texte pour le style, vérifié : son titre (rien : un titre tiré du début) et son texte.
pub fn verifier_style(titre: &str, texte: &str) -> Result<(String, String), String> {
    let t = nettoyer_texte(texte);
    if !a_du_texte(&t, 1) {
        return Err("Colle un texte que tu as écrit toi-même : une lettre, un mail, un message.".into());
    }
    if t.chars().count() < STYLE_TEXTE_MIN {
        return Err("Ce texte est très court : colle un texte de quelques phrases au moins.".into());
    }
    if trop_long(&t, STYLE_TEXTE_MAX) {
        return Err(format!(
            "Ce texte est trop long ({} caractères au plus). Colle-le en plusieurs morceaux.",
            crate::recherche::milliers(STYLE_TEXTE_MAX as u32)
        ));
    }
    let mut titre = une_ligne(titre);
    if trop_long(&titre, STYLE_TITRE_MAX) {
        return Err(format!("Le titre est trop long ({STYLE_TITRE_MAX} caractères au plus)."));
    }
    if titre.is_empty() {
        titre = debut(&t, 40);
    }
    Ok((titre, t))
}

/// Le début d'un texte, sur une ligne, coupé à un mot (« Bonjour, je vous écris pour... »).
pub fn debut(t: &str, max: usize) -> String {
    let l = une_ligne(t);
    if l.chars().count() <= max {
        return l;
    }
    let coupe: String = l.chars().take(max).collect();
    let coupe = match coupe.rfind(' ') {
        Some(i) if i > max / 2 => coupe[..i].to_string(),
        _ => coupe,
    };
    format!("{}...", coupe.trim_end_matches([',', ';', ':', '.', ' ']))
}

/// Le nombre de mots d'un texte.
pub fn mots(t: &str) -> usize {
    t.split_whitespace().filter(|m| m.chars().any(char::is_alphanumeric)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texte_remis_au_propre() {
        let brut = "  Jordan\u{a0}Exemple \r\n\r\n\r\n\u{f0b7}\tCariste \u{2013} CACES 3\n\n\nPro\u{fb01}l : o\u{fb03}ce\u{ad}\u{200b}\n  \n";
        assert_eq!(nettoyer_texte(brut), "Jordan Exemple\n\n• Cariste - CACES 3\n\nProfil : office");
        assert_eq!(nettoyer_texte("\n\n a \n"), "a");
        assert_eq!(nettoyer_texte(""), "");
        assert!(!nettoyer_texte("2019 \u{2014} 2021").contains(['\u{2013}', '\u{2014}']));
    }

    #[test]
    fn texte_coupe() {
        let (t, c) = couper("abc", 10);
        assert_eq!((t.as_str(), c), ("abc", false));
        let long = format!("{}\n{}", "a".repeat(95), "b".repeat(20));
        let (t, c) = couper(&long, 100);
        assert!(c);
        assert_eq!(t, "a".repeat(95));
        let (t, _) = couper(&"é".repeat(50), 10);
        assert_eq!(t.chars().count(), 10);
    }

    #[test]
    fn noms_et_textes_de_cv() {
        assert_eq!(nom_depuis_fichier(r"C:\Users\x\Documents\CV_Jordan 2026.pdf"), "CV Jordan 2026");
        assert_eq!(nom_depuis_fichier("cv.docx"), "cv");
        assert_eq!(nom_depuis_fichier(".pdf"), ".pdf");
        assert_eq!(nom_depuis_fichier(""), "Mon CV");
        assert_eq!(nom_depuis_fichier(&format!("{}.pdf", "n".repeat(200))).chars().count(), CV_NOM_MAX);
        assert_eq!(verifier_nom_cv("  CV   2026 ").unwrap(), "CV 2026");
        assert!(verifier_nom_cv(" ").is_err());
        assert!(verifier_nom_cv(&"x".repeat(81)).is_err());
        assert!(verifier_texte_cv("  \n ").unwrap_err().contains("vide"));
        assert!(verifier_texte_cv(&"mot ".repeat(10_000)).unwrap_err().contains("30 000"));
        assert_eq!(
            verifier_texte_cv("Jordan Exemple\r\nCariste, 2 ans d'expérience").unwrap(),
            "Jordan Exemple\nCariste, 2 ans d'expérience"
        );
    }

    #[test]
    fn textes_pour_le_style() {
        let texte = "Bonjour, je vous écris pour la place au magasin. J'ai fait un an de vente dans un magasin de sport à Metz.";
        let (titre, t) = verifier_style("", texte).unwrap();
        assert_eq!(titre, "Bonjour, je vous écris pour la place au...");
        assert_eq!(t, texte);
        assert_eq!(verifier_style(" Lettre  au magasin ", texte).unwrap().0, "Lettre au magasin");
        assert!(verifier_style("", "Bonjour").unwrap_err().contains("très court"));
        assert!(verifier_style("", "   ").unwrap_err().contains("toi-même"));
        assert!(verifier_style("", &"phrase ".repeat(2000)).unwrap_err().contains("8 000"));
        assert!(verifier_style(&"t".repeat(81), texte).is_err());
        assert_eq!(mots(texte), 22);
        assert_eq!(debut("court", 40), "court");
    }
}
