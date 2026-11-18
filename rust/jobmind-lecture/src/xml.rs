//! Le texte d'une partie XML de Word (document, en-tête, pied de page), lu balise par balise, sans bibliothèque :
//!
//! - le texte des balises `t` (`<w:t>`), seulement elles : ni les codes des champs (`instrText`), ni le texte barré
//!   des modifications suivies (`delText`) ;
//! - `tab` dans un morceau de texte (`<w:r>`) donne une espace (pas les taquets d'un paragraphe, `<w:tabs>`), `br` et
//!   `cr` un retour à la ligne, la fin d'un paragraphe (`</w:p>`) un retour à la ligne, la fin d'une ligne de tableau
//!   (`</w:tr>`) une ligne vide ;
//! - une zone de texte écrite deux fois (`mc:AlternateContent` : la nouvelle forme, puis la forme de secours
//!   `mc:Fallback`) n'est lue qu'une fois ;
//! - les entités (`&amp;`, `&#233;`, `&#xE9;`) sont remplacées par leur caractère.
//!
//! Les préfixes ne comptent pas (`w:`, `a:`, ou ceux d'un document « strict ») : seulement le nom après les deux-points.

/// Le texte d'une partie XML de Word.
pub fn texte_word(xml: &str) -> String {
    let mut sortie = String::new();
    let o = xml.as_bytes();
    let mut i = 0;
    // (dans <w:t> : le texte compte ; dans un morceau <w:r> ; dans une forme de secours, rien ne compte)
    let mut dans_t = false;
    let mut dans_r = 0usize;
    let mut secours = 0usize;
    while i < o.len() {
        if o[i] != b'<' {
            let fin = xml[i..].find('<').map_or(o.len(), |n| i + n);
            if dans_t && secours == 0 {
                entites(&xml[i..fin], &mut sortie);
            }
            i = fin;
            continue;
        }
        // Un commentaire, une instruction, une section CDATA, une déclaration
        if xml[i..].starts_with("<!--") {
            i = xml[i..].find("-->").map_or(o.len(), |n| i + n + 3);
            continue;
        }
        if xml[i..].starts_with("<![CDATA[") {
            let fin = xml[i..].find("]]>").map_or(o.len(), |n| i + n);
            if dans_t && secours == 0 {
                sortie.push_str(&xml[i + 9..fin]);
            }
            i = (fin + 3).min(o.len());
            continue;
        }
        if xml[i..].starts_with("<?") || xml[i..].starts_with("<!") {
            i = xml[i..].find('>').map_or(o.len(), |n| i + n + 1);
            continue;
        }
        // Une balise : jusqu'au « > » qui n'est pas dans des guillemets
        let mut j = i + 1;
        let mut guillemet = 0u8;
        while j < o.len() {
            match o[j] {
                b'"' | b'\'' if guillemet == 0 => guillemet = o[j],
                c if c == guillemet => guillemet = 0,
                b'>' if guillemet == 0 => break,
                _ => {}
            }
            j += 1;
        }
        let balise = &xml[i + 1..j.min(o.len())];
        i = j + 1;
        let fermante = balise.starts_with('/');
        let vide = balise.ends_with('/');
        let nom_complet =
            balise.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
        let nom = nom_complet.rsplit(':').next().unwrap_or("");
        if nom == "Fallback" {
            if fermante {
                secours = secours.saturating_sub(1);
            } else if !vide {
                secours += 1;
            }
            continue;
        }
        if secours > 0 {
            continue;
        }
        match (nom, fermante) {
            ("t", false) => dans_t = !vide,
            ("t", true) => dans_t = false,
            ("r", false) if !vide => dans_r += 1,
            ("r", true) => dans_r = dans_r.saturating_sub(1),
            ("tab", false) if dans_r > 0 => sortie.push(' '),
            ("br" | "cr", false) => sortie.push('\n'),
            ("noBreakHyphen", false) => sortie.push('-'),
            ("p", true) => sortie.push('\n'),
            ("tr", true) => sortie.push('\n'),
            _ => {}
        }
    }
    sortie
}

/// Ajoute un texte XML, ses entités remplacées (une entité inconnue reste telle quelle).
fn entites(t: &str, sortie: &mut String) {
    let mut reste = t;
    while let Some(n) = reste.find('&') {
        sortie.push_str(&reste[..n]);
        reste = &reste[n..];
        // (le « ; » dans les 12 premiers octets : une entité est courte)
        let Some(fin) = reste.bytes().take(12).position(|b| b == b';') else {
            sortie.push('&');
            reste = &reste[1..];
            continue;
        };
        let nom = &reste[1..fin];
        let c = match nom {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => nom
                .strip_prefix("#x")
                .or_else(|| nom.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| nom.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match c {
            Some(c) => {
                sortie.push(c);
                reste = &reste[fin + 1..];
            }
            None => {
                sortie.push('&');
                reste = &reste[1..];
            }
        }
    }
    sortie.push_str(reste);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphes_tableaux_et_entites() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="x"><w:body>
<w:p><w:pPr><w:tabs><w:tab w:val="left" w:pos="2880"/></w:tabs></w:pPr><w:r><w:t>Jordan</w:t></w:r><w:r><w:tab/><w:t xml:space="preserve">Exemple </w:t></w:r></w:p>
<w:p><w:r><w:t>Caf&#233; &amp; th&#xE9;, &lt;ok&gt;</w:t><w:br/><w:t>ligne 2</w:t></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>2023</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Cariste</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> HYPERLINK "x" </w:instrText></w:r><w:r><w:t>lien</w:t></w:r></w:p>
<w:p><w:del><w:r><w:delText>barré</w:delText></w:r></w:del><w:r><w:t>gardé</w:t></w:r><!-- <w:t>commentaire</w:t> --></w:p>
<w:p><w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:txbxContent><w:p><w:r><w:t>Zone</w:t></w:r></w:p></w:txbxContent></mc:Choice><mc:Fallback><w:pict><w:txbxContent><w:p><w:r><w:t>Zone</w:t></w:r></w:p></w:txbxContent></w:pict></mc:Fallback></mc:AlternateContent></w:r></w:p>
<w:p><w:r><w:t>a &unknown; b &amp</w:t></w:r></w:p>
</w:body></w:document>"#;
        assert_eq!(
            texte_word(xml),
            "Jordan Exemple \nCafé & thé, <ok>\nligne 2\n2023\nCariste\n\nlien\ngardé\nZone\n\na &unknown; b &amp\n"
        );
    }

    #[test]
    fn xml_abime_sans_arret() {
        for x in ["<w:t>coupé", "<w:p><w:t a=\">", "&#xFFFFFFFF;", "<", "<<<>>>", "<w:t>&#55296;</w:t>"] {
            let _ = texte_word(x);
        }
        assert_eq!(texte_word("<w:t>&#55296;x</w:t>"), "&#55296;x");
    }
}
