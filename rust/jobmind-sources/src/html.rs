//! Une page HTML lue sans navigateur (étape 5, « Ajouter à Jobmind ») : la page que Jordan regarde dans le panneau des
//! sites arrive ici en texte (son HTML, lu par l'appli au moment du clic), et n'est jamais exécutée ni réaffichée en
//! HTML. Ce module la découpe en balises et en textes, puis répond à de petites questions : le titre de la page, une
//! balise `<meta>`, les scripts de données (JSON-LD), le premier élément qui répond à un petit sélecteur, son texte.
//!
//! Le contenu est une donnée non fiable : le HTML est coupé à [`HTML_MAX`] octets, le nombre de balises à
//! [`JETONS_MAX`], chaque attribut et chaque nom sont bornés, et rien ne fait paniquer la lecture (une page abîmée donne
//! seulement moins de choses). Les textes sont rendus sans balises, entités lues.
//!
//! Le petit sélecteur : `balise`, `.classe`, `#id`, `[attribut]`, `[attribut="valeur"]`, `[attribut^="début"]`,
//! `[attribut*="morceau"]`, combinés (`h1.titre[data-x="1"]`), et un descendant par espace (`.carte .nom`).

use crate::texte;

/// Le HTML lu au plus (en octets) ; la suite est laissée.
pub const HTML_MAX: usize = 4_000_000;
/// Le nombre de balises et de textes gardés au plus.
pub const JETONS_MAX: usize = 400_000;
/// Les attributs gardés par balise, au plus.
const ATTRS_MAX: usize = 48;
/// La profondeur cherchée pour fermer une balise (une page qui ne ferme jamais ses balises reste rapide à lire).
const PILE_RECHERCHE: usize = 64;

/// Un morceau de la page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Jeton {
    /// Une balise ouvrante (son nom en minuscules, ses attributs, entités lues)
    Ouvre { nom: String, attrs: Vec<(String, String)> },
    /// Une balise fermante
    Ferme(String),
    /// Du texte (entités lues), ou le contenu d'un script ou d'un style tel quel
    Texte(String),
}

