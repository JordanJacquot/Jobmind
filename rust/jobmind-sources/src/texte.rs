//! Le texte d'une offre remis au propre : les sources mettent du HTML dans leurs extraits (Jooble : `<b>cariste</b>`,
//! `&nbsp;`), des retours à la ligne en trop, parfois des caractères de contrôle. Ici : les balises retirées (un
//! paragraphe ou un saut de ligne gardés en retour à la ligne), les entités lues, les espaces remis d'aplomb.

/// La longueur la plus grande d'un texte d'offre gardé (en caractères).
pub const DESCRIPTION_MAX: usize = 20_000;
/// La longueur la plus grande d'un titre, d'une entreprise ou d'un lieu.
pub const LIGNE_MAX: usize = 200;

/// Une entité HTML lue (« amp », « #233 », « #xE9 »).
fn entite(nom: &str) -> Option<char> {
    if let Some(n) = nom.strip_prefix("#x").or_else(|| nom.strip_prefix("#X")) {
        return u32::from_str_radix(n, 16).ok().and_then(char::from_u32);
    }
    if let Some(n) = nom.strip_prefix('#') {
        return n.parse::<u32>().ok().and_then(char::from_u32);
    }
    Some(match nom {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        "rsquo" | "lsquo" => '\'',
        "ldquo" | "rdquo" => '"',
        "oelig" => 'œ',
        "euml" => 'ë',
        "uuml" => 'ü',
        "Egrave" => 'È',
        "Agrave" => 'À',
        "Ccedil" => 'Ç',
        "laquo" => '«',
        "raquo" => '»',
        "hellip" => '…',
        "euro" => '€',
        "eacute" => 'é',
        "egrave" => 'è',
        "ecirc" => 'ê',
        "agrave" => 'à',
        "acirc" => 'â',
        "ccedil" => 'ç',
        "ocirc" => 'ô',
        "ucirc" => 'û',
        "ugrave" => 'ù',
        "icirc" => 'î',
        "iuml" => 'ï',
        "Eacute" => 'É',
        "deg" => '°',
        "bull" | "middot" => '·',
        "ndash" | "mdash" => '-',
        _ => return None,
    })
}

/// Les balises qui finissent une ligne.
fn coupe_ligne(balise: &str) -> bool {
    let b = balise.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
    matches!(
        b.to_ascii_lowercase().as_str(),
        "br" | "p" | "div" | "li" | "ul" | "ol" | "tr" | "h1" | "h2" | "h3" | "h4"
    )
}

/// Le texte d'un extrait HTML, propre : balises retirées, entités lues, tirets longs remplacés (règle de la suite),
/// pas plus d'une ligne vide de suite, espaces simples, coupé à `max` caractères.
pub fn propre(html: &str, max: usize) -> String {
    let html = sans_scripts(html);
    let html = html.as_str();
    let mut t = String::with_capacity(html.len());
    let mut it = html.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '<' => {
                // (une balise : jusqu'à « > » ; un « < » seul reste un caractère)
                let mut b = String::new();
                let mut fermee = false;
                for x in it.by_ref() {
                    if x == '>' {
                        fermee = true;
                        break;
                    }
                    if b.len() > 200 {
                        break;
                    }
                    b.push(x);
                }
                if !fermee || b.is_empty() || b.starts_with(' ') {
                    t.push('<');
                    t.push_str(&b);
                } else if coupe_ligne(&b) {
                    t.push('\n');
                }
            }
            '&' => {
                let mut nom = String::new();
                while let Some(&x) = it.peek() {
                    if x == ';' || nom.len() > 10 || !(x.is_ascii_alphanumeric() || x == '#') {
                        break;
                    }
                    nom.push(x);
                    it.next();
                }
                match (it.peek() == Some(&';'), entite(&nom)) {
                    (true, Some(e)) => {
                        it.next();
                        t.push(e);
                    }
                    _ => {
                        t.push('&');
                        t.push_str(&nom);
                    }
                }
            }
            _ => t.push(c),
        }
    }
    lignes_propres(&t, max)
}

/// Un extrait HTML sans le contenu de ses `<script>` et `<style>` (du code, jamais du texte à lire).
fn sans_scripts(html: &str) -> String {
    let bas = html.to_ascii_lowercase();
    let mut t = String::with_capacity(html.len());
    let mut i = 0;
    while let Some(p) = ["<script", "<style"].iter().filter_map(|b| bas[i..].find(b).map(|p| (p + i, *b))).min() {
        let (debut, balise) = p;
        // (« <scripts » ou « <styled » ne sont pas ces balises)
        let apres = bas.as_bytes().get(debut + balise.len()).copied().unwrap_or(b'>');
        if !(apres == b'>' || apres == b'/' || apres.is_ascii_whitespace()) {
            t.push_str(&html[i..debut + balise.len()]);
            i = debut + balise.len();
            continue;
        }
        t.push_str(&html[i..debut]);
        let fermante = format!("</{}", &balise[1..]);
        i = match bas[debut..].find(&fermante) {
            Some(f) => bas[debut + f..].find('>').map_or(bas.len(), |g| debut + f + g + 1),
            None => bas.len(),
        };
    }
    t.push_str(&html[i..]);
    t
}

