//! La lecture d'un mail (RFC 5322, MIME : RFC 2045 à 2047, et 2231 pour les noms simples), écrite ici : un mail est
//! une donnée NON FIABLE (il vient de n'importe qui), donc [`lire`] ne panique jamais, ne refuse jamais (un mail abîmé
//! donne un [`Mail`] plus pauvre) et borne tout : 200 en-têtes, 10 niveaux de parties imbriquées, 200 parties, un texte
//! de 20 000 caractères, un HTML de 2 000 000 d'octets, un objet de 300 caractères.
//!
//! Le texte retenu : la partie text/plain si elle existe, sinon le texte tiré de la partie HTML (scripts, styles et
//! balises retirés, entités lues). Les pièces jointes sont comptées, jamais décodées ni gardées. Les tirets longs sont
//! remplacés par « - », les caractères de contrôle et invisibles retirés.

use crate::{DateMail, Mail};

const TEXTE_MAX: usize = 20_000;
const HTML_MAX: usize = 2_000_000;
const OBJET_MAX: usize = 300;
const NOM_MAX: usize = 200;
const ENTETES_MAX: usize = 200;
/// La plus longue valeur d'un en-tête gardée (octets)
const ENTETE_MAX: usize = 20_000;
const PROFONDEUR_MAX: usize = 10;
const PARTIES_MAX: usize = 200;
/// Ce qui est décodé d'une partie text/plain (octets) : bien assez pour 20 000 caractères
const BRUT_TEXTE_MAX: usize = 400_000;
const REFERENCES_MAX: usize = 20;
const DESTINATAIRES_MAX: usize = 100;
const ID_MAX: usize = 500;

type Entetes = Vec<(String, String)>;

/// Lit un mail brut (tel que rendu par BODY.PEEK[]).
pub fn lire(uid: u32, octets: &[u8]) -> Mail {
    let (tete, corps) = separer(octets);
    let e = entetes(tete);
    let mut m = Mail { uid, ..Default::default() };
    m.message_id = ids(valeur(&e, "message-id")).into_iter().next().unwrap_or_default();
    let de = adresses(valeur(&e, "from"));
    if let Some((a, n)) = de.iter().find(|(a, _)| !a.is_empty()).or(de.first()) {
        m.de_adresse = a.clone();
        m.de_nom = n.clone();
    }
    m.repondre_a =
        adresses(valeur(&e, "reply-to")).into_iter().map(|(a, _)| a).find(|a| !a.is_empty()).unwrap_or_default();
    for (a, _) in adresses(valeur(&e, "to")) {
        if !a.is_empty() && !m.a.contains(&a) && m.a.len() < DESTINATAIRES_MAX {
            m.a.push(a);
        }
    }
    m.objet = propre_ligne(&mots_encodes(valeur(&e, "subject")), OBJET_MAX);
    m.date = lire_date(valeur(&e, "date"));
    m.en_reponse_a = references(valeur(&e, "in-reply-to"), valeur(&e, "references"));
    let precedence = valeur(&e, "precedence").trim().to_ascii_lowercase();
    m.masse = e.iter().any(|(n, _)| n == "list-unsubscribe") || precedence == "bulk" || precedence == "list";

    let mut etat = Etat::default();
    partie(&e, corps, 0, &mut etat);
    m.pieces_jointes = etat.pieces_jointes;
    m.html = etat.html.unwrap_or_default();
    m.texte = match etat.texte {
        Some(t) => t,
        None if !m.html.is_empty() => propre_texte(&html_en_texte(&m.html), TEXTE_MAX),
        None => String::new(),
    };
    m
}

/// Ce que la lecture des parties a trouvé.
#[derive(Default)]
struct Etat {
    texte: Option<String>,
    html: Option<String>,
    pieces_jointes: usize,
    parties: usize,
}

// ---------------------------------------------------------------- En-têtes

/// Sépare l'en-tête du corps (à la première ligne vide). Sans ligne vide : tout est en-tête.
fn separer(o: &[u8]) -> (&[u8], &[u8]) {
    if o.starts_with(b"\r\n") {
        return (&[], &o[2..]);
    }
    if o.starts_with(b"\n") {
        return (&[], &o[1..]);
    }
    let mut i = 0;
    while let Some(p) = o[i..].iter().position(|&c| c == b'\n') {
        let n = i + p;
        match o.get(n + 1) {
            Some(b'\n') => return (&o[..n + 1], &o[n + 2..]),
            Some(b'\r') if o.get(n + 2) == Some(&b'\n') => return (&o[..n + 1], &o[n + 3..]),
            _ => i = n + 1,
        }
    }
    (o, &[])
}

/// Les en-têtes (nom en minuscules, valeur dépliée), 200 au plus. Une ligne qui n'est pas un en-tête est passée.
fn entetes(tete: &[u8]) -> Entetes {
    let mut r: Vec<(String, Vec<u8>)> = Vec::new();
    let mut courant = false;
    for ligne in tete.split(|&c| c == b'\n') {
        let ligne = ligne.strip_suffix(b"\r").unwrap_or(ligne);
        if ligne.first().is_some_and(|&c| c == b' ' || c == b'\t') {
            if courant && let Some((_, v)) = r.last_mut() {
                let place = ENTETE_MAX.saturating_sub(v.len());
                v.extend_from_slice(&ligne[..ligne.len().min(place)]);
            }
            continue;
        }
        courant = false;
        let Some(p) = ligne.iter().position(|&c| c == b':') else { continue };
        let nom = &ligne[..p];
        if nom.is_empty() || nom.len() > 76 || !nom.iter().all(|&c| (33..=126).contains(&c)) {
            continue;
        }
        if r.len() >= ENTETES_MAX {
            break;
        }
        let v = &ligne[p + 1..];
        r.push((String::from_utf8_lossy(nom).to_ascii_lowercase(), v[..v.len().min(ENTETE_MAX)].to_vec()));
        courant = true;
    }
    r.into_iter().map(|(n, v)| (n, octets_en_texte(&v, "").trim().to_string())).collect()
}

/// La valeur du premier en-tête de ce nom (vide s'il n'y en a pas).
fn valeur<'a>(e: &'a Entetes, nom: &str) -> &'a str {
    e.iter().find(|(n, _)| n == nom).map(|(_, v)| v.as_str()).unwrap_or("")
}

// ---------------------------------------------------------------- Jeux de caractères

/// Windows-1252, de 0x80 à 0x9F (les cases vides gardent leur code de contrôle, retiré ensuite).
const CP1252: [u32; 32] = [
    0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x8D, 0x017D,
    0x8F, 0x90, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x9D,
    0x017E, 0x0178,
];

fn cp1252(o: &[u8]) -> String {
    o.iter()
        .map(|&b| match b {
            0x80..=0x9F => char::from_u32(CP1252[(b - 0x80) as usize]).unwrap_or('\u{FFFD}'),
            _ => char::from(b),
        })
        .collect()
}

fn latin9(o: &[u8]) -> String {
    o.iter()
        .map(|&b| match b {
            0xA4 => '\u{20AC}',
            0xA6 => '\u{0160}',
            0xA8 => '\u{0161}',
            0xB4 => '\u{017D}',
            0xB8 => '\u{017E}',
            0xBC => '\u{0152}',
            0xBD => '\u{0153}',
            0xBE => '\u{0178}',
            _ => char::from(b),
        })
        .collect()
}

/// Des octets en texte selon leur jeu de caractères. Sans jeu dit : UTF-8 s'il est valide, sinon Windows-1252.
/// Inconnu : UTF-8, les octets faux remplacés.
fn octets_en_texte(o: &[u8], charset: &str) -> String {
    let c = charset.trim().trim_matches('"').trim().to_ascii_lowercase();
    match c.as_str() {
        "" | "us-ascii" | "ascii" | "us_ascii" => match std::str::from_utf8(o) {
            Ok(s) => s.to_string(),
            Err(_) => cp1252(o),
        },
        // (comme les navigateurs : ISO-8859-1 annoncé est presque toujours du Windows-1252)
        "iso-8859-1" | "iso8859-1" | "iso_8859-1" | "latin1" | "latin-1" | "l1" | "windows-1252" | "cp1252"
        | "x-cp1252" | "cp-1252" => cp1252(o),
        "iso-8859-15" | "iso8859-15" | "iso_8859-15" | "latin9" | "latin-9" | "l9" => latin9(o),
        _ => String::from_utf8_lossy(o).into_owned(),
    }
}

// ---------------------------------------------------------------- Encodages de transfert

fn valeur_base64(c: u8) -> Option<u32> {
    Some(match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' | b'-' => 62,
        b'/' | b'_' => 63,
        _ => return None,
    } as u32)
}

