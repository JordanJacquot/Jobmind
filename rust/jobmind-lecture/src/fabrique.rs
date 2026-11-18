//! Les tests seulement : de petits fichiers d'exemple fabriqués ici, octet par octet (un zip, un .docx, un PDF avec du
//! texte, un PDF scanné), jamais pris sur le PC.

/// Un zip : (nom, contenu, compressé ou rangé tel quel).
pub fn zip(parties: &[(&str, &[u8], bool)]) -> Vec<u8> {
    let mut z = Vec::new();
    let mut repertoire = Vec::new();
    for (nom, contenu, compresse) in parties {
        let donnees = if *compresse { miniz_oxide::deflate::compress_to_vec(contenu, 6) } else { contenu.to_vec() };
        let crc = crc32fast::hash(contenu);
        let decalage = z.len() as u32;
        let methode: u16 = if *compresse { 8 } else { 0 };
        let entete = |sig: u32, central: bool| {
            let mut e = Vec::new();
            e.extend(sig.to_le_bytes());
            if central {
                e.extend(20u16.to_le_bytes());
            }
            e.extend(20u16.to_le_bytes());
            e.extend(0u16.to_le_bytes());
            e.extend(methode.to_le_bytes());
            e.extend(0u32.to_le_bytes());
            e.extend(crc.to_le_bytes());
            e.extend((donnees.len() as u32).to_le_bytes());
            e.extend((contenu.len() as u32).to_le_bytes());
            e.extend((nom.len() as u16).to_le_bytes());
            e.extend(0u16.to_le_bytes());
            if central {
                // (commentaire, disque, attributs internes, puis attributs externes)
                e.extend([0u8; 6]);
                e.extend(0u32.to_le_bytes());
                e.extend(decalage.to_le_bytes());
            }
            e.extend(nom.as_bytes());
            e
        };
        z.extend(entete(0x0403_4b50, false));
        z.extend(&donnees);
        repertoire.extend(entete(0x0201_4b50, true));
    }
    let debut = z.len() as u32;
    z.extend(&repertoire);
    z.extend(0x0605_4b50u32.to_le_bytes());
    z.extend([0u8; 4]);
    z.extend((parties.len() as u16).to_le_bytes());
    z.extend((parties.len() as u16).to_le_bytes());
    z.extend((repertoire.len() as u32).to_le_bytes());
    z.extend(debut.to_le_bytes());
    z.extend(0u16.to_le_bytes());
    z
}

const TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

/// Un .docx avec ces parties en plus de ses fichiers de base.
pub fn docx_parties(parties: &[(&str, &str)]) -> Vec<u8> {
    let mut l: Vec<(&str, &[u8], bool)> =
        vec![("[Content_Types].xml", TYPES.as_bytes(), true), ("_rels/.rels", RELS.as_bytes(), true)];
    for (n, x) in parties {
        l.push((n, x.as_bytes(), true));
    }
    zip(&l)
}

/// Un .docx d'un paragraphe par ligne.
pub fn docx_simple(lignes: &[&str]) -> Vec<u8> {
    let corps: String = lignes
        .iter()
        .map(|l| {
            format!(
                "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                l.replace('&', "&amp;").replace('<', "&lt;")
            )
        })
        .collect();
    let doc = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{corps}<w:sectPr/></w:body></w:document>"#
    );
    docx_parties(&[("word/document.xml", &doc)])
}

/// Un PDF : ces objets (le numéro de chacun est sa place, à partir de 1), avec sa table des objets juste.
fn pdf(objets: &[Vec<u8>]) -> Vec<u8> {
    let mut p = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut places = Vec::new();
    for (i, o) in objets.iter().enumerate() {
        places.push(p.len());
        p.extend(format!("{} 0 obj\n", i + 1).as_bytes());
        p.extend(o);
        p.extend(b"\nendobj\n");
    }
    let xref = p.len();
    p.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objets.len() + 1).as_bytes());
    for x in places {
        p.extend(format!("{x:010} 00000 n \n").as_bytes());
    }
    p.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objets.len() + 1).as_bytes());
    p
}

/// Un flux compressé (FlateDecode).
fn flux(contenu: &[u8], dico: &str) -> Vec<u8> {
    let c = miniz_oxide::deflate::compress_to_vec_zlib(contenu, 6);
    let mut o = format!("<< /Length {} /Filter /FlateDecode {dico} >>\nstream\n", c.len()).into_bytes();
    o.extend(c);
    o.extend(b"\nendstream");
    o
}

/// Une chaîne PDF en WinAnsi (é : \351), parenthèses et barres échappées.
fn chaine(t: &str) -> String {
    let mut s = String::from("(");
    for c in t.chars() {
        match c {
            '(' | ')' | '\\' => {
                s.push('\\');
                s.push(c);
            }
            c if c.is_ascii() => s.push(c),
            // (Latin-1, le même que WinAnsi pour les lettres accentuées)
            c if (c as u32) < 256 => s.push_str(&format!("\\{:03o}", c as u32)),
            _ => s.push('?'),
        }
    }
    s.push(')');
    s
}

/// Un PDF d'une page, une ligne de texte (Helvetica) par ligne donnée.
pub fn pdf_texte(lignes: &[&str]) -> Vec<u8> {
    let mut contenu = String::from("BT /F1 12 Tf 16 TL 72 770 Td\n");
    for l in lignes {
        contenu.push_str(&format!("{} Tj T*\n", chaine(l)));
    }
    contenu.push_str("ET\n");
    pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
            .to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
        flux(contenu.as_bytes(), ""),
    ])
}

/// Un PDF « scanné » : une page avec seulement une image (gris, 4 x 4), aucun texte.
pub fn pdf_avec_image() -> Vec<u8> {
    pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /XObject << /Im1 4 0 R >> >> /Contents 5 0 R >>"
            .to_vec(),
        flux(&[0x80; 16], "/Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceGray /BitsPerComponent 8"),
        flux(b"q 500 0 0 700 40 60 cm /Im1 Do Q\n", ""),
    ])
}

/// Un PDF dont la police donne une largeur nulle à « é » (comme les PDF imprimés par un navigateur) : chaque morceau
/// (x en points, texte) est posé à sa vraie place, sur la même ligne. Toutes les autres lettres font 0,6 de la taille.
pub fn pdf_morceaux(morceaux: &[(f64, &str)]) -> Vec<u8> {
    let mut contenu = String::from("BT /F1 12 Tf\n");
    for (x, t) in morceaux {
        contenu.push_str(&format!("1 0 0 1 {x:.2} 700 Tm {} Tj\n", chaine(t)));
    }
    contenu.push_str("ET\n");
    let largeurs: Vec<String> =
        (32..=255).map(|c| if c == 233 { "0".to_string() } else { "600".to_string() }).collect();
    pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
            .to_vec(),
        format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding /FirstChar 32 /LastChar 255 /Widths [{}] >>",
            largeurs.join(" ")
        )
        .into_bytes(),
        flux(contenu.as_bytes(), ""),
    ])
}
