//! L'export des candidatures (étape 12 du plan) : la liste en CSV pour Excel ([`csv`]) et en PDF lisible ([`pdf`]),
//! avec le nom proposé dans la boîte « Enregistrer sous » ([`nom_fichier`]).
//!
//! Le CSV garde ce qui se range dans un tableau (jamais le texte envoyé ni le CV : rien de long, rien de secret), chaque
//! champ protégé contre les formules d'Excel. Le PDF est écrit à la main, sans bibliothèque et sans compression : une
//! page A4 en portrait, les polices standard Helvetica et Helvetica-Bold en WinAnsi (les accents français, « œ », « € »,
//! les guillemets), un tableau dont chaque texte trop long est coupé avec « ... » d'après les vraies largeurs des
//! lettres d'Helvetica, pour ne jamais déborder sur la colonne voisine.

use crate::candidature::Candidature;
use crate::jour::Jour;

/// L'en-tête du CSV, dans l'ordre des colonnes.
pub const ENTETE_CSV: [&str; 12] = [
    "Entreprise",
    "Poste",
    "Ville",
    "Statut",
    "Envoyée le",
    "Par où",
    "Offre vue sur",
    "Lien de l'offre",
    "Relancée le",
    "Entretien",
    "Dernier changement",
    "Notes",
];

/// Le nom proposé dans la boîte « Enregistrer sous » : « Mes candidatures 2026-11-19.csv » (ou .pdf).
pub fn nom_fichier(extension: &str, aujourdhui: Jour) -> String {
    format!("Mes candidatures {}.{}", aujourdhui.iso(), extension.trim_start_matches('.'))
}

/// Les candidatures des plus récentes aux plus anciennes (jour d'envoi, puis numéro).
fn triees(liste: &[Candidature]) -> Vec<&Candidature> {
    let mut l: Vec<&Candidature> = liste.iter().collect();
    l.sort_by(|a, b| b.envoyee_le.cmp(&a.envoyee_le).then(b.id.cmp(&a.id)));
    l
}

/// « 20/11/2026 à 9 h 30, Zone Actipôle » (sans heure ni lieu s'ils manquent), ou rien.
fn entretien(c: &Candidature) -> String {
    let Some(e) = &c.entretien else { return String::new() };
    let mut t = e.jour.saisie();
    if let Some(h) = e.heure {
        t.push_str(" à ");
        t.push_str(&h.texte());
    }
    if !e.lieu.is_empty() {
        t.push_str(", ");
        t.push_str(&e.lieu);
    }
    t
}

/// Un champ du CSV : une apostrophe devant ce qu'Excel prendrait pour une formule (règle de l'OWASP), puis entre
/// guillemets s'il contient un point-virgule, un guillemet, un retour à la ligne ou une espace au bord.
fn champ_csv(t: &str) -> String {
    let mut t = t.to_string();
    if t.starts_with(['\t', '\r']) || t.trim_start_matches(' ').starts_with(['=', '+', '-', '@']) {
        t.insert(0, '\'');
    }
    if t.contains([';', '"', '\r', '\n']) || t.starts_with(' ') || t.ends_with(' ') {
        format!("\"{}\"", t.replace('"', "\"\""))
    } else {
        t
    }
}

/// Le CSV des candidatures pour Excel : UTF-8 avec BOM, séparateur point-virgule, fin de ligne \r\n.
pub fn csv(liste: &[Candidature], _aujourdhui: Jour) -> Vec<u8> {
    let mut t = String::from("\u{FEFF}");
    t.push_str(&ENTETE_CSV.join(";"));
    t.push_str("\r\n");
    for c in triees(liste) {
        let champs = [
            c.entreprise.clone(),
            c.poste.clone(),
            c.ville.clone(),
            c.statut.nom().to_string(),
            c.envoyee_le.saisie(),
            c.moyen.nom().to_string(),
            c.source.clone(),
            c.lien.clone(),
            c.relance_le.map(Jour::saisie).unwrap_or_default(),
            entretien(c),
            c.statut_le.saisie(),
            c.notes.clone(),
        ];
        t.push_str(&champs.iter().map(|x| champ_csv(x)).collect::<Vec<_>>().join(";"));
        t.push_str("\r\n");
    }
    t.into_bytes()
}

// Le PDF