/// Les balises sans contenu.
fn vide(nom: &str) -> bool {
    matches!(
        nom,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Les balises dont le contenu est du texte brut, lu jusqu'à leur balise fermante.
fn brute(nom: &str) -> bool {
    matches!(nom, "script" | "style" | "xmp" | "iframe" | "noembed" | "noframes" | "noscript" | "plaintext")
}

/// Les balises dont le contenu est du texte (entités lues), lu jusqu'à leur balise fermante.
fn texte_seul(nom: &str) -> bool {
    matches!(nom, "title" | "textarea")
}

/// Les balises qui font une ligne à part dans le texte.
fn bloc(nom: &str) -> bool {
    matches!(
        nom,
        "p" | "div"
            | "section"
            | "article"
            | "main"
            | "ul"
            | "ol"
            | "li"
            | "dl"
            | "dt"
            | "dd"
            | "tr"
            | "table"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "footer"
            | "aside"
            | "nav"
            | "blockquote"
            | "pre"
            | "br"
            | "hr"
            | "form"
            | "fieldset"
            | "figure"
            | "address"
    )
}

/// Les balises dont le texte n'est jamais lu (ce qui ne se voit pas, ou n'est pas du texte).
fn muette(nom: &str) -> bool {
    matches!(
        nom,
        "script"
            | "style"
            | "noscript"
            | "template"
            | "svg"
            | "math"
            | "iframe"
            | "object"
            | "canvas"
            | "video"
            | "audio"
            | "head"
            | "select"
            | "option"
            | "textarea"
            | "button"
    )
}

/// Les balises de cadre d'un site (menu, en-tête, pied de page) : laissées pour le texte principal d'une page.
fn cadre(nom: &str) -> bool {
    matches!(nom, "nav" | "header" | "footer" | "aside" | "form" | "dialog")
}

/// Coupe un texte à `max` octets au plus, sur une limite de caractère.
fn coupe_octets(t: &str, max: usize) -> &str {
    if t.len() <= max {
        return t;
    }
    let mut n = max;
    while !t.is_char_boundary(n) {
        n -= 1;
    }
    &t[..n]
}

/// La position de `aiguille` (en ASCII, sans tenir compte des majuscules) dans `t` à partir de `debut`.
fn chercher_sans_casse(t: &[u8], debut: usize, aiguille: &[u8]) -> Option<usize> {
    if aiguille.is_empty() || debut >= t.len() {
        return None;
    }
    t[debut..].windows(aiguille.len()).position(|w| w.eq_ignore_ascii_case(aiguille)).map(|p| p + debut)
}

/// La page découpée : ses morceaux, et pour chaque balise ouvrante l'indice qui suit sa fin.
#[derive(Debug, Clone, Default)]
pub struct Document {
    jetons: Vec<Jeton>,
    fins: Vec<usize>,
}

impl Document {
    /// Découpe une page (coupée à [`HTML_MAX`] octets).
    pub fn lire(html: &str) -> Document {
        let jetons = decouper(coupe_octets(html, HTML_MAX));
        let fins = fins(&jetons);
        Document { jetons, fins }
    }

    /// Le nombre de morceaux.
    pub fn len(&self) -> usize {
        self.jetons.len()
    }

    /// Aucun morceau.
    pub fn is_empty(&self) -> bool {
        self.jetons.is_empty()
    }

    /// Le nom de la balise ouvrante `i`.
    pub fn nom(&self, i: usize) -> Option<&str> {
        match self.jetons.get(i) {
            Some(Jeton::Ouvre { nom, .. }) => Some(nom),
            _ => None,
        }
    }

    /// Un attribut de la balise ouvrante `i`.
    pub fn attr(&self, i: usize, nom: &str) -> Option<&str> {
        match self.jetons.get(i) {
            Some(Jeton::Ouvre { attrs, .. }) => attrs.iter().find(|(n, _)| n == nom).map(|(_, v)| v.as_str()),
            _ => None,
        }
    }

    /// Ce qui suit la fin de l'élément `i` (lui-même et son contenu : `i..fin`).
    pub fn fin(&self, i: usize) -> usize {
        self.fins.get(i).copied().unwrap_or(i + 1).max(i + 1)
    }

    /// Le titre de la page (`<title>`), sur une ligne.
    pub fn titre(&self) -> String {
        self.trouver("title").map(|i| self.texte(i)).unwrap_or_default()
    }

    /// Une balise `<meta>` par son `name` ou sa `property` (« og:title ») : son `content`, sur une ligne.
    pub fn meta(&self, nom: &str) -> Option<String> {
        (0..self.jetons.len())
            .filter(|i| self.nom(*i) == Some("meta"))
            .find(|i| {
                self.attr(*i, "property").is_some_and(|p| p.eq_ignore_ascii_case(nom))
                    || self.attr(*i, "name").is_some_and(|p| p.eq_ignore_ascii_case(nom))
            })
            .and_then(|i| self.attr(i, "content"))
            .map(texte::ligne)
            .filter(|t| !t.is_empty())
    }

    /// Le contenu brut des scripts de ce type (« application/ld+json »), `max` octets au plus chacun, `combien` au plus.
    pub fn scripts(&self, type_: &str, max: usize, combien: usize) -> Vec<String> {
        let mut l = Vec::new();
        for i in 0..self.jetons.len() {
            if l.len() >= combien {
                break;
            }
            if self.nom(i) != Some("script")
                || !self.attr(i, "type").is_some_and(|t| t.trim().eq_ignore_ascii_case(type_))
            {
                continue;
            }
            if let Some(Jeton::Texte(t)) = self.jetons.get(i + 1)
                && t.len() <= max
            {
                l.push(t.clone());
            }
        }
        l
    }

    /// Le premier élément qui répond au sélecteur, dans toute la page.
    pub fn trouver(&self, selecteur: &str) -> Option<usize> {
        self.trouver_entre(selecteur, 0, self.jetons.len())
    }

    /// Le premier élément qui répond au sélecteur, entre `debut` et `fin` (les descendants de l'élément `debut - 1`).
    pub fn trouver_entre(&self, selecteur: &str, debut: usize, fin: usize) -> Option<usize> {
        let chaine = Selecteur::lire(selecteur)?;
        self.chercher(&chaine, debut, fin.min(self.jetons.len()))
    }

    fn chercher(&self, chaine: &[Simple], debut: usize, fin: usize) -> Option<usize> {
        let (premier, reste) = chaine.split_first()?;
        let mut i = debut;
        while i < fin {
            if let Some(Jeton::Ouvre { nom, attrs }) = self.jetons.get(i)
                && premier.accepte(nom, attrs)
            {
                if reste.is_empty() {
                    return Some(i);
                }
                if let Some(x) = self.chercher(reste, i + 1, self.fin(i).min(fin)) {
                    return Some(x);
                }
            }
            i += 1;
        }
        None
    }

    /// Les éléments qui répondent au sélecteur (au plus `combien`).
    pub fn tous(&self, selecteur: &str, combien: usize) -> Vec<usize> {
        let mut l = Vec::new();
        let Some(chaine) = Selecteur::lire(selecteur) else { return l };
        self.chercher_tous(&chaine, 0, self.jetons.len(), combien, &mut l);
        l.sort_unstable();
        l.dedup();
        l
    }

    /// Tous les éléments de la chaîne entre `debut` et `fin` (étape 5 bis : « ul.liste li » rend chaque `li` de la
    /// liste, pas seulement le premier). Un ancêtre fouillé n'est pas refouillé par ses propres descendants.
    fn chercher_tous(&self, chaine: &[Simple], debut: usize, fin: usize, combien: usize, l: &mut Vec<usize>) {
        let Some((premier, reste)) = chaine.split_first() else { return };
        let mut i = debut;
        while i < fin && l.len() < combien {
            if let Some(Jeton::Ouvre { nom, attrs }) = self.jetons.get(i)
                && premier.accepte(nom, attrs)
            {
                if reste.is_empty() {
                    l.push(i);
                } else {
                    let f = self.fin(i).min(fin);
                    self.chercher_tous(reste, i + 1, f, combien, l);
                    i = f.max(i + 1);
                    continue;
                }
            }
            i += 1;
        }
    }

    /// Le texte de l'élément `i` sur une ligne (titre, entreprise, lieu) : sans balise, coupé à `texte::LIGNE_MAX`.
    pub fn texte(&self, i: usize) -> String {
        let t = self.texte_brut(i, self.fin(i), false, false, Some(i));
        texte::ligne(&texte::lignes_propres(&t, texte::LIGNE_MAX * 4))
    }

    /// Le texte de l'élément `i` avec ses lignes (le texte d'une annonce), coupé à `max` caractères.
    pub fn texte_lignes(&self, i: usize, max: usize) -> String {
        let t = self.texte_brut(i, self.fin(i), true, false, Some(i));
        texte::lignes_propres(&t, max)
    }

    /// Le texte principal de la page : `<main>`, `<article>` ou `[role="main"]`, sinon `<body>`, sans le cadre du site
    /// (menus, en-tête, pied de page, formulaires).
    pub fn texte_principal(&self, max: usize) -> String {
        let i = ["main", "[role=\"main\"]", "article", "body"].iter().find_map(|s| self.trouver(s));
        let (debut, fin) = match i {
            Some(i) => (i, self.fin(i)),
            None => (0, self.jetons.len()),
        };
        texte::lignes_propres(&self.texte_brut(debut, fin, true, true, Some(debut)), max)
    }

    /// Le texte de la page entière, sauf la plage `sauf` (un texte d'annonce à ne pas relire), coupé à `max`.
    pub fn texte_page_sauf(&self, sauf: Option<(usize, usize)>, max: usize) -> String {
        let (debut, fin) = match self.trouver("body") {
            Some(b) => (b, self.fin(b)),
            None => (0, self.jetons.len()),
        };
        let mut t = String::new();
        match sauf {
            Some((a, b)) if a >= debut && b <= fin => {
                t.push_str(&self.texte_brut(debut, a, true, false, Some(debut)));
                t.push('\n');
                t.push_str(&self.texte_brut(b, fin, true, false, None));
            }
            _ => t.push_str(&self.texte_brut(debut, fin, true, false, Some(debut))),
        }
        texte::lignes_propres(&t, max)
    }

    /// Le texte des morceaux `debut..fin`, sans les balises muettes (ni, si `sans_cadre`, le cadre du site), avec ou
    /// sans retours à la ligne. Ce qui est caché à tous (`hidden`, `aria-hidden` sur un bloc, « visually-hidden » :
    /// lu seulement par les lecteurs d'écran, en double) est laissé.
    fn texte_brut(&self, debut: usize, fin: usize, lignes: bool, sans_cadre: bool, depart: Option<usize>) -> String {
        let fin = fin.min(self.jetons.len());
        let mut t = String::new();
        let mut i = debut;
        while i < fin {
            match &self.jetons[i] {
                Jeton::Ouvre { nom, attrs } => {
                    let cache = attrs.iter().any(|(n, v)| {
                        n == "hidden"
                            || (n == "class"
                                && v.split_whitespace().any(|c| {
                                    matches!(c, "visually-hidden" | "sr-only" | "screen-reader-text" | "a11y-text")
                                }))
                            || (n == "style" && v.replace(' ', "").to_ascii_lowercase().contains("display:none"))
                    });
                    // (l'élément de départ lui-même n'est jamais laissé)
                    if Some(i) != depart && (muette(nom) || cache || (sans_cadre && cadre(nom))) {
                        i = self.fin(i);
                        continue;
                    }
                    if nom == "br" {
                        t.push(if lignes { '\n' } else { ' ' });
                    } else if nom == "li" && lignes {
                        t.push_str("\n- ");
                    } else if bloc(nom) || nom == "td" || nom == "th" {
                        t.push(if lignes && bloc(nom) { '\n' } else { ' ' });
                    }
                }
                Jeton::Ferme(nom) => {
                    if bloc(nom) {
                        t.push(if lignes { '\n' } else { ' ' });
                    } else if nom == "td" || nom == "th" {
                        t.push(' ');
                    }
                }
                Jeton::Texte(x) => t.push_str(x),
            }
            i += 1;
        }
        t
    }
}

/// Découpe le HTML en morceaux.
fn decouper(h: &str) -> Vec<Jeton> {
    let b = h.as_bytes();
    let mut j: Vec<Jeton> = Vec::new();
    let mut i = 0usize;
    let mut texte_debut = 0usize;
    let pousser_texte = |j: &mut Vec<Jeton>, de: usize, a: usize| {
        if a > de {
            let t = &h[de..a];
            if !t.trim().is_empty() || t.contains(' ') {
                j.push(Jeton::Texte(texte::entites(t)));
            }
        }
    };
    while i < b.len() && j.len() < JETONS_MAX {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let suivant = b.get(i + 1).copied().unwrap_or(0);
        // Un commentaire
        if b[i..].starts_with(b"<!--") {
            pousser_texte(&mut j, texte_debut, i);
            i = match chercher_sans_casse(b, i + 4, b"-->") {
                Some(p) => p + 3,
                None => b.len(),
            };
            texte_debut = i;
            continue;
        }
        // <!doctype>, <?xml ?>, <![CDATA[ ]]>
        if suivant == b'!' || suivant == b'?' {
            pousser_texte(&mut j, texte_debut, i);
            i = b[i..].iter().position(|c| *c == b'>').map_or(b.len(), |p| i + p + 1);
            texte_debut = i;
            continue;
        }
        // Une balise fermante
        if suivant == b'/' && b.get(i + 2).is_some_and(u8::is_ascii_alphabetic) {
            pousser_texte(&mut j, texte_debut, i);
            let mut k = i + 2;
            while k < b.len() && (b[k].is_ascii_alphanumeric() || b[k] == b'-' || b[k] == b':') && k - i < 64 {
                k += 1;
            }
            let nom = h[i + 2..k].to_ascii_lowercase();
            i = b[k..].iter().position(|c| *c == b'>').map_or(b.len(), |p| k + p + 1);
            texte_debut = i;
            j.push(Jeton::Ferme(nom));
            continue;
        }
        // Une balise ouvrante
        if suivant.is_ascii_alphabetic() {
            pousser_texte(&mut j, texte_debut, i);
            let (nom, attrs, apres) = balise(h, i + 1);
            i = apres;
            texte_debut = i;
            let raw = brute(&nom);
            let rcdata = texte_seul(&nom);
            j.push(Jeton::Ouvre { nom: nom.clone(), attrs });
            if raw || rcdata {
                // (le contenu jusqu'à la balise fermante de même nom, tel quel)
                let fermante = format!("</{nom}");
                let fin = chercher_sans_casse(b, i, fermante.as_bytes()).unwrap_or(b.len());
                let contenu = &h[i..fin];
                if !contenu.is_empty() {
                    j.push(Jeton::Texte(if rcdata { texte::entites(contenu) } else { contenu.to_string() }));
                }
                j.push(Jeton::Ferme(nom));
                i = b[fin..].iter().position(|c| *c == b'>').map_or(b.len(), |p| fin + p + 1);
                texte_debut = i;
            }
            continue;
        }
        // Un « < » qui n'ouvre rien : du texte
        i += 1;
    }
    if j.len() < JETONS_MAX {
        pousser_texte(&mut j, texte_debut, b.len());
    }
    j
}

/// Une balise ouvrante à partir de son nom (`debut` : juste après « < ») : son nom, ses attributs et ce qui suit « > ».
fn balise(h: &str, debut: usize) -> (String, Vec<(String, String)>, usize) {
    let b = h.as_bytes();
    let mut k = debut;
    while k < b.len() && !b[k].is_ascii_whitespace() && b[k] != b'>' && b[k] != b'/' && k - debut < 64 {
        k += 1;
    }
    let nom = h[debut..k].to_ascii_lowercase();
    let mut attrs: Vec<(String, String)> = Vec::new();
    loop {
        while k < b.len() && (b[k].is_ascii_whitespace() || b[k] == b'/') {
            k += 1;
        }
        if k >= b.len() {
            return (nom, attrs, b.len());
        }
        if b[k] == b'>' {
            return (nom, attrs, k + 1);
        }
        // Le nom de l'attribut
        let n0 = k;
        while k < b.len() && !b[k].is_ascii_whitespace() && !matches!(b[k], b'=' | b'>' | b'/') {
            k += 1;
        }
        let n = h[n0..k].to_ascii_lowercase();
        while k < b.len() && b[k].is_ascii_whitespace() {
            k += 1;
        }
        let mut valeur = String::new();
        if k < b.len() && b[k] == b'=' {
            k += 1;
            while k < b.len() && b[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < b.len() && (b[k] == b'"' || b[k] == b'\'') {
                let q = b[k];
                let v0 = k + 1;
                let v1 = b[v0..].iter().position(|c| *c == q).map_or(b.len(), |p| v0 + p);
                valeur = h[v0..v1].to_string();
                k = (v1 + 1).min(b.len());
            } else {
                let v0 = k;
                while k < b.len() && !b[k].is_ascii_whitespace() && b[k] != b'>' {
                    k += 1;
                }
                valeur = h[v0..k].to_string();
            }
        }
        if !n.is_empty() && n.len() <= 64 && attrs.len() < ATTRS_MAX && !attrs.iter().any(|(x, _)| *x == n) {
            attrs.push((n, texte::entites(coupe_octets(&valeur, 100_000))));
        }
    }
}

/// Pour chaque balise ouvrante, l'indice qui suit sa fin (sa balise fermante, ou la fin d'un parent qui la ferme).
fn fins(j: &[Jeton]) -> Vec<usize> {
    let mut fins: Vec<usize> = (0..j.len()).map(|i| i + 1).collect();
    let mut pile: Vec<(usize, &str)> = Vec::new();
    for (i, x) in j.iter().enumerate() {
        match x {
            Jeton::Ouvre { nom, .. } => {
                if !vide(nom) {
                    pile.push((i, nom));
                }
            }
            Jeton::Ferme(nom) => {
                let bas = pile.len().saturating_sub(PILE_RECHERCHE);
                if let Some(p) = (bas..pile.len()).rev().find(|p| pile[*p].1 == nom) {
                    for (o, _) in pile.drain(p..) {
                        fins[o] = i + 1;
                    }
                }
            }
            Jeton::Texte(_) => {}
        }
    }
    for (o, _) in pile {
        fins[o] = j.len();
    }
    fins
}

/// Une condition sur un attribut.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Condition {
    Existe,
    Egal(String),
    Debut(String),
    Contient(String),
}

/// Un sélecteur simple : une balise, des classes, un id, des attributs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Simple {
    balise: Option<String>,
    classes: Vec<String>,
    id: Option<String>,
    attrs: Vec<(String, Condition)>,
}

impl Simple {
    fn accepte(&self, nom: &str, attrs: &[(String, String)]) -> bool {
        let a = |n: &str| attrs.iter().find(|(x, _)| x == n).map(|(_, v)| v.as_str());
        if self.balise.as_deref().is_some_and(|b| b != nom) {
            return false;
        }
        if let Some(id) = &self.id
            && a("id") != Some(id.as_str())
        {
            return false;
        }
        if !self.classes.is_empty() {
            let c = a("class").unwrap_or_default();
            if !self.classes.iter().all(|x| c.split_whitespace().any(|y| y == x)) {
                return false;
            }
        }
        self.attrs.iter().all(|(n, cond)| match (a(n), cond) {
            (None, _) => false,
            (Some(_), Condition::Existe) => true,
            (Some(v), Condition::Egal(x)) => v == x,
            (Some(v), Condition::Debut(x)) => v.starts_with(x.as_str()),
            (Some(v), Condition::Contient(x)) => v.contains(x.as_str()),
        })
    }
}

/// Un sélecteur : des sélecteurs simples, chacun descendant du précédent.
struct Selecteur;

impl Selecteur {
    /// Lit un sélecteur écrit dans le code (jamais venu de la page) ; rien s'il est mal écrit.
    fn lire(s: &str) -> Option<Vec<Simple>> {
        let mut l = Vec::new();
        for morceau in decouper_selecteur(s) {
            l.push(lire_simple(&morceau)?);
        }
        (!l.is_empty()).then_some(l)
    }
}

/// Coupe un sélecteur aux espaces qui ne sont pas entre crochets.
fn decouper_selecteur(s: &str) -> Vec<String> {
    let mut l = Vec::new();
    let mut m = String::new();
    let mut crochets = false;
    for c in s.chars() {
        match c {
            '[' => crochets = true,
            ']' => crochets = false,
            _ => {}
        }
        if c.is_whitespace() && !crochets {
            if !m.is_empty() {
                l.push(std::mem::take(&mut m));
            }
        } else {
            m.push(c);
        }
    }
    if !m.is_empty() {
        l.push(m);
    }
    l
}

fn lire_simple(s: &str) -> Option<Simple> {
    let mut x = Simple::default();
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mot = |i: &mut usize| -> String {
        let d = *i;
        while *i < c.len() && (c[*i].is_alphanumeric() || matches!(c[*i], '-' | '_')) {
            *i += 1;
        }
        c[d..*i].iter().collect()
    };
    while i < c.len() {
        match c[i] {
            '.' => {
                i += 1;
                x.classes.push(mot(&mut i));
            }
            '#' => {
                i += 1;
                x.id = Some(mot(&mut i));
            }
            '[' => {
                let fin = c[i..].iter().position(|y| *y == ']')? + i;
                let dedans: String = c[i + 1..fin].iter().collect();
                i = fin + 1;
                let (n, cond) = match dedans.split_once('=') {
                    None => (dedans.trim().to_ascii_lowercase(), Condition::Existe),
                    Some((n, v)) => {
                        let v = v.trim().trim_matches('"').to_string();
                        if let Some(n) = n.strip_suffix('^') {
                            (n.trim().to_ascii_lowercase(), Condition::Debut(v))
                        } else if let Some(n) = n.strip_suffix('*') {
                            (n.trim().to_ascii_lowercase(), Condition::Contient(v))
                        } else {
                            (n.trim().to_ascii_lowercase(), Condition::Egal(v))
                        }
                    }
                };
                x.attrs.push((n, cond));
            }
            y if y.is_alphanumeric() => {
                x.balise = Some(mot(&mut i).to_ascii_lowercase());
            }
            _ => return None,
        }
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoupe_et_textes() {
        let d = Document::lire(
            "<!doctype html><html><head><title>Cariste &amp; quai</title><meta property='og:title' \
             content=\"Cariste H/F\"><script>var a = '<b>';</script></head><body><!-- rien --><div class=\"a b\" \
             id=x>Un <b>gras</b><br>deux<ul><li>trois</li><li>quatre</ul></div><p hidden>caché</p>\
             <span class=\"visually-hidden\">double</span>3 &lt; 5</body></html>",
        );
        assert_eq!(d.titre(), "Cariste & quai");
        assert_eq!(d.meta("og:title").as_deref(), Some("Cariste H/F"));
        let x = d.trouver("div.a#x").unwrap();
        assert_eq!(d.texte(x), "Un gras deux trois quatre");
        assert_eq!(d.texte_lignes(x, 500), "Un gras\ndeux\n\n- trois\n\n- quatre");
        assert!(d.trouver("div.c").is_none());
        let t = d.texte_page_sauf(None, 500);
        assert!(t.contains("3 < 5") && !t.contains("caché") && !t.contains("double") && !t.contains("var a"), "{t}");
    }

    #[test]
    fn selecteurs() {
        let d = Document::lire(
            "<div data-testid=\"carte\"><h2 class=\"t\">A</h2><span itemprop=\"name\">Nom</span></div>\
             <h2 class=\"t\" data-job-id=\"123-x\">B</h2>",
        );
        assert_eq!(
            d.trouver("[data-testid=\"carte\"] [itemprop=\"name\"]").map(|i| d.texte(i)).as_deref(),
            Some("Nom")
        );
        assert_eq!(d.trouver("h2[data-job-id^=\"123\"]").map(|i| d.texte(i)).as_deref(), Some("B"));
        assert_eq!(d.trouver("[data-job-id*=\"-x\"]").map(|i| d.texte(i)).as_deref(), Some("B"));
        assert_eq!(d.tous("h2.t", 10).len(), 2);
        assert!(Selecteur::lire("a>b").is_none());
    }

    #[test]
    fn pages_abimees() {
        // Rien ne fait paniquer : balises jamais fermées, attributs sans fin, « < » seuls, texte coupé au milieu d'un
        // caractère
        for h in [
            "<div><div><div><span>a",
            "<a href=\"x",
            "< < <> </ > <!-- sans fin",
            "<script>jamais fermé",
            "<p class='x'>é",
            "",
            "</p></div>",
            "<\u{e9}t\u{e9}>",
        ] {
            let d = Document::lire(h);
            let _ = d.texte_principal(100);
            let _ = d.titre();
        }
        let long = format!("<p>{}</p>", "é".repeat(HTML_MAX));
        let d = Document::lire(&long);
        assert!(d.texte_principal(50).chars().count() <= 50);
        // Beaucoup de balises jamais fermées : la lecture reste rapide et bornée
        let d = Document::lire(&"<div>x".repeat(200_000));
        assert!(d.len() <= JETONS_MAX);
        assert!(!d.texte_principal(100).is_empty());
    }

    #[test]
    fn scripts_de_donnees() {
        let d = Document::lire(
            "<script type=\"application/ld+json\">{\"a\":\"<b>x</b>\"}</script><script type=\"text/javascript\">x</script>\
             <script type=\" Application/LD+JSON \">[1]</script>",
        );
        assert_eq!(d.scripts("application/ld+json", 1000, 10), vec!["{\"a\":\"<b>x</b>\"}".to_string(), "[1]".into()]);
        assert_eq!(d.scripts("application/ld+json", 3, 10), vec!["[1]".to_string()]);
    }
}