/// Base64, les caractères parasites passés, `max` octets au plus.
fn base64(e: &[u8], max: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity((e.len() / 4 * 3).min(max));
    let (mut acc, mut n) = (0u32, 0u32);
    for &c in e {
        if out.len() >= max {
            break;
        }
        if c == b'=' {
            // (fin d'un bloc : les octets en attente, puis on repart à zéro)
            match n {
                2 => out.push((acc >> 4) as u8),
                3 => out.extend_from_slice(&[(acc >> 10) as u8, (acc >> 2) as u8]),
                _ => {}
            }
            acc = 0;
            n = 0;
            continue;
        }
        let Some(v) = valeur_base64(c) else { continue };
        acc = (acc << 6) | v;
        n += 1;
        if n == 4 {
            out.extend_from_slice(&[(acc >> 16) as u8, (acc >> 8) as u8, acc as u8]);
            acc = 0;
            n = 0;
        }
    }
    match n {
        2 => out.push((acc >> 4) as u8),
        3 => out.extend_from_slice(&[(acc >> 10) as u8, (acc >> 2) as u8]),
        _ => {}
    }
    out.truncate(max);
    out
}

fn hexa(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|v| v as u8)
}

/// Quoted-printable (fins de ligne douces « = » retirées), `max` octets au plus. `souligne` : le Q des en-têtes
/// (« _ » est une espace).
fn quoted_printable(e: &[u8], max: usize, souligne: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(e.len().min(max));
    let mut i = 0;
    while i < e.len() && out.len() < max {
        let c = e[i];
        if c == b'=' {
            // (fin de ligne douce, même suivie d'espaces)
            let mut j = i + 1;
            while j < e.len() && (e[j] == b' ' || e[j] == b'\t') {
                j += 1;
            }
            if j >= e.len() {
                break;
            }
            if e[j] == b'\n' {
                i = j + 1;
                continue;
            }
            if e[j] == b'\r' && e.get(j + 1) == Some(&b'\n') {
                i = j + 2;
                continue;
            }
            if let (Some(h), Some(l)) = (e.get(i + 1).and_then(|&c| hexa(c)), e.get(i + 2).and_then(|&c| hexa(c))) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
            out.push(b'=');
        } else if souligne && c == b'_' {
            out.push(b' ');
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// Le corps d'une partie selon son Content-Transfer-Encoding (7bit, 8bit, binary : tel quel).
fn decoder(cte: &str, corps: &[u8], max: usize) -> Vec<u8> {
    match cte.trim().to_ascii_lowercase().as_str() {
        "base64" => base64(corps, max),
        "quoted-printable" => quoted_printable(corps, max, false),
        _ => corps[..corps.len().min(max)].to_vec(),
    }
}

// ---------------------------------------------------------------- Mots encodés (RFC 2047)

/// Un mot encodé au début de `s` (« =?utf-8?B?...?= ») : son jeu de caractères, ses octets, et ce qui suit.
fn mot_encode(s: &str) -> Option<(String, Vec<u8>, &str)> {
    let reste = s.strip_prefix("=?")?;
    let q1 = reste.find('?')?;
    let charset = &reste[..q1];
    if charset.is_empty() || charset.len() > 64 || charset.contains(|c: char| c.is_whitespace()) {
        return None;
    }
    let apres = &reste[q1 + 1..];
    let mut ch = apres.chars();
    let enc = ch.next()?.to_ascii_uppercase();
    if ch.next()? != '?' || (enc != 'B' && enc != 'Q') {
        return None;
    }
    let texte_debut = &apres[2..];
    let fin = texte_debut.find("?=")?;
    let texte = &texte_debut[..fin];
    if texte.len() > 4000 {
        return None;
    }
    let octets = if enc == 'B' {
        base64(texte.as_bytes(), usize::MAX)
    } else {
        quoted_printable(texte.as_bytes(), usize::MAX, true)
    };
    // (RFC 2231 : « UTF-8*fr » ; la langue est passée)
    let cs = charset.split('*').next().unwrap_or("").to_string();
    Some((cs, octets, &texte_debut[fin + 2..]))
}

/// Décode les mots encodés d'un en-tête. Les espaces entre deux mots encodés sont retirées, et deux mots voisins du
/// même jeu de caractères sont décodés ensemble (une lettre peut être coupée entre les deux).
fn mots_encodes(s: &str) -> String {
    let mut out = String::new();
    let mut attente: Option<(String, Vec<u8>)> = None;
    let mut reste = s;
    let vider = |attente: &mut Option<(String, Vec<u8>)>, out: &mut String| {
        if let Some((cs, o)) = attente.take() {
            out.push_str(&octets_en_texte(&o, if cs.is_empty() { "utf-8" } else { &cs }));
        }
    };
    loop {
        // le prochain mot encodé qui se lit
        let mut depuis = 0;
        let mut trouve = None;
        while let Some(p) = reste[depuis..].find("=?") {
            let i = depuis + p;
            if let Some(m) = mot_encode(&reste[i..]) {
                trouve = Some((i, m));
                break;
            }
            depuis = i + 2;
        }
        let Some((i, (cs, octets, apres))) = trouve else {
            vider(&mut attente, &mut out);
            out.push_str(reste);
            return out;
        };
        let avant = &reste[..i];
        if !(attente.is_some() && avant.trim().is_empty()) {
            vider(&mut attente, &mut out);
            out.push_str(avant);
        }
        match &mut attente {
            Some((c, o)) if c.eq_ignore_ascii_case(&cs) => o.extend_from_slice(&octets),
            _ => {
                vider(&mut attente, &mut out);
                attente = Some((cs, octets));
            }
        }
        reste = apres;
    }
}

// ---------------------------------------------------------------- Adresses, identifiants, date

/// Coupe une liste d'adresses en éléments (virgules hors guillemets, chevrons et commentaires ; les groupes
/// « Nom: a@b, c@d; » donnent leurs adresses).
fn elements(v: &str) -> Vec<String> {
    let mut r = Vec::new();
    let mut cur = String::new();
    let (mut guillemets, mut commentaire, mut chevron, mut echappe) = (false, 0usize, false, false);
    for c in v.chars() {
        if echappe {
            cur.push(c);
            echappe = false;
            continue;
        }
        match c {
            '\\' if guillemets || commentaire > 0 => {
                cur.push(c);
                echappe = true;
                continue;
            }
            '"' if commentaire == 0 => guillemets = !guillemets,
            '(' if !guillemets => commentaire += 1,
            ')' if !guillemets && commentaire > 0 => commentaire -= 1,
            '<' if !guillemets && commentaire == 0 => chevron = true,
            '>' if !guillemets && commentaire == 0 => chevron = false,
            ',' | ';' if !guillemets && commentaire == 0 && !chevron => {
                r.push(std::mem::take(&mut cur));
                continue;
            }
            ':' if !guillemets && commentaire == 0 && !chevron => {
                // (le nom d'un groupe : passé)
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(c);
        if r.len() >= DESTINATAIRES_MAX {
            break;
        }
    }
    r.push(cur);
    r.into_iter().filter(|e| !e.trim().is_empty()).collect()
}

/// Retire les guillemets (et leurs échappements) et met les commentaires à part : (texte, commentaires).
fn sans_guillemets(s: &str) -> (String, String) {
    let (mut texte, mut comm) = (String::new(), String::new());
    let (mut guillemets, mut commentaire, mut echappe) = (false, 0usize, false);
    for c in s.chars() {
        let dest = if commentaire > 0 { &mut comm } else { &mut texte };
        if echappe {
            dest.push(c);
            echappe = false;
            continue;
        }
        match c {
            '\\' if guillemets || commentaire > 0 => echappe = true,
            '"' if commentaire == 0 => guillemets = !guillemets,
            '(' if !guillemets => {
                if commentaire > 0 {
                    comm.push(c);
                }
                commentaire += 1;
            }
            ')' if !guillemets && commentaire > 0 => {
                commentaire -= 1;
                if commentaire > 0 {
                    comm.push(c);
                } else {
                    comm.push(' ');
                }
            }
            _ => dest.push(c),
        }
    }
    (texte, comm)
}

/// Une adresse en minuscules, si elle en a l'air (sinon vide).
fn adresse_propre(a: &str) -> String {
    let a: String = a.chars().filter(|c| !c.is_whitespace() && !c.is_control()).collect::<String>().to_lowercase();
    let a = a.strip_prefix("mailto:").unwrap_or(&a);
    // (route source « @a,@b:x@y » : la fin)
    let a = a.rsplit(':').next().unwrap_or(a);
    let a = a.trim_matches(|c| c == '<' || c == '>' || c == '"' || c == '\'');
    if a.len() <= 320 && a.contains('@') && !a.starts_with('@') && !a.ends_with('@') {
        a.to_string()
    } else {
        String::new()
    }
}

/// Les (adresse, nom) d'un en-tête From, To, Reply-To.
fn adresses(v: &str) -> Vec<(String, String)> {
    let mut r = Vec::new();
    for e in elements(v) {
        // le chevron hors guillemets
        let (mut guillemets, mut ouvre) = (false, None);
        let mut prec = ' ';
        for (i, c) in e.char_indices() {
            if c == '"' && prec != '\\' {
                guillemets = !guillemets;
            } else if c == '<' && !guillemets {
                ouvre = Some(i);
            }
            prec = c;
        }
        let (adresse, nom) = match ouvre {
            Some(i) => {
                let dedans = &e[i + 1..];
                let ferme = dedans.find('>').unwrap_or(dedans.len());
                let (n, comm) = sans_guillemets(&e[..i]);
                let n = if n.trim().is_empty() { comm } else { n };
                (adresse_propre(&dedans[..ferme]), n)
            }
            None => {
                let (a, comm) = sans_guillemets(&e);
                (adresse_propre(&a), comm)
            }
        };
        let nom = propre_ligne(&mots_encodes(nom.trim()), NOM_MAX);
        if !adresse.is_empty() || !nom.is_empty() {
            r.push((adresse, nom));
        }
    }
    r
}

/// Les Message-ID d'un en-tête, sans < >.
fn ids(v: &str) -> Vec<String> {
    let mut r = Vec::new();
    let mut reste = v;
    while let Some(o) = reste.find('<') {
        let apres = &reste[o + 1..];
        let Some(f) = apres.find('>') else { break };
        let id: String = apres[..f].chars().filter(|c| !c.is_whitespace() && !c.is_control()).collect();
        if !id.is_empty() && id.len() <= ID_MAX {
            r.push(id);
        }
        reste = &apres[f + 1..];
        if r.len() >= 1000 {
            break;
        }
    }
    if r.is_empty() {
        for mot in v.split_whitespace().filter(|m| m.contains('@')) {
            let id: String = mot.trim_matches(|c| c == '<' || c == '>').chars().filter(|c| !c.is_control()).collect();
            if !id.is_empty() && id.len() <= ID_MAX {
                r.push(id);
            }
        }
    }
    r
}

/// In-Reply-To puis References, sans doublons, 20 au plus (de References, les derniers cités : les plus proches).
fn references(en_reponse: &str, refs: &str) -> Vec<String> {
    let mut r: Vec<String> = Vec::new();
    for id in ids(en_reponse) {
        if !r.contains(&id) && r.len() < REFERENCES_MAX {
            r.push(id);
        }
    }
    let mut autres: Vec<String> = ids(refs).into_iter().filter(|id| !r.contains(id)).collect();
    autres.dedup();
    let mut vus = Vec::new();
    for id in autres.into_iter().rev() {
        if !vus.contains(&id) {
            vus.push(id);
        }
    }
    let place = REFERENCES_MAX.saturating_sub(r.len());
    vus.truncate(place);
    vus.reverse();
    r.extend(vus);
    r
}

/// La date d'un mail (« Wed, 19 Nov 2026 08:30:00 +0100 (CET) », sans jour de la semaine, année à 2 chiffres...).
fn lire_date(v: &str) -> Option<DateMail> {
    let (sans_comm, _) = sans_guillemets(v);
    let s = match sans_comm.find(',') {
        Some(p) if sans_comm[..p].trim().chars().all(|c| c.is_ascii_alphabetic()) => sans_comm[p + 1..].to_string(),
        _ => sans_comm,
    };
    let mots: Vec<&str> = s.split_whitespace().collect();
    if mots.len() < 4 {
        return None;
    }
    // (« 19 Nov 2026 » ou, plus rare, « Nov 19 2026 »)
    let (j, m) = if mots[0].chars().all(|c| c.is_ascii_digit()) { (mots[0], mots[1]) } else { (mots[1], mots[0]) };
    let jour: u32 = j.parse().ok().filter(|j| (1..=31).contains(j))?;
    let m3: String = m.chars().take(3).collect::<String>().to_ascii_lowercase();
    let mois = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"]
        .iter()
        .position(|x| *x == m3)? as u32
        + 1;
    let a = mots[2];
    if a.is_empty() || a.len() > 4 || !a.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut annee: i32 = a.parse().ok()?;
    if a.len() <= 2 {
        annee += if annee < 50 { 2000 } else { 1900 };
    } else if a.len() == 3 {
        annee += 1900;
    }
    let h: Vec<&str> = mots[3].split(':').collect();
    if h.len() < 2 || h.len() > 3 {
        return None;
    }
    let nombre = |t: &str, max: u32| t.parse::<u32>().ok().filter(|n| *n <= max && t.len() <= 2);
    let heure = nombre(h[0], 23)?;
    let minute = nombre(h[1], 59)?;
    let seconde = if h.len() == 3 { nombre(h[2], 60)? } else { 0 };
    let decalage_minutes = mots.get(4).map(|z| zone(z)).unwrap_or(0);
    Some(DateMail { annee, mois, jour, heure, minute, seconde, decalage_minutes })
}

fn zone(z: &str) -> i32 {
    let b = z.as_bytes();
    if b.len() == 5 && (b[0] == b'+' || b[0] == b'-') && b[1..].iter().all(u8::is_ascii_digit) {
        let h = i32::from(b[1] - b'0') * 10 + i32::from(b[2] - b'0');
        let m = i32::from(b[3] - b'0') * 10 + i32::from(b[4] - b'0');
        let t = h * 60 + m;
        return if b[0] == b'-' { -t } else { t };
    }
    match z.to_ascii_uppercase().as_str() {
        "EST" => -300,
        "EDT" => -240,
        "CST" => -360,
        "CDT" => -300,
        "MST" => -420,
        "MDT" => -360,
        "PST" => -480,
        "PDT" => -420,
        "CET" => 60,
        "CEST" => 120,
        _ => 0,
    }
}

// ---------------------------------------------------------------- Parties

/// Le type et les paramètres d'un Content-Type ou d'un Content-Disposition (noms en minuscules ; RFC 2231 simple :
/// « name*=utf-8''%C3%A9t%C3%A9.pdf », et les suites « name*0= », « name*1= »).
fn type_et_parametres(v: &str) -> (String, Vec<(String, String)>) {
    let mut morceaux = Vec::new();
    let (mut cur, mut guillemets, mut echappe) = (String::new(), false, false);
    for c in v.chars() {
        if echappe {
            cur.push(c);
            echappe = false;
            continue;
        }
        match c {
            '\\' if guillemets => echappe = true,
            '"' => guillemets = !guillemets,
            ';' if !guillemets => morceaux.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
        if morceaux.len() > 50 {
            break;
        }
    }
    morceaux.push(cur);
    let type_ = morceaux.first().map(|t| t.trim().to_ascii_lowercase()).unwrap_or_default();
    // (nom de base, numéro de suite, étendu ?, valeur)
    let mut bruts: Vec<(String, usize, bool, String)> = Vec::new();
    for m in morceaux.iter().skip(1) {
        let Some((k, v)) = m.split_once('=') else { continue };
        let k = k.trim().to_ascii_lowercase();
        let v = v.trim().to_string();
        let etendu = k.ends_with('*');
        let k = k.trim_end_matches('*');
        let (base, num) = match k.split_once('*') {
            Some((b, n)) => (b.to_string(), n.parse().unwrap_or(0)),
            None => (k.to_string(), 0),
        };
        bruts.push((base, num, etendu, v));
    }
    let mut params: Vec<(String, String)> = Vec::new();
    let mut bases: Vec<String> = bruts.iter().map(|b| b.0.clone()).collect();
    bases.dedup();
    for base in bases {
        if params.iter().any(|(k, _)| *k == base) {
            continue;
        }
        let mut seg: Vec<&(String, usize, bool, String)> = bruts.iter().filter(|b| b.0 == base).collect();
        seg.sort_by_key(|b| b.1);
        let mut charset = String::new();
        let mut octets = Vec::new();
        let mut etendu_vu = false;
        for (i, (_, _, etendu, v)) in seg.iter().map(|b| (b.0.as_str(), b.1, b.2, b.3.as_str())).enumerate() {
            if etendu {
                etendu_vu = true;
                let mut v = v;
                if i == 0 {
                    let mut p = v.splitn(3, '\'');
                    if let (Some(c), Some(_), Some(reste)) = (p.next(), p.next(), p.next()) {
                        charset = c.to_string();
                        v = reste;
                    }
                }
                octets.extend(pourcent(v));
            } else {
                octets.extend_from_slice(v.as_bytes());
            }
        }
        let texte =
            if etendu_vu { octets_en_texte(&octets, &charset) } else { String::from_utf8_lossy(&octets).into() };
        params.push((base, texte));
    }
    (type_, params)
}

fn pourcent(v: &str) -> Vec<u8> {
    let b = v.as_bytes();
    let mut r = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && let (Some(h), Some(l)) = (b.get(i + 1).and_then(|&c| hexa(c)), b.get(i + 2).and_then(|&c| hexa(c)))
        {
            r.push(h * 16 + l);
            i += 3;
            continue;
        }
        r.push(b[i]);
        i += 1;
    }
    r
}

fn parametre<'a>(p: &'a [(String, String)], nom: &str) -> &'a str {
    p.iter().find(|(k, _)| k == nom).map(|(_, v)| v.as_str()).unwrap_or("")
}

/// Les parties d'un corps multipart (le préambule et l'épilogue passés ; une limite jamais fermée : la dernière
/// partie va jusqu'au bout).
fn decouper<'a>(corps: &'a [u8], limite: &str, max: usize) -> Vec<&'a [u8]> {
    let mut delim = b"--".to_vec();
    delim.extend_from_slice(limite.as_bytes());
    let mut parts = Vec::new();
    let mut debut: Option<usize> = None;
    let mut i = 0;
    while i <= corps.len() && parts.len() < max {
        let j = corps[i..].iter().position(|&c| c == b'\n').map(|p| i + p).unwrap_or(corps.len());
        let ligne = &corps[i..j];
        let ligne = ligne.strip_suffix(b"\r").unwrap_or(ligne);
        if let Some(reste) = ligne.strip_prefix(delim.as_slice()) {
            let fermeture = reste.starts_with(b"--");
            if fermeture || reste.iter().all(|c| c.is_ascii_whitespace()) {
                if let Some(d) = debut {
                    let mut f = i;
                    if f > d && corps[f - 1] == b'\n' {
                        f -= 1;
                        if f > d && corps[f - 1] == b'\r' {
                            f -= 1;
                        }
                    }
                    parts.push(&corps[d..f]);
                }
                if fermeture {
                    return parts;
                }
                debut = Some((j + 1).min(corps.len()));
            }
        }
        if j >= corps.len() {
            break;
        }
        i = j + 1;
    }
    if let Some(d) = debut
        && parts.len() < max
    {
        parts.push(&corps[d..]);
    }
    parts
}

/// Lit une partie (ou le mail entier) : multipart (descend), pièce jointe (comptée), text/plain, text/html.
fn partie(e: &Entetes, corps: &[u8], profondeur: usize, etat: &mut Etat) {
    if etat.parties >= PARTIES_MAX {
        return;
    }
    etat.parties += 1;
    let (mut type_, params) = type_et_parametres(valeur(e, "content-type"));
    if type_.is_empty() || !type_.contains('/') {
        type_ = "text/plain".into();
    }
    let (disposition, dparams) = type_et_parametres(valeur(e, "content-disposition"));
    if type_.starts_with("multipart/") {
        let limite = parametre(&params, "boundary");
        if !limite.is_empty() && limite.len() <= 200 {
            if profondeur >= PROFONDEUR_MAX {
                return;
            }
            for p in decouper(corps, limite, PARTIES_MAX) {
                if etat.parties >= PARTIES_MAX {
                    return;
                }
                let (t, c) = separer(p);
                partie(&entetes(t), c, profondeur + 1, etat);
            }
            return;
        }
        // (sans limite : lu comme du texte)
        type_ = "text/plain".into();
    }
    let nom = [parametre(&dparams, "filename"), parametre(&params, "name")].into_iter().any(|n| !n.trim().is_empty());
    if disposition == "attachment" || nom || type_ == "message/rfc822" {
        etat.pieces_jointes += 1;
        return;
    }
    let cte = valeur(e, "content-transfer-encoding");
    let charset = parametre(&params, "charset");
    if type_ == "text/plain" && etat.texte.is_none() {
        let t = propre_texte(&octets_en_texte(&decoder(cte, corps, BRUT_TEXTE_MAX), charset), TEXTE_MAX);
        if !t.is_empty() {
            etat.texte = Some(t);
        }
    } else if type_ == "text/html" && etat.html.is_none() {
        let mut h = octets_en_texte(&decoder(cte, corps, HTML_MAX), charset);
        couper_octets(&mut h, HTML_MAX);
        if !h.trim().is_empty() {
            etat.html = Some(h);
        }
    }
}

/// Coupe un texte à `max` octets au plus, sur une limite de caractère.
fn couper_octets(s: &mut String, max: usize) {
    if s.len() > max {
        let mut n = max;
        while !s.is_char_boundary(n) {
            n -= 1;
        }
        s.truncate(n);
    }
}

// ---------------------------------------------------------------- Texte au propre

fn invisible(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}' | '\u{00AD}' | '\u{034F}'
        | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{180E}')
}