/// La page A4 en portrait (en points).
pub const PAGE_LARGEUR: f64 = 595.0;
pub const PAGE_HAUTEUR: f64 = 842.0;
/// La marge à gauche et à droite.
pub const MARGE: f64 = 36.0;
/// Les colonnes du tableau : titre et largeur (en tout 523 points, la page moins ses marges).
pub const COLONNES: [(&str, f64); 6] = [
    ("Envoyée le", 54.0),
    ("Entreprise", 118.0),
    ("Poste", 132.0),
    ("Ville", 75.0),
    ("Statut", 80.0),
    ("Offre vue sur", 64.0),
];
/// L'écart entre le texte et le bord de sa case, de chaque côté.
pub const RETRAIT: f64 = 3.0;
/// La taille des lettres du tableau.
pub const TAILLE: f64 = 9.0;
/// Le haut du tableau, la hauteur de sa ligne de titres, celle d'une ligne, et le plus bas où une ligne peut aller.
const HAUT_TABLEAU: f64 = 752.0;
const HAUTEUR_TITRES: f64 = 18.0;
const HAUTEUR_LIGNE: f64 = 15.0;
const BAS_TABLEAU: f64 = 56.0;
/// Le nombre de lignes d'une page (45).
pub const LIGNES_PAR_PAGE: usize = ((HAUT_TABLEAU - HAUTEUR_TITRES - BAS_TABLEAU) / HAUTEUR_LIGNE) as usize;

/// Les caractères 0x80 à 0x9F de WinAnsi (les autres sont ceux de l'ASCII et du Latin-1).
const WINANSI_80: [(u8, char); 27] = [
    (0x80, '€'),
    (0x82, '‚'),
    (0x83, 'ƒ'),
    (0x84, '„'),
    (0x85, '…'),
    (0x86, '†'),
    (0x87, '‡'),
    (0x88, 'ˆ'),
    (0x89, '‰'),
    (0x8A, 'Š'),
    (0x8B, '‹'),
    (0x8C, 'Œ'),
    (0x8E, 'Ž'),
    (0x91, '‘'),
    (0x92, '’'),
    (0x93, '“'),
    (0x94, '”'),
    (0x95, '•'),
    (0x96, '\u{2013}'),
    (0x97, '\u{2014}'),
    (0x98, '˜'),
    (0x99, '™'),
    (0x9A, 'š'),
    (0x9B, '›'),
    (0x9C, 'œ'),
    (0x9E, 'ž'),
    (0x9F, 'Ÿ'),
];

/// L'octet WinAnsi d'un caractère (aucun : hors de la table).
pub fn winansi(c: char) -> Option<u8> {
    match c {
        ' '..='~' | '\u{A0}'..='\u{FF}' => Some(c as u8),
        _ => WINANSI_80.iter().find(|(_, x)| *x == c).map(|(o, _)| *o),
    }
}

/// Le caractère d'un octet WinAnsi (pour relire un PDF).
pub fn depuis_winansi(o: u8) -> Option<char> {
    match o {
        0x20..=0x7E | 0xA0..=0xFF => Some(o as char),
        _ => WINANSI_80.iter().find(|(x, _)| *x == o).map(|(_, c)| *c),
    }
}

/// Le caractère dessiné à la place de `c` : les espaces fines et les caractères de contrôle deviennent une espace, un
/// caractère hors de WinAnsi devient « ? ».
fn affiche(c: char) -> char {
    if c.is_control() || matches!(c, '\u{2002}'..='\u{200A}' | '\u{202F}' | '\u{205F}') {
        ' '
    } else if winansi(c).is_some() {
        c
    } else {
        '?'
    }
}

/// Les largeurs d'Helvetica de l'espace (0x20) au tilde (0x7E), en millièmes de la taille des lettres (fichier AFM
/// d'Adobe).
#[rustfmt::skip]
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, // espace à /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // 0 à 9
    278, 278, 584, 584, 584, 556, 1015, // : à @
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, // A à M
    722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, // N à Z
    278, 278, 278, 469, 556, 333, // [ à `
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, // a à m
    556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, // n à z
    334, 260, 334, 584, // { à ~
];

/// Les mêmes pour Helvetica-Bold.
#[rustfmt::skip]
const HELVETICA_GRAS: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, // espace à /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // 0 à 9
    333, 333, 584, 584, 584, 611, 975, // : à @
    722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, // A à M
    722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, // N à Z
    333, 278, 333, 584, 556, 333, // [ à `
    556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, // a à m
    611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, // n à z
    389, 280, 389, 584, // { à ~
];