/// Les entités HTML d'un texte lues (« &amp; », « &#233; »), sans rien d'autre : une entité inconnue reste telle quelle.
pub fn entites(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut t = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '&' {
            t.push(c);
            continue;
        }
        let mut nom = String::new();
        while let Some(&x) = it.peek() {
            if x == ';' || nom.len() > 10 || !(x.is_ascii_alphanumeric() || x == '#') {
                break;
            }
            nom.push(x);
            it.next();
        }
        match (it.peek() == Some(&';'), entite(&nom)) {
            (true, Some(e)) => {
                it.next();
                t.push(e);
            }
            _ => {
                t.push('&');
                t.push_str(&nom);
            }
        }
    }
    t
}

/// Un texte déjà sans balises remis d'aplomb : tirets longs remplacés (règle de la suite), espaces simples, pas plus
/// d'une ligne vide de suite, aucun caractère de contrôle, coupé à `max` caractères.
pub fn lignes_propres(t: &str, max: usize) -> String {
    let mut lignes: Vec<String> = Vec::new();
    for l in t.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        let l: String = l
            .chars()
            .map(|c| match c {
                '\u{2013}' | '\u{2014}' | '\u{2212}' => '-',
                '\u{a0}' | '\u{202f}' | '\t' => ' ',
                c => c,
            })
            .filter(|c| !c.is_control())
            .collect();
        let l = l.split_whitespace().collect::<Vec<_>>().join(" ");
        if l.is_empty() && lignes.last().is_none_or(|x| x.is_empty()) {
            continue;
        }
        lignes.push(l);
    }
    while lignes.last().is_some_and(|l| l.is_empty()) {
        lignes.pop();
    }
    let t = lignes.join("\n");
    if t.chars().count() > max { t.chars().take(max).collect::<String>().trim_end().to_string() } else { t }
}

/// Une ligne propre (un titre, une entreprise, un lieu) : sans retour à la ligne, coupée à [`LIGNE_MAX`].
pub fn ligne(html: &str) -> String {
    propre(html, LIGNE_MAX * 2)
        .split('\n')
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(LIGNE_MAX)
        .collect()
}

/// Une adresse d'offre acceptée : http ou https, sans espace ni caractère de contrôle, pas trop longue. Sinon : vide.
pub fn adresse(url: &str) -> String {
    let u = url.trim();
    let bonne = (u.starts_with("https://") || u.starts_with("http://"))
        && u.len() <= 2000
        && !u.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"' | '\\'))
        && u.split("://").nth(1).is_some_and(|reste| !reste.is_empty() && !reste.starts_with('/'));
    if bonne { u.to_string() } else { String::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraits_html() {
        assert_eq!(
            propre("<b>Cariste</b>&nbsp;H/F : chargement &amp; d&eacute;chargement<br/>CACES&nbsp;3 exig&#233;", 500),
            "Cariste H/F : chargement & déchargement\nCACES 3 exigé"
        );
        assert_eq!(propre("<p>Un</p><p></p><p></p><p>Deux</p>", 500), "Un\n\nDeux");
        assert_eq!(propre("Salaire &lt; 2000 &euro; &#x2014; à voir", 500), "Salaire < 2000 € - à voir");
        // Ce qui ressemble à du HTML sans en être reste
        assert_eq!(propre("3 < 5 et R&D & co", 500), "3 < 5 et R&D & co");
        assert_eq!(propre("a &inconnu; b", 500), "a &inconnu; b");
        assert_eq!(propre("  \u{0}trop   d'espaces\t\r\n\r\n\r\n fin  ", 500), "trop d'espaces\n\nfin");
        assert_eq!(propre("Préparateur – équipe — nuit", 500), "Préparateur - équipe - nuit");
        assert_eq!(propre(&"é".repeat(30), 10).chars().count(), 10);
        assert_eq!(ligne("Agent\nde <i>quai</i>"), "Agent de quai");
        assert_eq!(ligne(&"x".repeat(500)).len(), LIGNE_MAX);
    }

    #[test]
    fn adresses() {
        assert_eq!(adresse(" https://www.adzuna.fr/land/ad/123?se=x "), "https://www.adzuna.fr/land/ad/123?se=x");
        assert_eq!(adresse("http://jooble.org/desc/42"), "http://jooble.org/desc/42");
        for faux in [
            "javascript:alert(1)",
            "file:///C:/x",
            "https://",
            "https:///x",
            "https://a b",
            "ftp://x",
            "",
            "https://x\"y",
        ] {
            assert_eq!(adresse(faux), "", "{faux}");
        }
    }
}