fn tiret(c: char) -> bool {
    matches!(c, '\u{2010}'..='\u{2015}' | '\u{2212}' | '\u{FE58}' | '\u{FE63}' | '\u{FF0D}')
}

/// Un texte sur une ligne : contrôles et retours à la ligne devenus des espaces, espaces simplifiées, `max`
/// caractères au plus.
pub(crate) fn propre_ligne(s: &str, max: usize) -> String {
    let mut r = String::new();
    let mut n = 0;
    let mut espace = false;
    for c in s.chars() {
        if invisible(c) {
            continue;
        }
        if c.is_whitespace() || c.is_control() {
            espace = true;
            continue;
        }
        if espace && !r.is_empty() {
            if n >= max {
                break;
            }
            r.push(' ');
            n += 1;
        }
        espace = false;
        if n >= max {
            break;
        }
        r.push(if tiret(c) { '-' } else { c });
        n += 1;
    }
    r
}

/// Un texte de plusieurs lignes remis au propre : fins de ligne unifiées, contrôles et invisibles retirés, tirets
/// longs remplacés, espaces simplifiées, pas plus d'une ligne vide de suite, `max` caractères au plus.
pub(crate) fn propre_texte(s: &str, max: usize) -> String {
    let mut r = String::new();
    let mut n = 0usize;
    let mut ligne = String::new();
    let mut vides = 0usize;
    let mut fini = false;
    let pousser = |ligne: &mut String, r: &mut String, n: &mut usize, vides: &mut usize| -> bool {
        let l = ligne.trim();
        if l.is_empty() {
            *vides += 1;
        } else {
            if !r.is_empty() {
                let sauts = if *vides > 0 { 2 } else { 1 };
                for _ in 0..sauts {
                    r.push('\n');
                    *n += 1;
                }
            }
            for c in l.chars() {
                if *n >= max {
                    return true;
                }
                r.push(c);
                *n += 1;
            }
            *vides = 0;
        }
        ligne.clear();
        *n >= max
    };
    let mut prec_cr = false;
    let mut espace = false;
    for c in s.chars() {
        if c == '\n' && prec_cr {
            prec_cr = false;
            continue;
        }
        prec_cr = c == '\r';
        if c == '\n' || c == '\r' || c == '\u{2028}' || c == '\u{2029}' {
            espace = false;
            if pousser(&mut ligne, &mut r, &mut n, &mut vides) {
                fini = true;
                break;
            }
            continue;
        }
        if invisible(c) || (c.is_control() && c != '\t') {
            continue;
        }
        if c.is_whitespace() {
            espace = true;
            continue;
        }
        if espace && !ligne.is_empty() {
            ligne.push(' ');
        }
        espace = false;
        ligne.push(if tiret(c) { '-' } else { c });
        if ligne.len() > max * 4 + 16 {
            // (une ligne immense : elle suffit à remplir le texte)
            break;
        }
    }
    if !fini {
        pousser(&mut ligne, &mut r, &mut n, &mut vides);
    }
    r.trim_end().to_string()
}