/// La largeur d'un caractère dessiné (en millièmes de la taille des lettres) ; 556 pour un caractère inconnu.
fn largeur_car(c: char, gras: bool) -> u16 {
    let ascii = |b: char| if gras { HELVETICA_GRAS[b as usize - 32] } else { HELVETICA[b as usize - 32] };
    let (normal, epais) = match c {
        ' '..='~' => return ascii(c),
        // (les lettres accentuées ont la largeur de leur lettre de base, sauf les « i » : 278)
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => return ascii('a'),
        'ç' => return ascii('c'),
        'è' | 'é' | 'ê' | 'ë' => return ascii('e'),
        'ñ' => return ascii('n'),
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => return ascii('o'),
        'ù' | 'ú' | 'û' | 'ü' => return ascii('u'),
        'ý' | 'ÿ' => return ascii('y'),
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => return ascii('A'),
        'Ç' => return ascii('C'),
        'È' | 'É' | 'Ê' | 'Ë' => return ascii('E'),
        'Ì' | 'Í' | 'Î' | 'Ï' => return ascii('I'),
        'Ñ' => return ascii('N'),
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => return ascii('O'),
        'Ù' | 'Ú' | 'Û' | 'Ü' => return ascii('U'),
        'Ý' | 'Ÿ' => return ascii('Y'),
        'Š' => return ascii('S'),
        'š' => return ascii('s'),
        'Ž' => return ascii('Z'),
        'ž' => return ascii('z'),
        'ì' | 'í' | 'î' | 'ï' | '\u{A0}' | '·' => (278, 278),
        'œ' => (944, 944),
        'Œ' | 'Æ' | '…' | '‰' | '™' | '\u{2014}' => (1000, 1000),
        'æ' => (889, 889),
        'ß' | 'ø' => (611, 611),
        'Ø' => (778, 778),
        '«' | '»' | '€' | '£' | '§' | '\u{2013}' => (556, 556),
        '‹' | '›' => (333, 333),
        '‘' | '’' | '‚' => (222, 278),
        '“' | '”' | '„' => (333, 500),
        '•' => (350, 350),
        '°' => (400, 400),
        '©' | '®' => (737, 737),
        '²' | '³' | '¹' | '´' | '¨' | '\u{AD}' => (333, 333),
        '±' | '×' | '÷' | '¬' => (584, 584),
        _ => (556, 556),
    };
    if gras { epais } else { normal }
}

/// La largeur d'un texte (en points) en Helvetica (ou en Helvetica-Bold), à cette taille.
pub fn largeur(t: &str, gras: bool, taille: f64) -> f64 {
    t.chars().map(|c| f64::from(largeur_car(affiche(c), gras))).sum::<f64>() * taille / 1000.0
}

/// Le texte coupé avec « ... » pour tenir dans `max` points (tel quel s'il tient déjà).
pub fn couper(t: &str, max: f64, gras: bool, taille: f64) -> String {
    if largeur(t, gras, taille) <= max {
        return t.to_string();
    }
    let reste = max - largeur("...", gras, taille);
    let mut garde = String::new();
    let mut l = 0.0;
    for c in t.chars() {
        let lc = f64::from(largeur_car(affiche(c), gras)) * taille / 1000.0;
        if l + lc > reste {
            break;
        }
        l += lc;
        garde.push(c);
    }
    format!("{}...", garde.trim_end())
}

/// Une chaîne PDF en WinAnsi : parenthèses et barres obliques échappées, octets hors de l'ASCII en octal.
fn chaine(t: &str) -> String {
    let mut s = String::from("(");
    for c in t.chars() {
        let o = winansi(affiche(c)).unwrap_or(b'?');
        match o {
            b'(' | b')' | b'\\' => {
                s.push('\\');
                s.push(o as char);
            }
            0x20..=0x7E => s.push(o as char),
            _ => s.push_str(&format!("\\{o:03o}")),
        }
    }
    s.push(')');
    s
}

/// Un texte posé sur la page (F1 : Helvetica, F2 : Helvetica-Bold), sur une ligne à lui.
fn texte(flux: &mut String, gras: bool, taille: f64, x: f64, y: f64, t: &str) {
    let police = if gras { "F2" } else { "F1" };
    flux.push_str(&format!("BT /{police} {taille} Tf {x:.2} {y:.2} Td {} Tj ET\n", chaine(t)));
}

/// Un rectangle plein (couleur RVB de 0 à 1).
fn rectangle(flux: &mut String, (r, v, b): (f64, f64, f64), x: f64, y: f64, l: f64, h: f64) {
    flux.push_str(&format!("{r} {v} {b} rg {x:.2} {y:.2} {l:.2} {h:.2} re f\n"));
}

/// Les cases d'une ligne du tableau.
fn cases(c: &Candidature) -> [String; 6] {
    [
        c.envoyee_le.saisie(),
        c.entreprise.clone(),
        c.poste.clone(),
        c.ville.clone(),
        c.statut.nom().to_string(),
        c.source.clone(),
    ]
}

/// Une ligne de cases sous la hauteur `haut` (son bord du haut), chacune coupée pour tenir dans sa colonne.
fn ligne(flux: &mut String, cases: &[String], gras: bool, haut: f64, hauteur: f64) {
    let y = haut - hauteur / 2.0 - TAILLE * 0.35;
    let mut x = MARGE;
    for ((_, l), t) in COLONNES.iter().zip(cases) {
        let t = couper(t, l - 2.0 * RETRAIT, gras, TAILLE);
        if !t.is_empty() {
            texte(flux, gras, TAILLE, x + RETRAIT, y, &t);
        }
        x += l;
    }
}

/// Le contenu d'une page : le titre, la phrase de l'export, le tableau (ou la phrase de la liste vide), le numéro.
fn page(lignes: &[&Candidature], numero: usize, total: usize, sous_titre: &str) -> String {
    let mut f = String::new();
    texte(&mut f, true, 16.0, MARGE, 796.0, "Mes candidatures");
    texte(&mut f, false, 10.0, MARGE, 776.0, sous_titre);
    let largeur_tableau: f64 = COLONNES.iter().map(|(_, l)| l).sum();
    if lignes.is_empty() {
        texte(&mut f, false, 11.0, MARGE, 730.0, "Aucune candidature pour l'instant.");
    } else {
        // (la ligne des titres sur le jaune de la suite, une ligne sur deux légèrement grisée)
        rectangle(&mut f, (1.0, 0.788, 0.235), MARGE, HAUT_TABLEAU - HAUTEUR_TITRES, largeur_tableau, HAUTEUR_TITRES);
        let mut haut = HAUT_TABLEAU - HAUTEUR_TITRES;
        for i in 0..lignes.len() {
            if i % 2 == 1 {
                rectangle(&mut f, (0.95, 0.95, 0.95), MARGE, haut - HAUTEUR_LIGNE, largeur_tableau, HAUTEUR_LIGNE);
            }
            haut -= HAUTEUR_LIGNE;
        }
        f.push_str("0 0 0 rg\n");
        let titres: Vec<String> = COLONNES.iter().map(|(t, _)| t.to_string()).collect();
        ligne(&mut f, &titres, true, HAUT_TABLEAU, HAUTEUR_TITRES);
        let mut haut = HAUT_TABLEAU - HAUTEUR_TITRES;
        for c in lignes {
            ligne(&mut f, &cases(c), false, haut, HAUTEUR_LIGNE);
            haut -= HAUTEUR_LIGNE;
        }
    }
    let bas = format!("Page {numero} sur {total}");
    texte(&mut f, false, TAILLE, (PAGE_LARGEUR - largeur(&bas, false, TAILLE)) / 2.0, 30.0, &bas);
    f
}