// ---------------------------------------------------------------- HTML en texte

/// Les balises qui font un retour à la ligne.
const BLOCS: [&str; 22] = [
    "br",
    "p",
    "div",
    "li",
    "tr",
    "table",
    "ul",
    "ol",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "blockquote",
    "pre",
    "section",
    "article",
    "header",
    "footer",
    "dd",
];
/// Les balises retirées avec tout leur contenu (jusqu'à leur fermeture ; jamais fermées : tout le reste).
const CACHEES: [&str; 8] = ["script", "style", "noscript", "template", "svg", "iframe", "object", "math"];
/// Retirées avec leur contenu seulement si elles sont fermées.
const ENTETES_HTML: [&str; 2] = ["head", "title"];

fn chercher_ci(o: &[u8], depuis: usize, mot: &[u8]) -> Option<usize> {
    if depuis > o.len() || o.len() - depuis < mot.len() {
        return None;
    }
    (depuis..=o.len() - mot.len()).find(|&i| o[i..i + mot.len()].eq_ignore_ascii_case(mot))
}

/// La fin d'une balise (« > » hors guillemets), à partir de `i`.
fn fin_balise(b: &[u8], i: usize) -> Option<usize> {
    let mut q: Option<u8> = None;
    for (k, &c) in b.iter().enumerate().skip(i) {
        match q {
            Some(x) if c == x => q = None,
            Some(_) => {}
            None if c == b'"' || c == b'\'' => q = Some(c),
            None if c == b'>' => return Some(k),
            None => {}
        }
    }
    None
}