/// Le PDF lisible de la liste des candidatures (A4).
pub fn pdf(liste: &[Candidature], aujourdhui: Jour) -> Vec<u8> {
    let liste = triees(liste);
    // (le jour avec son année, toujours : « jeudi 19 novembre 2026 »)
    let jour = format!("{} {}", aujourdhui.long(aujourdhui), aujourdhui.annee);
    let sous_titre = match liste.len() {
        0 => format!("Exportées le {jour}"),
        1 => format!("Exportée le {jour} : 1 candidature"),
        n => format!("Exportées le {jour} : {n} candidatures"),
    };
    let paquets: Vec<&[&Candidature]> =
        if liste.is_empty() { vec![&[][..]] } else { liste.chunks(LIGNES_PAR_PAGE).collect() };
    let total = paquets.len();

    // Les objets : 1 le catalogue, 2 les pages, 3 et 4 les polices, 5 la fiche, puis chaque page et son contenu.
    let mut objets: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {total} >>",
            (0..total).map(|i| format!("{} 0 R", 6 + 2 * i)).collect::<Vec<_>>().join(" ")
        )
        .into_bytes(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec(),
        format!(
            "<< /Title {} /Producer (Jobmind) /Creator (Jobmind) /CreationDate (D:{:04}{:02}{:02}000000) >>",
            chaine("Mes candidatures"),
            aujourdhui.annee,
            aujourdhui.mois,
            aujourdhui.jour
        )
        .into_bytes(),
    ];
    for (i, lignes) in paquets.iter().enumerate() {
        objets.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_LARGEUR} {PAGE_HAUTEUR}] /Resources << /Font << \
                 /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                7 + 2 * i
            )
            .into_bytes(),
        );
        let contenu = page(lignes, i + 1, total, &sous_titre);
        let mut o = format!("<< /Length {} >>\nstream\n", contenu.len()).into_bytes();
        o.extend(contenu.as_bytes());
        o.extend(b"\nendstream");
        objets.push(o);
    }

    // L'en-tête (sa seconde ligne, binaire, dit aux outils que le fichier n'est pas du texte), les objets, la table
    // de leurs places (lignes de 20 octets), la fin.
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
    p.extend(
        format!("trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n", objets.len() + 1)
            .as_bytes(),
    );
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidature::{Entretien, Moyen};
    use crate::jour::Heure;
    use crate::statut::Statut;

    fn auj() -> Jour {
        Jour::nouveau(2026, 11, 19).unwrap()
    }

    fn j(a: i32, m: u32, d: u32) -> Jour {
        Jour::nouveau(a, m, d).unwrap()
    }

    fn cand(id: i64, entreprise: &str, poste: &str, ville: &str, statut: Statut, envoyee: Jour) -> Candidature {
        Candidature {
            id,
            entreprise: entreprise.into(),
            poste: poste.into(),
            ville: ville.into(),
            statut,
            statut_le: envoyee,
            envoyee_le: envoyee,
            moyen: Moyen::Site,
            source: "Indeed".into(),
            lien: String::new(),
            envoye: "LETTRE SECRÈTE : Madame, Monsieur, je vous adresse ma candidature.".into(),
            cv: "CV cariste confidentiel.pdf".into(),
            notes: String::new(),
            relance_le: None,
            entretien: None,
        }
    }

    /// Quatre candidatures : des accents, des guillemets, un point-virgule, un retour à la ligne, des formules.
    fn liste() -> Vec<Candidature> {
        let mut a = cand(1, "Netéa Services", "Préparateur de commandes", "Woippy", Statut::Negative, j(2026, 11, 2));
        a.notes = "Rappeler ; demander \"M. Klein\"\nle lundi".into();
        a.lien = "https://exemple.fr/offre/42".into();
        a.statut_le = j(2026, 11, 12);
        let mut b =
            cand(2, "Œuvres de Moselle", "Agent d'accueil « polyvalent »", "Metz", Statut::Entretien, j(2026, 11, 10));
        b.moyen = Moyen::Mail;
        b.source = "France Travail".into();
        b.notes = "=SOMME(A1)".into();
        b.relance_le = Some(j(2026, 11, 17));
        b.entretien =
            Some(Entretien { jour: j(2026, 11, 20), heure: Heure::nouvelle(9, 30), lieu: "Zone Actipôle".into() });
        let mut c =
            cand(3, "Boulangerie Ça Marche", "Vendeuse 1 500 € brut", "Thionville", Statut::Envoyee, j(2026, 11, 10));
        c.moyen = Moyen::MainPropre;
        c.source = String::new();
        c.notes = "+33 6 12 34 56 78".into();
        let mut d =
            cand(4, "Garage Löwe", "Mécanicien de flotte légère", "Forbach", Statut::SansReponse, j(2026, 10, 1));
        d.moyen = Moyen::Autre;
        d.source = "@bouche à oreille".into();
        d.notes = " reçu, espaces autour ".into();
        vec![a, b, c, d]
    }

    /// Un petit lecteur CSV (point-virgule, guillemets, retours à la ligne dans un champ, lignes en \r\n).
    fn lire_csv(t: &str) -> Vec<Vec<String>> {
        let mut lignes = Vec::new();
        let mut ligne = Vec::new();
        let mut champ = String::new();
        let mut dans_guillemets = false;
        let mut cs = t.chars().peekable();
        while let Some(c) = cs.next() {
            if dans_guillemets {
                if c == '"' {
                    if cs.peek() == Some(&'"') {
                        cs.next();
                        champ.push('"');
                    } else {
                        dans_guillemets = false;
                    }
                } else {
                    champ.push(c);
                }
                continue;
            }
            match c {
                '"' if champ.is_empty() => dans_guillemets = true,
                ';' => ligne.push(std::mem::take(&mut champ)),
                '\r' => {
                    assert_eq!(cs.next(), Some('\n'), "un \\r seul hors des guillemets");
                    ligne.push(std::mem::take(&mut champ));
                    lignes.push(std::mem::take(&mut ligne));
                }
                '\n' => panic!("un \\n seul hors des guillemets"),
                _ => champ.push(c),
            }
        }
        assert!(champ.is_empty() && ligne.is_empty(), "la dernière ligne finit par \\r\\n");
        lignes
    }

    #[test]
    fn nom_du_fichier() {
        assert_eq!(nom_fichier("csv", auj()), "Mes candidatures 2026-11-19.csv");
        assert_eq!(nom_fichier(".pdf", auj()), "Mes candidatures 2026-11-19.pdf");
    }

    #[test]
    fn le_csv_relu() {
        let octets = csv(&liste(), auj());
        assert!(octets.starts_with(&[0xEF, 0xBB, 0xBF]), "le BOM");
        let t = String::from_utf8(octets[3..].to_vec()).unwrap();
        let l = lire_csv(&t);
        assert_eq!(l[0], ENTETE_CSV.map(String::from).to_vec());
        assert_eq!(
            t.lines().next().unwrap(),
            "Entreprise;Poste;Ville;Statut;Envoyée le;Par où;Offre vue sur;Lien de l'offre;Relancée le;Entretien;\
             Dernier changement;Notes"
        );
        assert_eq!(l.len(), 5);
        assert!(l.iter().all(|x| x.len() == 12));
        // Le tri : le 10/11 (n° 3 puis n° 2), le 02/11, le 01/10
        let entreprises: Vec<&str> = l[1..].iter().map(|x| x[0].as_str()).collect();
        assert_eq!(entreprises, ["Boulangerie Ça Marche", "Œuvres de Moselle", "Netéa Services", "Garage Löwe"]);
        // La ligne complète de l'entretien
        assert_eq!(
            l[2],
            [
                "Œuvres de Moselle",
                "Agent d'accueil « polyvalent »",
                "Metz",
                "Entretien",
                "10/11/2026",
                "Par mail",
                "France Travail",
                "",
                "17/11/2026",
                "20/11/2026 à 9 h 30, Zone Actipôle",
                "10/11/2026",
                "'=SOMME(A1)",
            ]
        );
        // Le point-virgule, les guillemets et le retour à la ligne gardés dans leur champ
        assert_eq!(l[3][11], "Rappeler ; demander \"M. Klein\"\nle lundi");
        assert_eq!(l[3][3], "Réponse négative");
        assert_eq!(l[3][7], "https://exemple.fr/offre/42");
        assert_eq!(l[3][10], "12/11/2026");
        // Les formules neutralisées, « Autrement », les espaces au bord gardées
        assert_eq!(l[1][11], "'+33 6 12 34 56 78");
        assert_eq!(l[1][5], "En main propre");
        assert_eq!(l[1][1], "Vendeuse 1 500 € brut");
        assert_eq!(l[4][6], "'@bouche à oreille");
        assert_eq!(l[4][5], "Autrement");
        assert_eq!(l[4][11], " reçu, espaces autour ");
        assert_eq!(l[4][1], "Mécanicien de flotte légère");
        for a in ["é", "è", "à", "ç", "Ç", "Œ", "«", "»", "€"] {
            assert!(t.contains(a), "{a}");
        }
        // Ni le texte envoyé, ni le CV
        assert!(!t.contains("SECRÈTE") && !t.contains("Madame") && !t.contains("confidentiel"));
        // Liste vide : l'en-tête seul
        assert_eq!(csv(&[], auj()), format!("\u{FEFF}{}\r\n", ENTETE_CSV.join(";")).into_bytes());
    }

    #[test]
    fn champs_du_csv() {
        assert_eq!(champ_csv("simple"), "simple");
        assert_eq!(champ_csv("-5"), "'-5");
        assert_eq!(champ_csv("\tcaché"), "'\tcaché");
        assert_eq!(champ_csv("  =1+1"), "'  =1+1");
        assert_eq!(champ_csv("a\r\nb"), "\"a\r\nb\"");
        assert_eq!(champ_csv("dit \"oui\""), "\"dit \"\"oui\"\"\"");
        assert_eq!(champ_csv("fin "), "\"fin \"");
        assert_eq!(champ_csv(""), "");
    }

    // La relecture d'un PDF, sans bibliothèque

    fn trouver(dans: &[u8], quoi: &[u8], depuis: usize) -> Option<usize> {
        dans[depuis..].windows(quoi.len()).position(|w| w == quoi).map(|p| p + depuis)
    }

    /// Vérifie la structure : l'en-tête, la table des places (chaque place tombe sur « N 0 obj »), la fin, la longueur
    /// de chaque flux, la fiche. Rend le nombre d'entrées de la table (les objets plus un).
    fn verifier_structure(p: &[u8]) -> usize {
        assert!(p.starts_with(b"%PDF-1.4\n%"));
        assert!(p[10..14].iter().all(|&o| o >= 128), "la ligne binaire");
        assert!(p.ends_with(b"%%EOF\n"));
        let s = trouver(p, b"startxref\n", 0).unwrap();
        let fin = trouver(p, b"\n", s + 10).unwrap();
        let xref: usize = std::str::from_utf8(&p[s + 10..fin]).unwrap().parse().unwrap();
        assert!(p[xref..].starts_with(b"xref\n0 "));
        let nl = trouver(p, b"\n", xref + 5).unwrap();
        let n: usize = std::str::from_utf8(&p[xref + 7..nl]).unwrap().parse().unwrap();
        let table = nl + 1;
        assert_eq!(&p[table..table + 20], b"0000000000 65535 f \n");
        for k in 1..n {
            let e = &p[table + 20 * k..table + 20 * (k + 1)];
            assert!(e.ends_with(b" 00000 n \n"), "{:?}", String::from_utf8_lossy(e));
            let place: usize = std::str::from_utf8(&e[..10]).unwrap().parse().unwrap();
            assert!(p[place..].starts_with(format!("{k} 0 obj\n").as_bytes()), "objet {k}");
        }
        let trailer = String::from_utf8_lossy(&p[table + 20 * n..]);
        assert!(trailer.starts_with(&format!("trailer\n<< /Size {n} /Root 1 0 R /Info 5 0 R >>")), "{trailer}");
        // Chaque flux a la longueur annoncée
        let mut depuis = 0;
        while let Some(d) = trouver(p, b"stream\n", depuis) {
            depuis = d + 7;
            if p[..d].ends_with(b"end") {
                continue;
            }
            let l = trouver(p, b"/Length ", d.saturating_sub(40)).unwrap();
            let fin = trouver(p, b" ", l + 8).unwrap();
            let longueur: usize = std::str::from_utf8(&p[l + 8..fin]).unwrap().parse().unwrap();
            assert!(p[d + 7 + longueur..].starts_with(b"\nendstream"));
        }
        let brut = String::from_utf8_lossy(p);
        assert!(brut.contains("/Producer (Jobmind)") && brut.contains("/Title (Mes candidatures)"));
        n
    }

    /// Un texte posé : police grasse ou non, taille, x, y, texte redécodé de WinAnsi.
    struct Pose {
        gras: bool,
        taille: f64,
        x: f64,
        y: f64,
        texte: String,
    }

    fn decoder(s: &[u8]) -> String {
        let mut octets = Vec::new();
        let mut i = 0;
        while i < s.len() {
            if s[i] == b'\\' {
                if s[i + 1].is_ascii_digit() {
                    let o = std::str::from_utf8(&s[i + 1..i + 4]).unwrap();
                    octets.push(u8::from_str_radix(o, 8).unwrap());
                    i += 4;
                } else {
                    octets.push(s[i + 1]);
                    i += 2;
                }
            } else {
                octets.push(s[i]);
                i += 1;
            }
        }
        octets.into_iter().map(|o| depuis_winansi(o).expect("un octet hors de WinAnsi")).collect()
    }

    /// Les textes posés, page par page.
    fn textes(p: &[u8]) -> Vec<Vec<Pose>> {
        let mut pages = Vec::new();
        let mut depuis = 0;
        while let Some(d) = trouver(p, b"stream\n", depuis) {
            depuis = d + 7;
            if p[..d].ends_with(b"end") {
                continue;
            }
            let f = trouver(p, b"\nendstream", d).unwrap();
            let mut poses = Vec::new();
            for l in p[d + 7..f].split(|&o| o == b'\n') {
                let Some(reste) = l.strip_prefix(b"BT /F") else { continue };
                let ouv = reste.iter().position(|&o| o == b'(').unwrap();
                assert!(reste.ends_with(b") Tj ET"));
                let tete: Vec<&str> = std::str::from_utf8(&reste[..ouv]).unwrap().split_whitespace().collect();
                poses.push(Pose {
                    gras: tete[0] == "2",
                    taille: tete[1].parse().unwrap(),
                    x: tete[3].parse().unwrap(),
                    y: tete[4].parse().unwrap(),
                    texte: decoder(&reste[ouv + 1..reste.len() - 7]),
                });
            }
            pages.push(poses);
        }
        pages
    }

    /// Chaque texte du tableau tient dans sa colonne (son bord moins le retrait).
    fn tient_dans_sa_colonne(pages: &[Vec<Pose>]) {
        for pose in pages.iter().flatten().filter(|p| p.taille == TAILLE && p.y > 50.0) {
            let mut debut = MARGE;
            let mut trouve = false;
            for (_, l) in COLONNES {
                if pose.x >= debut && pose.x < debut + l {
                    let fin = pose.x + largeur(&pose.texte, pose.gras, pose.taille);
                    assert!(fin <= debut + l - RETRAIT + 0.01, "{} déborde ({fin} > {})", pose.texte, debut + l);
                    trouve = true;
                }
                debut += l;
            }
            assert!(trouve, "{} hors du tableau", pose.texte);
        }
    }

    #[test]
    fn le_pdf_relu() {
        let p = pdf(&liste(), auj());
        assert_eq!(verifier_structure(&p), 8);
        let pages = textes(&p);
        assert_eq!(pages.len(), 1);
        let tous: Vec<&str> = pages[0].iter().map(|x| x.texte.as_str()).collect();
        for attendu in [
            "Mes candidatures",
            "Exportées le jeudi 19 novembre 2026 : 4 candidatures",
            "Netéa Services",
            "Préparateur de commandes",
            "Réponse négative",
            "Œuvres de Moselle",
            "Agent d'accueil « polyvalent »",
            "Vendeuse 1 500 € brut",
            "Mécanicien de flotte légère",
            "Garage Löwe",
            "Envoyée le",
            "Offre vue sur",
            "Page 1 sur 1",
        ] {
            assert!(tous.contains(&attendu), "{attendu} manque dans {tous:?}");
        }
        // Rien de secret ni de long
        let brut = String::from_utf8_lossy(&p);
        assert!(!brut.contains("SECR") && !brut.contains("confidentiel") && !brut.contains("Rappeler"));
        // Le tri : la première ligne est celle de la boulangerie
        let x_entreprise = MARGE + COLONNES[0].1 + RETRAIT;
        let entreprises: Vec<&str> = pages[0]
            .iter()
            .filter(|x| !x.gras && (x.x - x_entreprise).abs() < 0.01)
            .map(|x| x.texte.as_str())
            .collect();
        assert_eq!(entreprises, ["Boulangerie Ça Marche", "Œuvres de Moselle", "Netéa Services", "Garage Löwe"]);
        tient_dans_sa_colonne(&pages);
        // Les titres des colonnes ne sont jamais coupés, et le tableau tient dans la page
        for (t, l) in COLONNES {
            assert!(largeur(t, true, TAILLE) <= l - 2.0 * RETRAIT, "{t}");
        }
        assert_eq!(COLONNES.iter().map(|(_, l)| l).sum::<f64>(), PAGE_LARGEUR - 2.0 * MARGE);
    }

    #[test]
    fn beaucoup_de_candidatures_sur_plusieurs_pages() {
        let liste: Vec<Candidature> = (0..130)
            .map(|i| {
                let mut c = cand(
                    i,
                    &format!("Entreprise n° {i}"),
                    "Cariste",
                    "Saint-Avold",
                    Statut::TOUS[i as usize % 7],
                    j(2026, 11, 19).plus(-(i / 3)),
                );
                if i == 77 {
                    c.entreprise = "Société Lorraine de Logistique et de Transports Frigorifiques du Grand Est \
                                    (site de Hauconcourt)"
                        .into();
                }
                c
            })
            .collect();
        let p = pdf(&liste, auj());
        assert_eq!(verifier_structure(&p), 6 + 2 * 3);
        assert!(String::from_utf8_lossy(&p).contains("/Count 3"));
        let pages = textes(&p);
        assert_eq!(pages.len(), 3);
        assert_eq!(LIGNES_PAR_PAGE, 45);
        for (k, page) in pages.iter().enumerate() {
            let bas: Vec<&str> =
                page.iter().filter(|x| x.texte.starts_with("Page ")).map(|x| x.texte.as_str()).collect();
            assert_eq!(bas, [format!("Page {} sur 3", k + 1)]);
            assert!(page.iter().any(|x| x.texte == "Exportées le jeudi 19 novembre 2026 : 130 candidatures"));
            assert!(page.iter().any(|x| x.texte == "Entreprise" && x.gras), "les titres sur chaque page");
            let lignes = page.iter().filter(|x| x.texte == "Cariste").count();
            assert_eq!(lignes, [45, 45, 40][k]);
        }
        // Chaque candidature une seule fois, et la longue coupée
        let tous: Vec<&str> = pages.iter().flatten().map(|x| x.texte.as_str()).collect();
        for i in (0..130).filter(|&i| i != 77) {
            let e = format!("Entreprise n° {i}");
            assert_eq!(tous.iter().filter(|t| **t == e).count(), 1, "{e}");
        }
        let coupee = tous.iter().find(|t| t.starts_with("Société Lorraine")).unwrap();
        assert!(coupee.ends_with("...") && !coupee.ends_with(" ..."), "{coupee}");
        assert!(largeur(coupee, false, TAILLE) <= COLONNES[1].1 - 2.0 * RETRAIT);
        assert!(coupee.chars().count() < 40, "{coupee}");
        tient_dans_sa_colonne(&pages);
    }

    #[test]
    fn une_liste_vide_et_des_caracteres_hors_table() {
        let p = pdf(&[], auj());
        assert_eq!(verifier_structure(&p), 8);
        let pages = textes(&p);
        let tous: Vec<&str> = pages[0].iter().map(|x| x.texte.as_str()).collect();
        assert_eq!(
            tous,
            [
                "Mes candidatures",
                "Exportées le jeudi 19 novembre 2026",
                "Aucune candidature pour l'instant.",
                "Page 1 sur 1"
            ]
        );
        // Une seule : au singulier ; un caractère hors de WinAnsi devient « ? », une parenthèse est échappée, une
        // espace fine devient une espace
        let c = cand(9, "Łódź (Pologne)", "Agent\u{202F}: « tri »", "Nancy", Statut::Relancee, j(2025, 3, 4));
        let p = pdf(&[c], auj());
        verifier_structure(&p);
        let tous: Vec<String> = textes(&p).remove(0).into_iter().map(|x| x.texte).collect();
        for attendu in [
            "?ód? (Pologne)",
            "Agent : « tri »",
            "Relancée",
            "04/03/2025",
            "Exportée le jeudi 19 novembre 2026 : 1 candidature",
        ] {
            assert!(tous.iter().any(|t| t == attendu), "{attendu} manque dans {tous:?}");
        }
        assert!(String::from_utf8_lossy(&p).contains("(?\\363d? \\(Pologne\\)) Tj"));
    }

    #[test]
    fn largeurs_et_coupes() {
        // (valeurs du fichier AFM d'Adobe : H 722, e 556, l 222, v 500, t 278, i 222, c 500, a 556)
        assert_eq!(largeur("Helvetica", false, 1000.0), 4112.0);
        // (en gras : H 722, e 556, l 278, v 556, t 333, i 278, c 556, a 556)
        assert_eq!(largeur("Helvetica", true, 1000.0), 4391.0);
        assert_eq!(largeur("é", false, 10.0), 5.56);
        assert_eq!(largeur("œŒ€", false, 1000.0), 944.0 + 1000.0 + 556.0);
        assert_eq!(largeur("\u{1F600}", false, 1000.0), 556.0);
        assert_eq!(couper("court", 100.0, false, TAILLE), "court");
        let c = couper("Préparateur de commandes en chambre froide", 80.0, false, TAILLE);
        assert!(c.ends_with("...") && largeur(&c, false, TAILLE) <= 80.0, "{c}");
        assert!(!c.contains(['\u{2013}', '\u{2014}']));
        for o in 0x20..=0xFFu8 {
            if let Some(ch) = depuis_winansi(o) {
                assert_eq!(winansi(ch), Some(o));
            }
        }
        assert_eq!(winansi('\u{1F600}'), None);
    }
}