/// Le texte d'une page HTML (sans scripts, styles ni balises ; entités lues), avec ses retours à la ligne.
fn html_en_texte(html: &str) -> String {
    let limite = TEXTE_MAX * 8;
    let b = html.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let mut texte_debut = 0;
    while i < b.len() && out.len() < limite {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        texte_brut(&html[texte_debut..i], &mut out);
        let suiv = b.get(i + 1).copied().unwrap_or(0);
        if b[i..].starts_with(b"<!--") {
            i = chercher_ci(b, i + 4, b"-->").map(|p| p + 3).unwrap_or(b.len());
        } else if suiv == b'!' || suiv == b'?' {
            i = fin_balise(b, i).map(|p| p + 1).unwrap_or(b.len());
        } else {
            let fermante = suiv == b'/';
            let mut j = i + 1 + usize::from(fermante);
            let debut_nom = j;
            while j < b.len() && b[j].is_ascii_alphanumeric() && j - debut_nom < 20 {
                j += 1;
            }
            if j == debut_nom {
                // (« a < b » : un vrai signe)
                out.push('<');
                i += 1;
                texte_debut = i;
                continue;
            }
            let nom = html[debut_nom..j].to_ascii_lowercase();
            let Some(fin) = fin_balise(b, j) else {
                i = b.len();
                texte_debut = i;
                break;
            };
            let auto_fermee = fin > 0 && b[fin - 1] == b'/';
            i = fin + 1;
            if !fermante && !auto_fermee && (CACHEES.contains(&nom.as_str()) || ENTETES_HTML.contains(&nom.as_str())) {
                let mut fermeture = b"</".to_vec();
                fermeture.extend_from_slice(nom.as_bytes());
                match chercher_ci(b, i, &fermeture) {
                    Some(p) => i = fin_balise(b, p).map(|f| f + 1).unwrap_or(b.len()),
                    None if CACHEES.contains(&nom.as_str()) => i = b.len(),
                    None => {}
                }
            } else if nom == "br" || (BLOCS.contains(&nom.as_str()) && !out.trim_end_matches(' ').ends_with('\n')) {
                // (un bloc ouvert puis fermé ne fait qu'un retour à la ligne ; chaque <br> en fait un)
                out.push('\n');
            } else if nom == "td" || nom == "th" {
                out.push(' ');
            }
        }
        texte_debut = i.min(b.len());
    }
    if texte_debut < b.len() && out.len() < limite {
        texte_brut(&html[texte_debut..], &mut out);
    }
    out
}

/// Un morceau de texte HTML : les blancs deviennent des espaces, les entités sont lues.
fn texte_brut(s: &str, out: &mut String) {
    let mut reste = s;
    while let Some(p) = reste.find('&') {
        blancs(&reste[..p], out);
        let apres = &reste[p + 1..];
        let fin = apres.as_bytes().iter().take(32).position(|&c| c == b';');
        match fin.and_then(|f| entite(&apres[..f]).map(|c| (f, c))) {
            Some((f, c)) => {
                out.push(c);
                reste = &apres[f + 1..];
            }
            None => {
                out.push('&');
                reste = apres;
            }
        }
    }
    blancs(reste, out);
}

fn blancs(s: &str, out: &mut String) {
    out.extend(s.chars().map(|c| if c == '\n' || c == '\r' || c == '\t' { ' ' } else { c }));
}

/// Une entité HTML (sans & ni ;).
fn entite(nom: &str) -> Option<char> {
    if let Some(n) = nom.strip_prefix('#') {
        let v = match n.strip_prefix(['x', 'X']) {
            Some(h) => u32::from_str_radix(h, 16).ok()?,
            None => n.parse().ok()?,
        };
        return match v {
            0 => None,
            0x80..=0x9F => char::from_u32(CP1252[(v - 0x80) as usize]),
            _ => char::from_u32(v),
        };
    }
    Some(match nom {
        "amp" | "AMP" => '&',
        "lt" | "LT" => '<',
        "gt" | "GT" => '>',
        "quot" | "QUOT" => '"',
        "apos" => '\'',
        "nbsp" => '\u{A0}',
        "eacute" => '\u{E9}',
        "egrave" => '\u{E8}',
        "ecirc" => '\u{EA}',
        "euml" => '\u{EB}',
        "agrave" => '\u{E0}',
        "aacute" => '\u{E1}',
        "acirc" => '\u{E2}',
        "auml" => '\u{E4}',
        "ccedil" => '\u{E7}',
        "icirc" => '\u{EE}',
        "iuml" => '\u{EF}',
        "igrave" => '\u{EC}',
        "iacute" => '\u{ED}',
        "ocirc" => '\u{F4}',
        "ouml" => '\u{F6}',
        "oacute" => '\u{F3}',
        "ograve" => '\u{F2}',
        "ucirc" => '\u{FB}',
        "ugrave" => '\u{F9}',
        "uuml" => '\u{FC}',
        "uacute" => '\u{FA}',
        "yuml" => '\u{FF}',
        "ntilde" => '\u{F1}',
        "Eacute" => '\u{C9}',
        "Egrave" => '\u{C8}',
        "Ecirc" => '\u{CA}',
        "Agrave" => '\u{C0}',
        "Acirc" => '\u{C2}',
        "Ccedil" => '\u{C7}',
        "Ocirc" => '\u{D4}',
        "Icirc" => '\u{CE}',
        "Ucirc" => '\u{DB}',
        "oelig" => '\u{153}',
        "OElig" => '\u{152}',
        "aelig" => '\u{E6}',
        "szlig" => '\u{DF}',
        "rsquo" => '\u{2019}',
        "lsquo" => '\u{2018}',
        "sbquo" => '\u{201A}',
        "ldquo" => '\u{201C}',
        "rdquo" => '\u{201D}',
        "bdquo" => '\u{201E}',
        "laquo" => '\u{AB}',
        "raquo" => '\u{BB}',
        "hellip" => '\u{2026}',
        "mdash" | "ndash" | "minus" | "dash" | "hyphen" => '-',
        "euro" => '\u{20AC}',
        "copy" => '\u{A9}',
        "reg" => '\u{AE}',
        "trade" => '\u{2122}',
        "middot" => '\u{B7}',
        "bull" => '\u{2022}',
        "deg" => '\u{B0}',
        "times" => '\u{D7}',
        "divide" => '\u{F7}',
        "shy" | "zwnj" | "zwj" | "lrm" | "rlm" => '\u{200B}',
        "thinsp" | "ensp" | "emsp" => ' ',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Base64 pour fabriquer les mails d'essai (lignes de 76 caractères).
    fn en_base64(o: &[u8]) -> String {
        const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut r = String::new();
        for (i, c) in o.chunks(3).enumerate() {
            if i > 0 && i % 19 == 0 {
                r.push_str("\r\n");
            }
            let n = (u32::from(c[0]) << 16)
                | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
                | u32::from(*c.get(2).unwrap_or(&0));
            r.push(A[(n >> 18) as usize & 63] as char);
            r.push(A[(n >> 12) as usize & 63] as char);
            r.push(if c.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
            r.push(if c.len() > 2 { A[n as usize & 63] as char } else { '=' });
        }
        r
    }

    const LINKEDIN: &[u8] = b"Return-Path: <jobs-noreply@linkedin.com>\r\n\
From: LinkedIn Job Alerts <jobalerts-noreply@linkedin.com>\r\n\
To: Jordan Essai <jordan.essai@exemple.fr>\r\n\
Subject: =?UTF-8?Q?=C2=AB_Agent_de_quai_=C2=BB_:_30_nouvelles_offres?=\r\n\
Date: Wed, 19 Nov 2026 08:30:00 +0000 (UTC)\r\n\
Message-ID: <1234.abcd@linkedin.com>\r\n\
List-Unsubscribe: <https://www.linkedin.com/e/unsub>\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/alternative; \r\n\
\tboundary=\"----=_Part_123_456.789\"\r\n\
\r\n\
------=_Part_123_456.789\r\n\
Content-Type: text/plain;charset=UTF-8\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
Vos offres d=E2=80=99emploi\r\n\
Agent de quai =E2=80=93 Logistique Exemple\r\n\
Metz, Grand Est, France\r\n\
Voir l'offre : https://www.linkedin.com/comm/jobs/view/4012345678/?trk=3Deml=\r\n\
-jymbii\r\n\
------=_Part_123_456.789\r\n\
Content-Type: text/html;charset=UTF-8\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
<html><head><style>.a{color:red}</style></head><body><table><tr><td><a href=3D\"https://www.linkedin.com/comm/=\r\n\
jobs/view/4012345678/\">Agent de quai</a></td></tr></table></body></html>\r\n\
------=_Part_123_456.789--\r\n";

    #[test]
    fn une_alerte_linkedin() {
        let m = lire(7, LINKEDIN);
        assert_eq!(m.uid, 7);
        assert_eq!(m.de_adresse, "jobalerts-noreply@linkedin.com");
        assert_eq!(m.de_nom, "LinkedIn Job Alerts");
        assert_eq!(m.a, vec!["jordan.essai@exemple.fr"]);
        assert_eq!(m.objet, "\u{ab} Agent de quai \u{bb} : 30 nouvelles offres");
        assert_eq!(m.message_id, "1234.abcd@linkedin.com");
        assert!(m.masse);
        assert_eq!(m.date.as_ref().map(DateMail::iso).as_deref(), Some("2026-11-19T08:30:00"));
        assert!(
            m.texte.starts_with("Vos offres d\u{2019}emploi\nAgent de quai - Logistique Exemple\nMetz"),
            "{}",
            m.texte
        );
        assert!(m.texte.contains("view/4012345678/?trk=eml-jymbii"), "{}", m.texte);
        assert!(m.html.contains("href=\"https://www.linkedin.com/comm/jobs/view/4012345678/\""), "{}", m.html);
        assert_eq!(m.pieces_jointes, 0);
        assert!(!m.coupe);
    }

    #[test]
    fn une_alerte_indeed_en_base64() {
        let html = "<html><body><h1>Nouvelles offres</h1><p>Pr&eacute;parateur de commandes &#x2014; CDI</p>\
                    <div>Nancy (54)</div><script>alert(1)</script><a href=\"https://fr.indeed.com/rc/clk?jk=abc\">Voir</a>\
                    </body></html>";
        let brut = format!(
            "From: Indeed <alert@indeed.com>\r\nTo: jordan.essai@exemple.fr\r\nSubject: 3 nouveaux emplois\r\n\
             Precedence: bulk\r\nContent-Type: text/html; charset=\"utf-8\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n",
            en_base64(html.as_bytes())
        );
        let m = lire(1, brut.as_bytes());
        assert!(m.masse);
        assert_eq!(m.html, html);
        assert_eq!(m.texte, "Nouvelles offres\nPr\u{e9}parateur de commandes - CDI\nNancy (54)\nVoir");
    }

    #[test]
    fn un_recruteur_outlook_en_windows_1252_avec_un_pdf() {
        let pdf = en_base64(b"%PDF-1.4\n%\xc7\xec\x8f\xa2\n5 0 obj\n<</Length 6 0 R>>\nstream\n");
        let brut = format!(
            "From: \"Dupont, Marie\" <Marie.Dupont@Exemple-Recrutement.fr>\r\n\
Reply-To: RH@exemple-recrutement.fr\r\n\
To: jordan.essai@exemple.fr, \"Service RH\" <rh@exemple-recrutement.fr>\r\n\
Subject: =?Windows-1252?Q?Votre_candidature_=96_pr=E9parateur?=\r\n\
Date: Tue, 18 Nov 2026 17:05:12 +0100\r\n\
Message-ID: <AM0PR01MB1234.eurprd01.prod.outlook.com>\r\n\
Content-Type: multipart/mixed; boundary=\"_004_AM0PR_\"\r\n\
MIME-Version: 1.0\r\n\
\r\n\
--_004_AM0PR_\r\n\
Content-Type: multipart/alternative; boundary=\"_000_AM0PR_\"\r\n\
\r\n\
--_000_AM0PR_\r\n\
Content-Type: text/plain; charset=\"Windows-1252\"\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
Bonjour,\r\n\
\r\n\
\r\n\
Nous avons bien re=E7u votre candidature =96 merci =85 Pouvez-vous nous rappeler =\r\n\
au 03 00 00 00 00 ? =80 =93entretien=94\r\n\
\r\n\
Cordialement,\r\n\
Marie Dupont\r\n\
--_000_AM0PR_\r\n\
Content-Type: text/html; charset=\"Windows-1252\"\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
<html><body><p>Bonjour,</p><p>Nous avons bien re=E7u votre candidature</p></body></html>\r\n\
--_000_AM0PR_--\r\n\
\r\n\
--_004_AM0PR_\r\n\
Content-Type: application/pdf; name=\"Fiche de poste.pdf\"\r\n\
Content-Disposition: attachment; filename=\"Fiche de poste.pdf\"; size=1234\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
{pdf}\r\n\
--_004_AM0PR_--\r\n"
        );
        let m = lire(3, brut.as_bytes());
        assert_eq!(m.de_nom, "Dupont, Marie");
        assert_eq!(m.de_adresse, "marie.dupont@exemple-recrutement.fr");
        assert_eq!(m.repondre_a, "rh@exemple-recrutement.fr");
        assert_eq!(m.a, vec!["jordan.essai@exemple.fr", "rh@exemple-recrutement.fr"]);
        assert_eq!(m.objet, "Votre candidature - pr\u{e9}parateur");
        assert_eq!(
            m.texte,
            "Bonjour,\n\nNous avons bien re\u{e7}u votre candidature - merci \u{2026} Pouvez-vous nous rappeler au 03 00 \
             00 00 00 ? \u{20ac} \u{201c}entretien\u{201d}\n\nCordialement,\nMarie Dupont"
        );
        assert!(m.html.contains("re\u{e7}u"));
        assert_eq!(m.pieces_jointes, 1);
        assert!(!m.masse);
        assert_eq!(
            m.date,
            Some(DateMail { annee: 2026, mois: 11, jour: 18, heure: 17, minute: 5, seconde: 12, decalage_minutes: 60 })
        );
    }

    #[test]
    fn une_reponse_gmail_en_utf8() {
        let brut = "From: Paul Martin <paul.martin@exemple.com>\r\n\
To: jordan.essai@exemple.fr\r\n\
Subject: Re: Candidature au poste de cariste\r\n\
Date: 19 Nov 26 10:02 +0100\r\n\
Message-ID: <CAF=abc123@mail.gmail.com>\r\n\
In-Reply-To: <envoi-42@jobmind.local>\r\n\
References: <premier@jobmind.local>\r\n <envoi-42@jobmind.local>\r\n\
Content-Type: text/plain; charset=\"UTF-8\"\r\n\
Content-Transfer-Encoding: 8bit\r\n\
\r\n\
Bonjour Jordan,\r\n\
\r\n\
Merci pour votre candidature. \u{ca}tes-vous disponible jeudi \u{e0} 14 h ?\r\n\
\r\n\
\r\n\
\r\n\
Le mer. 19 nov. 2026 \u{e0} 08:30, Jordan a \u{e9}crit :\r\n\
> Bonjour,\r\n";
        let m = lire(9, brut.as_bytes());
        assert_eq!(m.message_id, "CAF=abc123@mail.gmail.com");
        assert_eq!(m.en_reponse_a, vec!["envoi-42@jobmind.local", "premier@jobmind.local"]);
        assert_eq!(m.date.as_ref().map(DateMail::iso).as_deref(), Some("2026-11-19T10:02:00"));
        assert_eq!(
            m.texte,
            "Bonjour Jordan,\n\nMerci pour votre candidature. \u{ca}tes-vous disponible jeudi \u{e0} 14 h ?\n\nLe mer. 19 \
             nov. 2026 \u{e0} 08:30, Jordan a \u{e9}crit :\n> Bonjour,"
        );
        assert_eq!(m.de_nom, "Paul Martin");
        assert!(m.html.is_empty());
    }

    #[test]
    fn les_mots_encodes() {
        let cas = [
            ("=?UTF-8?B?UsOpcG9uc2Ugw6Agdm90cmUgY2FuZGlkYXR1cmU=?=", "R\u{e9}ponse \u{e0} votre candidature"),
            ("=?iso-8859-1?Q?R=E9ponse?=", "R\u{e9}ponse"),
            ("=?UTF-8?B?Q2FuZGlk?==?UTF-8?B?YXR1cmU=?=", "Candidature"),
            ("=?UTF-8?Q?=C3?= =?UTF-8?Q?=A9t=C3=A9?=", "\u{e9}t\u{e9}"),
            ("Offre :  =?utf-8?q?caissi=C3=A8re?=  (CDD)", "Offre : caissi\u{e8}re (CDD)"),
            ("=?ISO-8859-15?Q?Prime_=A4_100?=", "Prime \u{20ac} 100"),
            ("=?windows-1252?Q?l=92offre?=", "l\u{2019}offre"),
            ("=?x-inconnu?Q?caf=C3=A9?=", "caf\u{e9}"),
            ("=?UTF-8*fr?Q?bient=C3=B4t?=", "bient\u{f4}t"),
            ("=?UTF-8?X?abc?= et =?pas fini", "=?UTF-8?X?abc?= et =?pas fini"),
            ("Un\r\n objet\tsur deux lignes", "Un objet sur deux lignes"),
        ];
        for (brut, attendu) in cas {
            assert_eq!(propre_ligne(&mots_encodes(brut), OBJET_MAX), attendu, "{brut}");
        }
    }

    #[test]
    fn les_adresses() {
        let cas: [(&str, Vec<(&str, &str)>); 8] = [
            ("\"Nom, Pr\u{e9}nom\" <X@Y.fr>", vec![("x@y.fr", "Nom, Pr\u{e9}nom")]),
            ("x@y.fr", vec![("x@y.fr", "")]),
            ("x@y.fr (Jean Dupont)", vec![("x@y.fr", "Jean Dupont")]),
            ("<x@y.fr>", vec![("x@y.fr", "")]),
            ("Amis: a@b.fr, \"C\" <c@d.fr>;, e@f.fr", vec![("a@b.fr", ""), ("c@d.fr", "C"), ("e@f.fr", "")]),
            ("Undisclosed recipients:;", vec![]),
            ("=?UTF-8?Q?Z=C3=A9lie_Martin?= <zelie@exemple.fr>", vec![("zelie@exemple.fr", "Z\u{e9}lie Martin")]),
            ("\"Le \\\"chef\\\" <faux@x.fr>\" <vrai@y.fr>", vec![("vrai@y.fr", "Le \"chef\" <faux@x.fr>")]),
        ];
        for (brut, attendu) in cas {
            let r = adresses(brut);
            let r: Vec<(&str, &str)> = r.iter().map(|(a, n)| (a.as_str(), n.as_str())).collect();
            assert_eq!(r, attendu, "{brut}");
        }
    }

    #[test]
    fn les_dates() {
        let cas = [
            ("Wed, 19 Nov 2026 08:30:00 +0100", Some((2026, 11, 19, 8, 30, 0, 60))),
            ("19 Nov 2026 08:30:00 -0500 (EST)", Some((2026, 11, 19, 8, 30, 0, -300))),
            ("Wed, 19 Nov 2026 08:30:00 +0100 (CET)", Some((2026, 11, 19, 8, 30, 0, 60))),
            ("Mon, 3 Feb 98 23:59:60 GMT", Some((1998, 2, 3, 23, 59, 60, 0))),
            ("Tue,  4 Mar 2025 7:05 CEST", Some((2025, 3, 4, 7, 5, 0, 120))),
            ("19 nov 2026 08:30:00", Some((2026, 11, 19, 8, 30, 0, 0))),
            ("hier soir", None),
            ("32 Nov 2026 08:30:00 +0100", None),
            ("19 Foo 2026 08:30:00 +0100", None),
            ("19 Nov 2026 25:30:00 +0100", None),
            ("", None),
        ];
        for (brut, attendu) in cas {
            let d =
                lire_date(brut).map(|d| (d.annee, d.mois, d.jour, d.heure, d.minute, d.seconde, d.decalage_minutes));
            assert_eq!(d, attendu, "{brut}");
        }
    }

    #[test]
    fn le_html_piege() {
        let brut = "From: x@y.fr\r\nContent-Type: text/html\r\n\r\n<!DOCTYPE html><html><head><title>T</title>\
<script>var a = '<b>'; document.write(\"alert\")</script><style>p { color: red }</style></head>\
<body onload=\"alert('x')\"><!-- commentaire cache --><p onclick=\"javascript:voler()\">Poste&nbsp;: cariste</p>\
<p>2 &lt; 3 &amp;&amp; caf&#233; &#x2014; fin&#8211;ici &unknown; R&D</p><STYLE>body{}</STYLE><Script >x()</SCRIPT >\
<br/><br><br><br>Fin<noscript>active</noscript><script>non ferme";
        let m = lire(1, brut.as_bytes());
        assert_eq!(
            m.texte,
            "Poste\u{a0}: cariste\n2 < 3 && caf\u{e9} - fin-ici &unknown; R&D\n\nFin".replace('\u{a0}', " ")
        );
        for interdit in
            ["alert", "color", "onclick", "javascript", "voler", "commentaire", "active", "non ferme", "x()"]
        {
            assert!(!m.texte.contains(interdit), "{interdit} dans {}", m.texte);
        }
    }

    #[test]
    fn un_mail_enorme_reste_borne_et_rapide() {
        let debut = std::time::Instant::now();
        let mut html = String::from("<html><body>");
        while html.len() < 7_500_000 {
            html.push_str("<p>Offre de cariste &agrave; Metz, CDI, temps plein.</p>\n");
        }
        let b64 = en_base64(html.as_bytes());
        assert!(b64.len() > 10_000_000);
        let brut = format!(
            "Subject: gros\r\nContent-Type: text/html; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{b64}"
        );
        let m = lire(1, brut.as_bytes());
        assert!(m.html.len() <= HTML_MAX && m.html.len() > HTML_MAX - 100);
        assert!(m.texte.chars().count() <= TEXTE_MAX && m.texte.starts_with("Offre de cariste \u{e0} Metz"));
        // (la même chose en pièce jointe : jamais décodée)
        let brut = format!(
            "Content-Type: multipart/mixed; boundary=b\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nVoir la pi\u{e8}ce.\r\n\
             --b\r\nContent-Type: application/zip; name=a.zip\r\nContent-Transfer-Encoding: base64\r\n\r\n{b64}\r\n--b--\r\n"
        );
        let m = lire(1, brut.as_bytes());
        assert_eq!((m.texte.as_str(), m.pieces_jointes), ("Voir la pi\u{e8}ce.", 1));
        // (et du texte brut immense)
        let brut = format!("Subject: x\r\n\r\n{}", "a".repeat(10_000_000));
        assert_eq!(lire(1, brut.as_bytes()).texte.len(), TEXTE_MAX);
        assert!(debut.elapsed() < std::time::Duration::from_secs(20), "{:?}", debut.elapsed());
    }

    #[test]
    fn des_mails_abimes() {
        // En-têtes cassés : sans « : », binaires, sans fin
        let m =
            lire(1, b"Ceci n'est pas un en-tete\r\nSubject: Bonjour\r\n\x00\xff\xfe: x\r\n\xc3(: y\r\n\r\nCorps\r\n");
        assert_eq!((m.objet.as_str(), m.texte.as_str()), ("Bonjour", "Corps"));
        let m = lire(1, b"Subject: sans fin\r\nFrom: a@b.fr");
        assert_eq!((m.objet.as_str(), m.de_adresse.as_str(), m.texte.as_str()), ("sans fin", "a@b.fr", ""));
        // Limite jamais fermée, préambule
        let m = lire(
            1,
            b"Content-Type: multipart/alternative; boundary=zz\r\n\r\nPreambule a passer\r\n--zz\r\nContent-Type: text/plain\r\n\r\nPartie un\r\n--zz\r\nContent-Type: text/html\r\n\r\n<p>deux</p>",
        );
        assert_eq!((m.texte.as_str(), m.html.as_str()), ("Partie un", "<p>deux</p>"));
        // Multipart sans limite : lu comme du texte
        assert_eq!(lire(1, b"Content-Type: multipart/mixed\r\n\r\nsimple").texte, "simple");
        // Base64 invalide, charset inconnu, octets nuls
        let m = lire(1, b"Content-Type: text/plain; charset=klingon\r\nContent-Transfer-Encoding: base64\r\n\r\n!!!Qm9u***am91cg==@@\r\n");
        assert_eq!(m.texte, "Bonjour");
        let m = lire(
            1,
            b"Subject: a\x00b\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nli\x00gne\x07 un\x1b[31m\r\nfin\xff",
        );
        assert_eq!(m.objet, "a b");
        assert_eq!(m.texte, "ligne un[31m\nfin\u{fffd}");
        // Sans en-têtes du tout, et le vide
        assert_eq!(lire(1, b"\r\nRien que le corps").texte, "Rien que le corps");
        assert_eq!(lire(1, b""), Mail { uid: 1, ..Default::default() });
        // Un objet de 10 000 caractères
        let brut = format!("Subject: {}\r\n\r\nx", "\u{e9}".repeat(10_000));
        assert_eq!(lire(1, brut.as_bytes()).objet.chars().count(), OBJET_MAX);
        // Plus de 200 en-têtes
        let mut brut = String::new();
        for i in 0..1000 {
            brut.push_str(&format!("X-H{i}: v\r\n"));
        }
        brut.push_str("Subject: trop tard\r\n\r\ncorps");
        let m = lire(1, brut.as_bytes());
        assert_eq!((m.objet.as_str(), m.texte.as_str()), ("", "corps"));
    }

    #[test]
    fn cinquante_niveaux_et_trop_de_parties() {
        let mut brut = String::new();
        for i in 0..50 {
            brut.push_str(&format!("Content-Type: multipart/mixed; boundary=\"n{i}\"\r\n\r\n--n{i}\r\n"));
        }
        brut.push_str("Content-Type: text/plain\r\n\r\nau fond\r\n");
        let m = lire(1, brut.as_bytes());
        assert_eq!(m.texte, "");
        // (dix niveaux : lu)
        let mut brut = String::new();
        for i in 0..10 {
            brut.push_str(&format!("Content-Type: multipart/related; boundary=\"n{i}\"\r\n\r\n--n{i}\r\n"));
        }
        brut.push_str("Content-Type: text/plain\r\n\r\nau fond\r\n");
        assert_eq!(lire(1, brut.as_bytes()).texte, "au fond");
        // 1000 pièces jointes : 200 parties lues au plus
        let mut brut = String::from("Content-Type: multipart/mixed; boundary=b\r\n\r\n");
        for i in 0..1000 {
            brut.push_str(&format!("--b\r\nContent-Disposition: attachment; filename=f{i}.txt\r\n\r\nx\r\n"));
        }
        brut.push_str("--b--\r\n");
        let m = lire(1, brut.as_bytes());
        assert!(m.pieces_jointes > 100 && m.pieces_jointes < PARTIES_MAX, "{}", m.pieces_jointes);
    }

    #[test]
    fn des_formes_variees() {
        // Majuscules partout, related avec une image intégrée sans nom (pas une pièce jointe), text/plain vide : le
        // texte vient du HTML
        let brut = b"FROM: =?utf-8?b?V2VsY29tZSB0byB0aGUgSnVuZ2xl?= <Alertes@Welcometothejungle.com>\n\
SUBJECT: Nouvelles offres\n\
PRECEDENCE: List\n\
CONTENT-TYPE: MULTIPART/RELATED; BOUNDARY=Rel\n\n\
--Rel\n\
Content-Type: multipart/alternative; boundary=Alt\n\n\
--Alt\n\
Content-Type: TEXT/PLAIN; CHARSET=UTF-8\n\n\
  \n\
--Alt\n\
Content-Type: text/html; charset=UTF-8\n\
Content-Transfer-Encoding: QUOTED-PRINTABLE\n\n\
<div>Vendeur&#160;<b>H/F</b> <i>Metz</i></div><img src=3D\"cid:logo\" alt=3D\"logo\"><ul><li>CDI</li><li>Temps plein</li></ul>\n\
--Alt--\n\
--Rel\n\
Content-Type: image/png\n\
Content-ID: <logo>\n\
Content-Transfer-Encoding: base64\n\n\
iVBORw0KGgo=\n\
--Rel--\n";
        let m = lire(2, brut);
        assert_eq!(m.de_nom, "Welcome to the Jungle");
        assert_eq!(m.de_adresse, "alertes@welcometothejungle.com");
        assert_eq!(m.objet, "Nouvelles offres");
        assert!(m.masse);
        assert_eq!(m.pieces_jointes, 0);
        assert_eq!(m.texte, "Vendeur H/F Metz\nCDI\nTemps plein");
        // format=flowed (Apple Mail, Thunderbird) : lu tel quel, lignes gardées
        let m = lire(
            3,
            b"Content-Type: text/plain; charset=utf-8; format=flowed\r\n\r\nUne phrase coup\xc3\xa9e \r\nen deux.\r\n",
        );
        assert_eq!(m.texte, "Une phrase coup\u{e9}e\nen deux.");
        // Adresses rares : nom vide entre guillemets, sans adresse, adresse cassée ; Message-ID sans chevrons
        assert_eq!(adresses("\"\" <A@B.FR>"), vec![("a@b.fr".to_string(), String::new())]);
        assert!(adresses("Jean Dupont").is_empty());
        assert!(adresses("<pas-une-adresse>").is_empty());
        assert_eq!(ids("abc@x.fr"), vec!["abc@x.fr"]);
        // 30 références : In-Reply-To d'abord, puis les dernières citées
        let refs: Vec<String> = (0..30).map(|i| format!("<r{i}@x>")).collect();
        let r = references("<r29@x>", &refs.join(" "));
        assert_eq!(r.len(), 20);
        assert_eq!((r[0].as_str(), r[1].as_str(), r[19].as_str()), ("r29@x", "r10@x", "r28@x"));
    }

    #[test]
    fn les_parametres_rfc_2231() {
        let (t, p) = type_et_parametres("application/pdf; name*=utf-8''%C3%A9t%C3%A9.pdf; x=\"a;b\"");
        assert_eq!(t, "application/pdf");
        assert_eq!(parametre(&p, "name"), "\u{e9}t\u{e9}.pdf");
        assert_eq!(parametre(&p, "x"), "a;b");
        let (_, p) = type_et_parametres("attachment; filename*0=\"lettre_\"; filename*1=\"de motivation.docx\"");
        assert_eq!(parametre(&p, "filename"), "lettre_de motivation.docx");
        let (_, p) = type_et_parametres("text/plain; CHARSET=ISO-8859-1");
        assert_eq!(parametre(&p, "charset"), "ISO-8859-1");
        let m = lire(
            1,
            b"Content-Type: text/plain; charset=iso-8859-1\r\nContent-Transfer-Encoding: 8bit\r\n\r\nD\xe9j\xe0 vu",
        );
        assert_eq!(m.texte, "D\u{e9}j\u{e0} vu");
        let m = lire(1, b"Content-Type: text/plain\r\n\r\nSans charset : \xe9t\xe9");
        assert_eq!(m.texte, "Sans charset : \u{e9}t\u{e9}");
    }

    #[test]
    fn des_octets_au_hasard_sans_panique() {
        let mut graine: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut hasard = move || {
            graine ^= graine << 13;
            graine ^= graine >> 7;
            graine ^= graine << 17;
            graine
        };
        let morceaux: [&[u8]; 16] = [
            b"Content-Type: multipart/mixed; boundary=x\r\n",
            b"\r\n",
            b"--x\r\n",
            b"--x--",
            b"=?UTF-8?B?",
            b"?=",
            b"=?iso-8859-1?Q?=E9",
            b"Content-Transfer-Encoding: base64\r\n",
            b"Content-Transfer-Encoding: quoted-printable\r\n",
            b"Content-Type: text/html\r\n",
            b"<script>",
            b"&#x",
            b"<p",
            b"=\r\n",
            b"From: \"a, <b@c>\r\n",
            b"Date: 1 Nov ",
        ];
        for _ in 0..3000 {
            let mut o = Vec::new();
            for _ in 0..(hasard() % 40) {
                if hasard() % 3 == 0 {
                    for _ in 0..(hasard() % 8) {
                        o.push((hasard() & 0xff) as u8);
                    }
                } else {
                    o.extend_from_slice(morceaux[(hasard() % 16) as usize]);
                }
            }
            let m = lire(1, &o);
            assert!(m.texte.chars().count() <= TEXTE_MAX && m.objet.chars().count() <= OBJET_MAX);
        }
        // et les vrais mails abîmés au hasard
        for _ in 0..2000 {
            let mut o = LINKEDIN.to_vec();
            for _ in 0..(1 + hasard() % 10) {
                let i = (hasard() as usize) % o.len();
                o[i] = (hasard() & 0xff) as u8;
            }
            let fin = (hasard() as usize) % (o.len() + 1);
            let _ = lire(1, &o[..fin]);
        }
    }
}
