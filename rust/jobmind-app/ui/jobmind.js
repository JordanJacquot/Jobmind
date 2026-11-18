// L'accueil de Jobmind : la maquette C validée par Jordan (maquettes\proposition-c.html, « les dossiers ») : trois
// onglets, Candidatures, Offres et Mon profil, chacun avec une liste fine à gauche et le dossier ouvert à droite. Le
// kit (kit.js) l'appelle à la place de ses tuiles (bloc accueil.perso de l'état, voir « perso » en tête de kit.js) ;
// il garde l'en-tête, le bas, les flèches, A et B (Entrée et Échap) et les sons.
//
// Rien n'est calculé ici : Rust (src\ecran.rs) envoie les onglets, leurs listes et leurs dossiers, textes déjà écrits
// en français. Tout texte reçu est échappé (esc du kit), chaque id et chaque icône vérifiés. Ce fichier ne parle
// jamais à Rust lui-même : chaque geste passe par le kit (action), et Rust vérifie tout de nouveau (src\suivi.rs).
//
// - un onglet (clic, A ou Entrée) le montre ; sa liste à gauche, son dossier à droite ;
// - Candidatures (étape 2) : la liste groupée (À faire maintenant, En attente, Terminées), chaque ligne avec sa
//   pastille de statut et sa ligne d'action, le petit bilan du mois en bas ; une ligne ouvre son dossier (action
//   'c-ouvrir') : le statut à changer par un menu (Entretien : sa date, son heure, son lieu), « où on en est », la
//   prochaine action et ses boutons (« Copier un message », « C'est relancé », « Classer en Sans réponse »), la frise,
//   les pilules « Ce que j'ai envoyé », « L'offre », « Notes » (gardées en quittant le champ), « Modifier » et
//   « Supprimer » (jamais au premier clic) ;
// - « Ajouter une candidature » ouvre le formulaire à droite ; « Enregistrer » envoie tout à Rust ('c-enregistrer'),
//   qui répond par l'ordre { kit: 'accueil', enregistre: id } (le formulaire se ferme sur le dossier) ou par une
//   bulle qui dit ce qui ne va pas (le formulaire reste ouvert). Un brouillon fermé par Retour est gardé ;
// - les dates (la date d'envoi, celle de l'entretien) : le champ JJ/MM/AAAA, et un petit calendrier d'un mois (repris
//   de Paymind) ouvert par l'agenda du champ ou un clic dedans ; un clic sur un jour remplit le champ. Rust vérifie ;
// - Mon profil : ses rubriques ; Réglages montre le délai avant relance ('reglage-relance') ;
// - Retour (Échap, B) ferme d'abord ce qui est ouvert (menu, question, formulaire), revient ensuite au premier
//   onglet, puis à la console de Mindset ;
// - les ordres de l'appli : { kit: 'accueil', onglet: id }, { rubrique: id }, { candidature: id } (le dossier
//   annoncé par une bulle), { ajouter: true }, { modifier: true }, { enregistre: id } (le formulaire enregistré),
//   { fait: id } (un statut changé : le menu ou le petit formulaire de l'entretien se referment), { statut: 'menu' }
//   et { statut: 'entretien' } (le menu des statuts, le petit formulaire de l'entretien du dossier ouvert) ; pour les
//   captures, { calendrier: true } (le calendrier de la date ouverte) et { cliquer: data-k } (un clic sur cet élément).
(() => {
  'use strict';
  const reduit = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
  const idSur = (t) => typeof t === 'string' && /^[a-z]{1,30}$/.test(t);
  const codeSur = (t) => typeof t === 'string' && /^[a-z_]{1,30}$/.test(t);
  const numSur = (n) => Number.isInteger(n) && n > 0 && n < 1e12;
  const texteSur = (t) => (typeof t === 'string' ? t : '');
  const svg = (corps, w = 1.9) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${corps}</svg>`;
  // Les icônes que Rust peut demander (une autre n'est pas dessinée)
  const ICONES = {
    dossier: svg('<path d="M3.5 7.5a2 2 0 0 1 2-2h4l2 2.2h7a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/><path d="M9 14l2.2 2.2L15.5 12"/>'),
    loupe: svg('<circle cx="11" cy="11" r="6.5"/><path d="M20 20l-4.2-4.2"/>'),
    profil: svg('<circle cx="12" cy="8.5" r="3.8"/><path d="M4.5 20c1.2-3.8 4-5.6 7.5-5.6s6.3 1.8 7.5 5.6"/>'),
    cv: svg('<path d="M6.5 3.5h7.5l4.5 4.5v12a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1v-15.5a1 1 0 0 1 1-1z"/><path d="M14 3.5V8h4.5M9 12.5h6M9 16h6"/>'),
    plume: svg('<path d="M19.5 4.5c-6 .5-10.5 4.5-12 11.5l-1.5 4"/><path d="M7.5 16c3 0 6.5-1.5 8.5-5M12 8.5c2.5-.2 4.5-.8 7.5-4"/>'),
    cible: svg('<circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="4"/><circle cx="12" cy="12" r=".6" fill="currentColor"/>'),
    reglages: svg('<circle cx="12" cy="12" r="3"/><path d="M12 3.5v2.2M12 18.3v2.2M3.5 12h2.2M18.3 12h2.2M6 6l1.6 1.6M16.4 16.4L18 18M6 18l1.6-1.6M16.4 7.6L18 6"/>'),
    relance: svg('<path d="M20 11a8 8 0 0 0-14.3-4.9L4 8"/><path d="M4 3.5V8h4.5"/><path d="M4 13a8 8 0 0 0 14.3 4.9L20 16"/><path d="M20 20.5V16h-4.5"/>'),
    horloge: svg('<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>'),
    agenda: svg('<rect x="3.5" y="5" width="17" height="15" rx="2.5"/><path d="M3.5 10h17M8 3v4M16 3v4M8.5 14h2M13.5 14h2"/>'),
    coche: svg('<path d="M5 12.5l4.5 4.5L19 7.5"/>', 2.3),
    copie: svg('<rect x="8.5" y="8.5" width="11" height="11" rx="2.2"/><path d="M15.5 8.5V6.2a1.7 1.7 0 0 0-1.7-1.7H6.2a1.7 1.7 0 0 0-1.7 1.7v7.6a1.7 1.7 0 0 0 1.7 1.7h2.3"/>'),
    plus: svg('<path d="M12 5v14M5 12h14"/>', 2.2),
    crayon: svg('<path d="M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16z"/><path d="M13.5 6.5l4 4"/>'),
    poubelle: svg('<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>'),
    lien: svg('<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>'),
    bas: svg('<path d="M6 9l6 6 6-6"/>', 2.2),
    gauche: svg('<path d="M15 5l-7 7 7 7"/>', 2.4),
    droite: svg('<path d="M9 5l7 7-7 7"/>', 2.4),
  };
  const icone = (n) => (typeof n === 'string' && Object.hasOwn(ICONES, n) ? ICONES[n] : '');
  // La couleur de chaque statut, par son rôle dans le thème (jamais en dur : elle suit le thème de la suite)
  const COULEURS = { envoyee: 'var(--P)', relancee: 'var(--T)', entretien: 'var(--S)', positive: 'var(--vert)', negative: 'var(--rouge)', sans_reponse: 'var(--pale)', abandonnee: 'var(--pale)' };
  const couleurStatut = (c) => (Object.hasOwn(COULEURS, c) ? COULEURS[c] : 'var(--pale)');
  // Les champs du formulaire, dans l'ordre
  const CHAMPS = ['entreprise', 'poste', 'ville', 'date', 'source', 'cv', 'lien', 'envoye', 'notes'];
  const LONGUEURS = { entreprise: 120, poste: 120, ville: 80, date: 12, source: 60, cv: 120, lien: 500, envoye: 8000, notes: 4000 };

  let P = null;           // le dernier accueil reçu
  let O = null;           // les outils du kit (esc, son, action...)
  let hote = null;        // #ecran
  let onglet = null;      // l'onglet montré
  const rubrique = {};    // la rubrique ouverte, par onglet
  // Le suivi : ce qui ne vit que dans la page (oublié en fermant)
  let vueC = 'dossier';   // 'dossier' ou 'formulaire' (à droite)
  let form = null;        // le formulaire ouvert : { id (rien : nouvelle), v: { champs } }
  let brouillon = null;   // une nouvelle candidature commencée puis fermée par Retour
  let ficheVue = null;    // le dossier dessiné en dernier (un autre : ses petites choses se referment)
  let pilule = 'envoye';  // la pilule montrée dans le dossier
  let menu = false;       // le menu des statuts ouvert
  let rdv = null;         // le petit formulaire de l'entretien : { date, heure, lieu }
  let cal = null;         // le petit calendrier ouvert : { ou: 'f' (la date d'envoi) ou 'e' (l'entretien), a, m (0 à 11) }
  let confirme = false;   // « Supprimer ? » demandé
  let choisie = null;     // la ligne choisie à l'instant (avant que Rust ne renvoie le dossier)
  const notes = {};       // les notes tapées, par candidature (gardées si un état arrive pendant qu'on tape)
  let envoiJusqua = 0;    // (un double clic n'envoie pas deux fois)

  const onglets = () => (Array.isArray(P?.onglets) ? P.onglets.filter((o) => idSur(o?.id)) : []);
  const ongletDe = (id) => onglets().find((o) => o.id === id) || onglets()[0] || null;
  const rubriquesDe = (o) => (Array.isArray(o?.rubriques) ? o.rubriques.filter((r) => idSur(r?.id)) : []);
  const suiviDe = () => { const o = onglets().find((x) => x.id === 'candidatures'); return o?.suivi && typeof o.suivi === 'object' ? o.suivi : null; };
  const textesC = () => { const t = suiviDe()?.textes; return t && typeof t === 'object' ? t : {}; };
  const T = (k) => texteSur(textesC()[k]);
  const fiche = () => { const f = suiviDe()?.fiche; return f && typeof f === 'object' && numSur(f.id) ? f : null; };
  // La rubrique ouverte d'un onglet : celle choisie, sinon la première (Mon profil s'ouvre sur Mon CV)
  function rubriqueDe(o) {
    const l = rubriquesDe(o);
    return l.find((r) => r.id === rubrique[o.id]) || l[0] || null;
  }

  // ---------------------------------------------------------------- dessiner
  function ongletsHTML(actif) {
    const { esc } = O;
    return onglets().map((o) => {
      const n = Number.isInteger(o.nombre) && o.nombre > 0 ? `<span class="nb">${o.nombre}</span>` : '';
      const oui = o.id === actif.id;
      return `<button type="button" class="jm-onglet f${oui ? ' actif' : ''}" role="tab" aria-selected="${oui}" data-perso="onglet" data-id="${o.id}" data-k="o:${o.id}">${esc(texteSur(o.nom))}${n}</button>`;
    }).join('');
  }

  function listeHTML(o, choisieR) {
    const { esc } = O;
    const l = rubriquesDe(o);
    const lignes = l.map((r) => `<button type="button" class="jm-ligne f${r === choisieR ? ' sel' : ''}" data-perso="rubrique" data-id="${r.id}" data-k="r:${r.id}" aria-current="${r === choisieR}">
        <span class="jm-l-ic">${icone(r.icone)}</span><span class="jm-l-txt"><b>${esc(texteSur(r.nom))}</b>${r.sous ? `<span>${esc(texteSur(r.sous))}</span>` : ''}</span></button>`).join('');
    const vide = !l.length && o.vide ? `<p class="jm-vide-l">${esc(texteSur(o.vide))}</p>` : '';
    return `<div class="jm-liste-t">${esc(texteSur(o.liste))}</div>${lignes}${vide}`;
  }

  function pastille(code, nom, gros) {
    return `<span class="jm-st${gros ? ' gros' : ''}" style="--c:${couleurStatut(code)}">${O.esc(texteSur(nom))}</span>`;
  }

  // La liste des candidatures : le bouton d'ajout, les groupes, le bilan
  function listeCandidaturesHTML(o, s) {
    const { esc } = O;
    const groupes = Array.isArray(s.groupes) ? s.groupes : [];
    const ouverte = vueC === 'formulaire' ? null : (choisie ?? s.ouverte);
    let h = `<div class="jm-liste-tete"><span class="jm-liste-t">${esc(texteSur(o.liste))}</span>
      <button type="button" class="jm-ajout f" data-perso="ajouter" data-k="c:ajouter" title="${esc(T('ajouter'))}">${icone('plus')}<span>${esc(T('ajouter_court'))}</span></button></div>`;
    for (const g of groupes) {
      const lignes = Array.isArray(g?.lignes) ? g.lignes.filter((x) => numSur(x?.id)) : [];
      if (!lignes.length) continue;
      h += `<div class="jm-groupe-t">${esc(texteSur(g.nom))} <span>${lignes.length}</span></div>`;
      for (const x of lignes) {
        const sel = x.id === ouverte;
        h += `<button type="button" class="jm-cand f${sel ? ' sel' : ''}${x.fini === true ? ' fini' : ''}" data-perso="cand" data-id="${x.id}" data-k="c:${x.id}" aria-current="${sel}">
          <span class="jm-c1"><b>${esc(texteSur(x.nom))}</b>${codeSur(x.statut) ? pastille(x.statut, x.statut_nom) : ''}</span>
          <span class="jm-c2">${esc(texteSur(x.sous))}</span>${x.action ? `<span class="jm-c3">${esc(texteSur(x.action))}</span>` : ''}</button>`;
      }
    }
    if (!groupes.length && o.vide) h += `<p class="jm-vide-l">${esc(texteSur(o.vide))}</p>`;
    const bilan = Array.isArray(s.bilan) ? s.bilan.filter((t) => typeof t === 'string') : [];
    if (bilan.length) h += `<div class="jm-bilan">${bilan.map((t, i) => (i ? `<span>${esc(t)}</span>` : `<b>${esc(t)}</b>`)).join('')}</div>`;
    return h;
  }

  function dossierVideHTML(d, avecAjout) {
    const { esc } = O;
    if (!d || typeof d !== 'object') return '';
    return `<div class="jm-vide${O.anime ? ' entree' : ''}">
        <div class="jm-v-ic plasma-bord">${icone(d.icone)}</div>
        <h1>${esc(texteSur(d.titre))}</h1>
        ${d.texte ? `<p>${esc(texteSur(d.texte))}</p>` : ''}
        ${avecAjout ? `<button type="button" class="jm-bt principal grand f" data-perso="ajouter" data-k="v:ajouter">${icone('plus')}${esc(T('ajouter'))}</button>` : ''}
        ${d.note ? `<p class="jm-note">${esc(texteSur(d.note))}</p>` : ''}
      </div>`;
  }

  // Un bouton du dossier
  const bouton = (perso, k, texte, ic, style, extra = '') => `<button type="button" class="jm-bt${style ? ` ${style}` : ''} f" data-perso="${perso}" data-k="${k}"${extra}>${icone(ic)}<span>${O.esc(texte)}</span></button>`;

  function menuStatutsHTML(f, s) {
    const { esc } = O;
    const statuts = Array.isArray(s.statuts) ? s.statuts.filter((x) => codeSur(x?.id)) : [];
    return `<div class="jm-menu" role="menu">${statuts.map((x) => `<button type="button" role="menuitemradio" aria-checked="${x.id === f.statut}" class="jm-menu-l f${x.id === f.statut ? ' ici' : ''}" data-perso="statut-choix" data-id="${x.id}" data-k="m:${x.id}" style="--c:${couleurStatut(x.id)}"><i></i>${esc(texteSur(x.nom))}</button>`).join('')}</div>`;
  }

  // ---------------------------------------------------------------- le petit calendrier des dates (repris de Paymind)
  // La date d'envoi du formulaire ('f') et celle de l'entretien ('e') : le champ JJ/MM/AAAA reste (taper reste
  // possible) ; l'agenda à sa droite, ou un clic dans le champ, ouvre le mois dessous. Un clic sur un jour écrit
  // « 05/01/2027 » dans le champ. Les jours qui se cliquent, les mois et aujourd'hui viennent de Rust (textes.calendrier),
  // qui vérifie de nouveau la date en enregistrant : c'est sa vérification qui compte.
  const ISO = /^\d{4}-\d{2}-\d{2}$/;
  const deux = (n) => String(n).padStart(2, '0');
  const textesCal = () => { const k = textesC().calendrier; return k && typeof k === 'object' ? k : {}; };
  const prefixe = (ou) => (ou === 'e' ? 'e' : 'f');
  // Un jour tapé, pour le montrer seulement (comme Jour::lire_saisie de Rust) : « 2/11/2026 », « 02.11.26 », « 2026-11-02 »
  function isoDe(t) {
    const s = String(t || '').trim();
    let a, m, j;
    if (ISO.test(s)) [a, m, j] = s.split('-').map(Number);
    else {
      const p = s.split(/[/.-]/).map((x) => x.trim());
      if (p.length !== 3 || !p.every((x) => /^\d+$/.test(x)) || p[0].length > 2 || p[1].length > 2 || !(p[2].length === 2 || p[2].length === 4)) return '';
      j = +p[0]; m = +p[1]; a = p[2].length === 2 ? 2000 + +p[2] : +p[2];
    }
    if (a < 1900 || a > 9999 || m < 1 || m > 12 || j < 1 || j > new Date(Date.UTC(a, m, 0)).getUTCDate()) return '';
    return `${a}-${deux(m)}-${deux(j)}`;
  }
  const saisieDe = (iso) => `${iso.slice(8, 10)}/${iso.slice(5, 7)}/${iso.slice(0, 4)}`;
  function bornesCal(ou) {
    const b = textesCal()[ou === 'e' ? 'entretien' : 'envoi'] || {};
    return { min: ISO.test(b.min || '') ? b.min : '1900-01-01', max: ISO.test(b.max || '') ? b.max : '9999-12-31' };
  }
  const valeurCal = (ou) => (ou === 'e' ? rdv?.date : form?.v?.date);
  // (le calendrier n'est montré qu'avec son champ : le formulaire ouvert, ou le petit formulaire de l'entretien)
  const calOuvert = (ou) => !!cal && cal.ou === ou && (ou === 'e' ? !!rdv : vueC === 'formulaire' && !!form);

  // Le champ de la date : le texte, et l'agenda qui ouvre le calendrier
  function champDate(ou, label, attrs, val, req) {
    const { esc } = O;
    const k = textesCal(), ouvert = calOuvert(ou), p = prefixe(ou);
    return `<div class="jm-champ jm-date${ouvert ? ' ouvert' : ''}"><span>${label}</span><div class="jm-date-l">
      <input class="f" ${attrs} data-champ="date" data-k="${p}:date" maxlength="12" autocomplete="off" spellcheck="false" placeholder="${esc(T('i_date'))}" value="${esc(val || '')}"${req ? ' aria-required="true"' : ''}>
      <button type="button" class="jm-cal-b f" data-perso="cal-ouvrir" data-ou="${ou}" data-k="${p}:cal" aria-label="${esc(texteSur(k.ouvrir))}" title="${esc(texteSur(k.ouvrir))}" aria-expanded="${ouvert}">${icone('agenda')}</button></div></div>`;
  }

  // Le mois montré : les jours de ses bornes se cliquent, le jour du champ et aujourd'hui sont marqués
  function calendrierHTML(ou) {
    const { esc } = O;
    const k = textesCal(), mois = Array.isArray(k.mois) ? k.mois : [];
    const { min, max } = bornesCal(ou), choisi = isoDe(valeurCal(ou)), auj = ISO.test(k.aujourdhui || '') ? k.aujourdhui : '';
    const { a, m } = cal, cle = `${a}-${deux(m + 1)}`, nomMois = texteSur(mois[m]);
    const decale = (new Date(Date.UTC(a, m, 1)).getUTCDay() + 6) % 7, n = new Date(Date.UTC(a, m + 1, 0)).getUTCDate();
    let jours = '<span class="jm-cal-vide"></span>'.repeat(decale);
    for (let j = 1; j <= n; j += 1) {
      const d = `${cle}-${deux(j)}`, cls = `${d === choisi ? ' oui' : ''}${d === auj ? ' auj' : ''}`;
      jours += d >= min && d <= max
        ? `<button type="button" class="jm-cal-d f${cls}" data-perso="cal-jour" data-ou="${ou}" data-v="${d}" data-k="cal:${d}" aria-label="${j} ${esc(nomMois)} ${a}" aria-pressed="${d === choisi}">${j}</button>`
        : `<span class="jm-cal-d non${cls}">${j}</span>`;
    }
    const fleche = (sens, ok, ic, aide) => (ok
      ? `<button type="button" class="jm-cal-fl f" data-perso="cal-mois" data-ou="${ou}" data-v="${sens}" data-k="cal:${sens < 0 ? 'avant' : 'apres'}" aria-label="${esc(texteSur(aide))}" title="${esc(texteSur(aide))}">${icone(ic)}</button>`
      : `<span class="jm-cal-fl non" aria-hidden="true">${icone(ic)}</span>`);
    const titre = texteSur(ou === 'e' ? k.titre_entretien : k.titre_envoi);
    const tete = (Array.isArray(k.jours) ? k.jours : []).map((j) => `<span class="jm-cal-j">${esc(texteSur(j))}</span>`).join('');
    // (il glisse en s'ouvrant seulement, pas à chaque mois changé)
    const entree = cal.neuf === true && !reduit(); cal.neuf = false;
    return `<div class="jm-cal${entree ? ' entree' : ''}" role="group" aria-label="${esc(titre)}">
      <div class="jm-cal-titre">${esc(titre)}</div>
      <div class="jm-cal-tete">${fleche(-1, cle > min.slice(0, 7), 'gauche', k.avant)}<b>${esc(nomMois)} ${a}</b>${fleche(1, cle < max.slice(0, 7), 'droite', k.apres)}</div>
      <div class="jm-cal-grille">${tete}${jours}</div>
      <div class="jm-cal-bas">${bouton('cal-fermer', 'cal:fermer', texteSur(k.fermer), '', 'discret', ` data-ou="${ou}"`)}</div></div>`;
  }

  // Ouvre le calendrier sur le mois du jour du champ (aujourd'hui s'il est vide, faux ou hors des bornes) ; le choix va
  // sur ce jour, sauf ouvert par un clic dans le champ (on peut continuer à taper)
  function ouvrirCal(ou, focusK) {
    const { min, max } = bornesCal(ou), k = textesCal();
    let d = isoDe(valeurCal(ou));
    if (!d || d < min || d > max) {
      const auj = ISO.test(k.aujourdhui || '') ? k.aujourdhui : min;
      d = auj < min ? min : auj > max ? max : auj;
    }
    cal = { ou, a: +d.slice(0, 4), m: +d.slice(5, 7) - 1, neuf: true };
    menu = false; confirme = false;
    dessinerTout(focusK || `cal:${d}`);
    montrerCal();
  }

  // Le champ et son calendrier, montrés ensemble dans le dossier (le champ d'abord, si tout ne tient pas)
  function montrerCal() {
    const box = hote.querySelector('.jm-dossier'), c = hote.querySelector('.jm-cal'), ch = hote.querySelector('.jm-date.ouvert');
    if (!box || !c || !ch) return;
    const b = box.getBoundingClientRect(), haut = ch.getBoundingClientRect().top - 8, bas = c.getBoundingClientRect().bottom + 8;
    let dy = 0;
    if (bas > b.bottom) dy = bas - b.bottom;
    if (haut - dy < b.top) dy = haut - b.top;
    if (dy) box.scrollTop += dy;
  }

  function fermerCal() {
    const p = prefixe(cal?.ou);
    cal = null;
    dessinerTout(`${p}:cal`);
  }

  // Ce qui est tapé dans le champ, calendrier ouvert : il suit (le mois du jour tapé, ce jour marqué), sans tout redessiner
  function suivreCal(ou) {
    if (!calOuvert(ou)) return;
    const d = isoDe(valeurCal(ou)), { min, max } = bornesCal(ou);
    if (d && d >= min && d <= max) cal = { ou, a: +d.slice(0, 4), m: +d.slice(5, 7) - 1 };
    const el = hote.querySelector('.jm-cal');
    if (el) el.outerHTML = calendrierHTML(ou);
  }

  function rdvHTML() {
    const { esc } = O;
    const champ = (k, label, indice, val, large) => `<label class="jm-champ${large ? ' large' : ''}"><span>${esc(label)}</span><input class="f" data-perso="rdv" data-champ="${k}" data-k="e:${k}" maxlength="${k === 'lieu' ? 120 : 12}" autocomplete="off" spellcheck="false" placeholder="${esc(indice)}" value="${esc(rdv[k] || '')}"></label>`;
    const date = champDate('e', esc(T('entretien_date')), `data-perso="rdv" aria-label="${esc(T('entretien_date'))}"`, rdv.date, false);
    return `<div class="jm-rdv"><b class="jm-rdv-t">${icone('agenda')}${esc(T('entretien_titre'))}</b>
      <div class="jm-rdv-champs">${date}${champ('heure', T('entretien_heure'), T('i_heure'), rdv.heure)}
        ${calOuvert('e') ? `<div class="jm-cal-place">${calendrierHTML('e')}</div>` : ''}${champ('lieu', T('entretien_lieu'), T('i_lieu'), rdv.lieu, true)}</div>
      <div class="jm-btns">${bouton('rdv-ok', 'e:ok', T('enregistrer'), 'coche', 'principal')}${bouton('rdv-non', 'e:non', T('annuler'), '', 'second')}</div></div>`;
  }

  function panneauHTML(f) {
    const { esc } = O;
    if (pilule === 'offre') {
      const o = f.offre || {};
      if (!o.lien) return `<p class="jm-panneau-vide">${esc(texteSur(o.vide))}</p>`;
      return `<div class="jm-meta">${esc(texteSur(o.texte))}</div><div class="jm-lien-l"><span class="jm-lien" data-brut>${esc(texteSur(o.lien))}</span>${bouton('copier', 'p:lien', T('copier_lien'), 'lien', 'discret', ' data-quoi="lien"')}</div>`;
    }
    if (pilule === 'notes') {
      const val = Object.hasOwn(notes, f.id) ? notes[f.id] : texteSur(f.notes);
      return `<textarea class="jm-notes f" data-perso="notes" data-k="p:notes" maxlength="4000" spellcheck="true" placeholder="${esc(T('i_notes'))}" aria-label="${esc(T('p_notes'))}">${esc(val)}</textarea><p class="jm-petit">${esc(T('notes_aide'))}</p>`;
    }
    const e = f.envoye || {};
    if (!e.texte) return `<div class="jm-meta">${esc(texteSur(e.meta))}</div><p class="jm-panneau-vide">${esc(texteSur(e.vide))}</p>`;
    return `<div class="jm-meta"><span>${esc(texteSur(e.meta))}</span>${bouton('copier', 'p:envoye', T('copier'), 'copie', 'discret', ' data-quoi="envoye"')}</div><div class="jm-lettre" data-brut>${esc(texteSur(e.texte))}</div>`;
  }

  function dossierHTML(f, s) {
    const { esc } = O;
    const a = f.action && typeof f.action === 'object' ? f.action : null;
    const c = f.calme && typeof f.calme === 'object' ? f.calme : null;
    let h = `<div class="jm-d${O.anime ? ' entree' : ''}"><div class="jm-d-tete"><div class="jm-d-titres"><h1>${esc(texteSur(f.titre))}</h1><div class="jm-d-sous">${esc(texteSur(f.sous))}</div></div>
      <div class="jm-st-place"><button type="button" class="jm-st gros bouton f" data-perso="statut-menu" data-k="d:statut" aria-haspopup="menu" aria-expanded="${menu}" title="${esc(T('statut'))}" style="--c:${couleurStatut(f.statut)}">${esc(texteSur(f.statut_nom))}${icone('bas')}</button>${menu ? menuStatutsHTML(f, s) : ''}</div></div>`;
    if (rdv) h += rdvHTML();
    h += `<p class="jm-ou">${esc(texteSur(f.ou))}</p><p class="jm-ou-sous">${esc(texteSur(f.ou_sous))}</p>`;
    if (a) {
      const bts = (Array.isArray(a.boutons) ? a.boutons : []).filter((b) => codeSur(b?.id)).map((b) => bouton(`act-${b.id === 'sans_reponse' ? 'sans' : b.id}`, `a:${b.id}`, texteSur(b.texte), b.icone, b.principal ? 'principal' : '')).join('');
      h += `<div class="jm-action"><span class="jm-a-ic">${icone(a.icone)}</span><div class="jm-a-txt"><b>${esc(texteSur(a.titre))}</b><span>${esc(texteSur(a.texte))}</span></div><div class="jm-a-bts">${bts}</div></div>`;
    } else if (c) {
      h += `<p class="jm-calme">${icone(c.icone)}<span>${esc(texteSur(c.texte))}</span></p>`;
    }
    const frise = Array.isArray(f.frise) ? f.frise : [];
    if (frise.length) h += `<div class="jm-frise">${frise.map((e, i) => `${i ? '<b class="jm-fl"></b>' : ''}<span><i></i>${esc(texteSur(e?.texte))} <em>${esc(texteSur(e?.date))}</em></span>`).join('')}</div>`;
    const pil = [['envoye', T('p_envoye')], ['offre', T('p_offre')], ['notes', T('p_notes')]];
    h += `<div class="jm-pilules" role="tablist">${pil.map(([id, nom]) => `<button type="button" role="tab" aria-selected="${pilule === id}" class="jm-pil f${pilule === id ? ' actif' : ''}" data-perso="pilule" data-id="${id}" data-k="p:${id}">${esc(nom)}</button>`).join('')}</div>`;
    h += `<div class="jm-panneau">${panneauHTML(f)}</div>`;
    if (confirme) {
      h += `<div class="jm-suppr"><p>${esc(texteSur(f.supprimer))}</p><div class="jm-btns">${bouton('suppr-oui', 'd:oui', T('oui_supprimer'), 'poubelle', 'danger')}${bouton('suppr-non', 'd:non', T('non'), '', 'second')}</div></div>`;
    } else {
      h += `<div class="jm-d-bas">${bouton('modifier', 'd:modifier', T('modifier'), 'crayon', 'discret')}${bouton('supprimer', 'd:supprimer', T('supprimer'), 'poubelle', 'discret danger')}</div>`;
    }
    return `${h}</div>`;
  }

  function formulaireHTML(s) {
    const { esc } = O;
    const v = form.v;
    const champ = (k, req, large) => {
      const lab = `${esc(T(k))}${req ? '' : ` <small>${esc(T('facultatif'))}</small>`}`;
      return `<label class="jm-champ${large ? ' large' : ''}"><span>${lab}</span><input class="f" data-perso="champ" data-champ="${k}" data-k="f:${k}" maxlength="${LONGUEURS[k]}" autocomplete="off" spellcheck="false" placeholder="${esc(T(`i_${k}`))}" value="${esc(v[k] || '')}"${req ? ' aria-required="true"' : ''}></label>`;
    };
    const zone = (k, haut) => `<label class="jm-champ large"><span>${esc(T(k))} <small>${esc(T('facultatif'))}</small></span><textarea class="f${haut ? ' haut' : ''}" data-perso="champ" data-champ="${k}" data-k="f:${k}" maxlength="${LONGUEURS[k]}" spellcheck="true" placeholder="${esc(T(`i_${k}`))}">${esc(v[k] || '')}</textarea></label>`;
    const moyens = (Array.isArray(s.moyens) ? s.moyens : []).filter((m) => codeSur(m?.id));
    const pastilles = moyens.map((m) => `<button type="button" role="radio" aria-checked="${v.moyen === m.id}" class="jm-puce f${v.moyen === m.id ? ' oui' : ''}" data-perso="moyen" data-id="${m.id}" data-k="f:moyen:${m.id}">${esc(texteSur(m.nom))}</button>`).join('');
    return `<form class="jm-form${O.anime ? ' entree' : ''}" novalidate>
      <h1>${esc(form.id ? T('modifier_titre') : T('nouvelle'))}</h1>${form.id ? '' : `<p class="jm-intro">${esc(T('intro'))}</p>`}
      <div class="jm-grille">${champ('entreprise', true)}${champ('poste', true)}${champ('ville')}${champDate('f', esc(T('date')), `data-perso="champ" aria-label="${esc(T('date'))}"`, v.date, true)}
        ${calOuvert('f') ? `<div class="jm-cal-place">${calendrierHTML('f')}</div>` : ''}
        <div class="jm-champ large"><span>${esc(T('moyen'))}</span><div class="jm-puces" role="radiogroup" aria-label="${esc(T('moyen'))}">${pastilles}</div></div>
        ${champ('source')}${champ('cv')}${champ('lien', false, true)}${zone('envoye', true)}${zone('notes')}</div>
      <p class="jm-err" role="alert"></p>
      <div class="jm-btns">${bouton('enregistrer', 'f:enregistrer', T('enregistrer'), 'coche', 'principal')}${bouton('annuler', 'f:annuler', T('annuler'), '', 'second')}</div></form>`;
  }

  // Le dossier de droite de l'onglet Candidatures
  function droiteCandidatures(o, s) {
    if (vueC === 'formulaire' && form) return formulaireHTML(s);
    const f = fiche();
    if (f) return dossierHTML(f, s);
    const vide = !(Array.isArray(s.groupes) && s.groupes.length);
    if (vide) return dossierVideHTML(o.dossier, true);
    return `<p class="jm-choisir">${O.esc(T('choisir'))}</p>`;
  }

  // Les réglages (Mon profil > Réglages) : le délai avant relance
  function reglagesHTML(d) {
    const { esc } = O;
    const r = d.reglage;
    const v = Number.isInteger(r.valeur) ? r.valeur : 14, min = Number.isInteger(r.min) ? r.min : 3, max = Number.isInteger(r.max) ? r.max : 60;
    const choix = (Array.isArray(r.choix) ? r.choix : []).filter((n) => Number.isInteger(n) && n >= min && n <= max);
    return `<div class="jm-reglages${O.anime ? ' entree' : ''}"><h1>${esc(texteSur(d.titre))}</h1>
      <div class="jm-carte-r"><div class="jm-r-tete"><span class="jm-a-ic">${icone('relance')}</span><div><b>${esc(texteSur(r.titre))}</b><p>${esc(texteSur(r.texte))}</p></div></div>
        <div class="jm-r-reglage"><div class="jm-r-valeur">
          <button type="button" class="jm-r-pm f" data-perso="delai" data-v="${v - 1}" data-k="g:moins" aria-label="${esc(texteSur(r.moins))}"${v <= min ? ' disabled' : ''}>−</button>
          <span class="jm-r-n" aria-live="polite"><b>${v}</b> ${esc(texteSur(r.unite))}</span>
          <button type="button" class="jm-r-pm f" data-perso="delai" data-v="${v + 1}" data-k="g:plus" aria-label="${esc(texteSur(r.plus))}"${v >= max ? ' disabled' : ''}>+</button></div>
          <div class="jm-puces">${choix.map((n) => `<button type="button" class="jm-puce f${n === v ? ' oui' : ''}" data-perso="delai" data-v="${n}" data-k="g:${n}" aria-pressed="${n === v}">${n} ${esc(texteSur(r.unite))}</button>`).join('')}</div></div></div>
      ${d.note ? `<p class="jm-note gauche">${esc(texteSur(d.note))}</p>` : ''}</div>`;
  }

  function droiteRubrique(r, o) {
    const d = r ? r.dossier : o.dossier;
    if (d && typeof d === 'object' && d.reglage && typeof d.reglage === 'object') return reglagesHTML(d);
    return dossierVideHTML(d, false);
  }

  // Ce qui est montré change de dossier : ses petites choses ouvertes se referment
  function suivreFiche() {
    const f = fiche(), id = f ? f.id : null;
    if (id !== ficheVue) { ficheVue = id; pilule = 'envoye'; menu = false; rdv = null; confirme = false; if (cal?.ou === 'e') cal = null; }
    if (cal && !calOuvert(cal.ou)) cal = null;
    if (choisie !== null && (id === choisie || !suiviDe()?.groupes?.some((g) => g?.lignes?.some((x) => x?.id === choisie)))) choisie = null;
  }

  function dessinerTout(focusK) {
    const o = ongletDe(onglet);
    if (!o) { hote.innerHTML = ''; return; }
    onglet = o.id;
    // (ce qui est en cours de frappe et la place dans les panneaux, gardés)
    const actif = document.activeElement, kActif = actif?.closest?.('#jm [data-k]')?.dataset.k;
    const curseur = actif && /^(INPUT|TEXTAREA)$/.test(actif.tagName) ? [actif.selectionStart, actif.selectionEnd] : null;
    const hautD = hote.querySelector('.jm-dossier')?.scrollTop || 0, hautL = hote.querySelector('.jm-liste')?.scrollTop || 0;
    const memeOnglet = hote.querySelector('#jm')?.dataset.onglet === o.id;
    let gauche, droite;
    const s = o.id === 'candidatures' ? suiviDe() : null;
    if (s) {
      suivreFiche();
      gauche = listeCandidaturesHTML(o, s);
      droite = droiteCandidatures(o, s);
    } else {
      const r = rubriqueDe(o);
      gauche = listeHTML(o, r);
      droite = droiteRubrique(r, o);
    }
    hote.innerHTML = `<div class="jm${O.anime ? ' entree' : ''}" id="jm" data-onglet="${o.id}">
        <nav class="jm-onglets" role="tablist" aria-label="Jobmind">${ongletsHTML(o)}</nav>
        <div class="jm-corps">
          <aside class="jm-liste${s ? ' suivi' : ''}" aria-label="${O.esc(texteSur(o.liste))}">${gauche}</aside>
          <section class="jm-dossier" aria-live="polite">${droite}</section>
        </div>
      </div>`;
    O.anime = false;
    if (memeOnglet) {
      const d = hote.querySelector('.jm-dossier'), l = hote.querySelector('.jm-liste');
      if (d) d.scrollTop = hautD;
      if (l) l.scrollTop = hautL;
    }
    const k = focusK || kActif;
    const cible = k && hote.querySelector(`[data-k="${CSS.escape(k)}"]`);
    (cible || hote.querySelector('.jm-onglet.actif'))?.focus({ preventScroll: true });
    if (cible && curseur && /^(INPUT|TEXTAREA)$/.test(cible.tagName) && k === kActif) { try { cible.setSelectionRange(curseur[0], curseur[1]); } catch (_) { /* (rien) */ } }
  }

  // Montre un onglet (noté au journal par Rust) ; le choix reste sur l'onglet
  function montrer(id, focusK) {
    const o = ongletDe(id);
    if (!o) return;
    if (o.id !== onglet) {
      onglet = o.id;
      O.anime = !reduit();
      O.action?.('onglet', o.id);
    }
    dessinerTout(focusK || `o:${o.id}`);
  }

  // Ouvre une rubrique de l'onglet montré (Mon profil)
  function ouvrirRubrique(id, focusK) {
    const o = ongletDe(onglet);
    if (!o || !rubriquesDe(o).some((r) => r.id === id)) return;
    if (rubrique[o.id] !== id) { rubrique[o.id] = id; O.anime = !reduit(); }
    dessinerTout(focusK || `r:${id}`);
  }

  // ---------------------------------------------------------------- le suivi
  // Ouvre le dossier d'une candidature (Rust renvoie l'état avec son dossier)
  function ouvrirCandidature(id) {
    vueC = 'dossier';
    if (fiche()?.id !== id) { choisie = id; O.anime = !reduit(); O.action('c-ouvrir', { id }); }
    dessinerTout(`c:${id}`);
  }

  function ouvrirFormulaire(f) {
    const t = textesC();
    if (f) {
      const c = f.champs && typeof f.champs === 'object' ? f.champs : {};
      form = { id: f.id, v: Object.fromEntries([...CHAMPS, 'moyen'].map((k) => [k, texteSur(c[k])])) };
    } else {
      form = { id: null, v: brouillon ? { ...brouillon } : { ...Object.fromEntries(CHAMPS.map((k) => [k, ''])), date: texteSur(t.aujourdhui), moyen: 'mail' } };
    }
    vueC = 'formulaire'; menu = false; rdv = null; confirme = false; cal = null;
    O.anime = !reduit();
    dessinerTout('f:entreprise');
  }

  // Ferme le formulaire ; une nouvelle candidature commencée est gardée en brouillon (sauf après l'enregistrement)
  function fermerFormulaire(garder) {
    if (form && !form.id) brouillon = garder && CHAMPS.some((k) => k !== 'date' && String(form.v[k] || '').trim()) ? { ...form.v } : null;
    form = null; vueC = 'dossier'; cal = null;
    const f = fiche();
    dessinerTout(f ? `c:${f.id}` : 'c:ajouter');
  }

  function enregistrer() {
    if (!form) return;
    const err = hote.querySelector('.jm-err');
    for (const k of ['entreprise', 'poste']) {
      if (!String(form.v[k] || '').trim()) {
        const el = hote.querySelector(`[data-k="f:${k}"]`);
        el?.closest('.jm-champ')?.classList.add('faux');
        if (err) err.textContent = k === 'entreprise' ? "Écris le nom de l'entreprise." : 'Écris le poste.';
        el?.focus({ preventScroll: false });
        O.son?.retour?.();
        return;
      }
    }
    if (Date.now() < envoiJusqua) return;
    envoiJusqua = Date.now() + 800;
    const v = Object.fromEntries([...CHAMPS, 'moyen'].map((k) => [k, String(form.v[k] || '').slice(0, LONGUEURS[k] || 30)]));
    if (form.id) v.id = form.id;
    O.son?.ok?.();
    O.action('c-enregistrer', v);
  }

  // Copie un texte (le presse-papiers de la page ; sinon l'ancienne façon)
  function copier(texte, el, quoi) {
    const fini = () => {
      const s = el?.querySelector('span');
      if (s) { const avant = s.textContent; s.textContent = T('copie'); el.classList.add('fait'); setTimeout(() => { if (s.isConnected) { s.textContent = avant; el.classList.remove('fait'); } }, 1600); }
      O.son?.ok?.();
      const f = fiche(); if (f) O.action('c-copie', { id: f.id, quoi });
    };
    const ancienne = () => {
      const z = document.createElement('textarea'); z.value = texte; z.setAttribute('readonly', ''); z.style.cssText = 'position:fixed;left:-9999px;top:0;opacity:0';
      document.body.appendChild(z); z.select();
      let ok = false; try { ok = document.execCommand('copy'); } catch (_) { ok = false; }
      z.remove(); if (ok) fini();
      el?.focus({ preventScroll: true });
    };
    try { navigator.clipboard.writeText(texte).then(fini, ancienne); } catch (_) { ancienne(); }
  }

  // Les notes tapées : envoyées en quittant le champ, si elles ont changé
  function garderNotes(el) {
    const f = fiche(); if (!f) return;
    const v = el.value;
    if (v === texteSur(f.notes)) { delete notes[f.id]; return; }
    notes[f.id] = v;
    O.action('c-notes', { id: f.id, notes: v.slice(0, 4000) });
  }

  // Un élément du suivi (clic, A ou Entrée) ; vrai s'il en était un
  function validerSuivi(el, d) {
    const f = fiche();
    switch (d.perso) {
      case 'cand': { const id = Number(d.id); if (numSur(id)) { O.son?.ok?.(); ouvrirCandidature(id); } return true; }
      case 'ajouter': O.son?.ok?.(); ouvrirFormulaire(null); return true;
      case 'champ': {   // (Entrée dans un champ d'une ligne : le champ suivant)
        if (el.tagName !== 'INPUT') return true;
        const els = [...hote.querySelectorAll('.jm-form .f')], i = els.indexOf(el);
        els[i + 1]?.focus({ preventScroll: false });
        return true;
      }
      case 'moyen':
        if (!form || !codeSur(d.id)) return true;
        form.v.moyen = d.id; O.son?.bascule?.();
        el.parentElement.querySelectorAll('.jm-puce').forEach((b) => { const oui = b === el; b.classList.toggle('oui', oui); b.setAttribute('aria-checked', String(oui)); });
        return true;
      case 'enregistrer': enregistrer(); return true;
      case 'annuler': O.son?.retour?.(); fermerFormulaire(false); return true;
      case 'pilule': if (['envoye', 'offre', 'notes'].includes(d.id) && pilule !== d.id) { pilule = d.id; O.son?.tic?.(); dessinerTout(`p:${d.id}`); } return true;
      case 'notes': return true;
      case 'copier': {
        if (!f) return true;
        const t = d.quoi === 'lien' ? texteSur(f.offre?.lien) : texteSur(f.envoye?.texte);
        if (t) copier(t, el, d.quoi === 'lien' ? 'lien' : 'envoye');
        return true;
      }
      case 'act-copier': if (f && f.relance) copier(texteSur(f.relance), el, 'relance'); return true;
      case 'act-relancee': if (f) { O.son?.ok?.(); O.action('c-relancee', { id: f.id }); } return true;
      case 'act-sans': if (f) { O.son?.ok?.(); O.action('c-sans-reponse', { id: f.id }); } return true;
      case 'act-entretien':
        if (!f) return true;
        O.son?.ok?.(); rdv = { ...(f.entretien || {}) }; menu = false; dessinerTout('e:date'); return true;
      case 'statut-menu': if (!f) return true; menu = !menu; O.son?.tic?.(); dessinerTout(menu ? `m:${f.statut}` : 'd:statut'); return true;
      case 'statut-choix': {
        if (!f || !codeSur(d.id)) return true;
        menu = false;
        if (d.id === 'entretien') { O.son?.ok?.(); rdv = { ...(f.entretien || {}) }; dessinerTout('e:date'); return true; }
        O.son?.ok?.(); dessinerTout('d:statut');
        if (d.id !== f.statut) O.action('c-statut', { id: f.id, statut: d.id });
        return true;
      }
      case 'rdv': {   // (Entrée : le champ suivant, puis Enregistrer)
        const els = [...hote.querySelectorAll('.jm-rdv .f')], i = els.indexOf(el);
        els[i + 1]?.focus({ preventScroll: false });
        return true;
      }
      case 'rdv-ok':
        if (!f || !rdv) return true;
        O.son?.ok?.();
        O.action('c-statut', { id: f.id, statut: 'entretien', date: String(rdv.date || '').slice(0, 12), heure: String(rdv.heure || '').slice(0, 12), lieu: String(rdv.lieu || '').slice(0, 120) });
        return true;
      case 'rdv-non': O.son?.retour?.(); rdv = null; dessinerTout('d:statut'); return true;
      case 'modifier': if (f) { O.son?.ok?.(); ouvrirFormulaire(f); } return true;
      case 'supprimer': O.son?.ok?.(); confirme = true; dessinerTout('d:non'); hote.querySelector('.jm-suppr')?.scrollIntoView({ block: 'nearest' }); return true;
      case 'suppr-non': O.son?.retour?.(); confirme = false; dessinerTout('d:supprimer'); return true;
      case 'suppr-oui': if (f && confirme) { confirme = false; el.disabled = true; O.action('c-supprimer', { id: f.id }); } return true;
      case 'cal-ouvrir': {
        const ou = d.ou === 'e' ? 'e' : 'f';
        if (ou === 'e' ? !rdv : !form) return true;
        O.son?.tic?.();
        if (calOuvert(ou)) fermerCal(); else ouvrirCal(ou);
        return true;
      }
      case 'cal-mois': {
        if (!cal || !calOuvert(cal.ou)) return true;
        const sens = Number(d.v) < 0 ? -1 : 1, t = new Date(Date.UTC(cal.a, cal.m + sens, 1));
        cal = { ou: cal.ou, a: t.getUTCFullYear(), m: t.getUTCMonth() };
        O.son?.tic?.();
        const k = el.dataset.k;
        dessinerTout(k);
        // (au bout des bornes, la flèche n'est plus là : le choix va sur le premier jour du mois)
        if (!hote.querySelector(`[data-k="${CSS.escape(k)}"]`)) hote.querySelector('.jm-cal .jm-cal-d.f, .jm-cal .f')?.focus({ preventScroll: true });
        return true;
      }
      case 'cal-jour': {
        const ou = d.ou === 'e' ? 'e' : 'f', { min, max } = bornesCal(ou), iso = String(d.v || '');
        if (!calOuvert(ou) || !ISO.test(iso) || iso < min || iso > max) return true;
        if (ou === 'e') rdv.date = saisieDe(iso);
        else {
          form.v.date = saisieDe(iso);
          const err = hote.querySelector('.jm-err'); if (err) err.textContent = '';
        }
        O.son?.ok?.();
        cal = null;
        dessinerTout(`${prefixe(ou)}:cal`);
        return true;
      }
      case 'cal-fermer': O.son?.retour?.(); if (cal) fermerCal(); return true;
      case 'delai': { const n = Number(d.v); if (Number.isInteger(n) && !el.disabled) { O.son?.bascule?.(); O.action('reglage-relance', n); } return true; }
      default: return false;
    }
  }

  // Ce qui se tape : gardé dans la page (un nouvel état qui arrive ne l'efface pas)
  function surSaisie(e) {
    const el = e.target, d = el.dataset || {};
    if (d.perso === 'champ' && form && CHAMPS.includes(d.champ)) {
      form.v[d.champ] = el.value;
      el.closest('.jm-champ')?.classList.remove('faux');
      const err = hote.querySelector('.jm-err'); if (err) err.textContent = '';
      if (d.champ === 'date') suivreCal('f');
    } else if (d.perso === 'rdv' && rdv && ['date', 'heure', 'lieu'].includes(d.champ)) {
      rdv[d.champ] = el.value;
      if (d.champ === 'date') suivreCal('e');
    } else if (d.perso === 'notes') {
      const f = fiche(); if (f) notes[f.id] = el.value;
    }
  }

  // Dans une zone de texte, les flèches, Entrée et Retour arrière restent à la zone (le kit les prendrait pour la
  // manette) ; Échap garde son sens (Retour). Pendant qu'on tape, la souris ne déplace pas le choix.
  function surTouche(e) {
    if (e.target.tagName === 'TEXTAREA' && ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Enter', 'Backspace', 'Home', 'End'].includes(e.key)) e.stopPropagation();
  }
  function surSurvol(e) {
    if (document.activeElement?.tagName === 'TEXTAREA' && document.activeElement !== e.target) e.stopPropagation();
  }
  function surSortie(e) {
    const el = e.target;
    if (el.dataset?.perso === 'notes') garderNotes(el);
  }
  // Un clic dans le champ d'une date ouvre aussi son calendrier (le curseur reste dans le champ)
  function surClic(e) {
    const el = e.target, d = el.dataset || {};
    if (el.tagName !== 'INPUT' || d.champ !== 'date' || (d.perso !== 'champ' && d.perso !== 'rdv')) return;
    const ou = d.perso === 'rdv' ? 'e' : 'f';
    if ((ou === 'e' ? rdv : form) && !calOuvert(ou)) ouvrirCal(ou, d.k);
  }
  let ecoute = null;
  function ecouter(h) {
    if (ecoute === h) return;
    ecoute = h;
    h.addEventListener('click', surClic);
    h.addEventListener('input', surSaisie);
    h.addEventListener('keydown', surTouche);
    h.addEventListener('pointerover', surSurvol);
    h.addEventListener('focusout', surSortie);
    h.addEventListener('submit', (e) => e.preventDefault());
  }

  // ---------------------------------------------------------------- avec le kit
  window.kitAccueil = Object.freeze({
    // L'accueil dessiné (à chaque nouvel état et à chaque retour sur l'accueil), sur l'onglet montré
    dessiner(h, perso, outils) {
      const garde = hote === h ? document.activeElement?.closest?.('#jm [data-k]')?.dataset.k : null;
      hote = h; P = perso || {}; O = { ...outils };
      ecouter(h);
      // (des notes que Rust a gardées : plus besoin de les retenir ici)
      const f = fiche();
      if (f && Object.hasOwn(notes, f.id) && notes[f.id] === texteSur(f.notes) && document.activeElement?.dataset?.perso !== 'notes') delete notes[f.id];
      dessinerTout(outils.focus || garde);
    },
    // Un onglet, une ligne, un bouton (clic, A ou Entrée)
    valider(el) {
      const d = el.dataset;
      if (d.perso === 'onglet' && idSur(d.id)) { if (d.id !== onglet) O.son?.tic?.(); montrer(d.id); return; }
      if (d.perso === 'rubrique' && idSur(d.id)) { O.son?.ok?.(); ouvrirRubrique(d.id); return; }
      validerSuivi(el, d);
    },
    // Retour (Échap, B, le bouton du bas) : d'abord ce qui est ouvert, puis le premier onglet ; sinon le kit revient
    // au menu de Mindset
    retour() {
      if (cal && calOuvert(cal.ou)) { fermerCal(); return true; }
      if (menu) { menu = false; dessinerTout('d:statut'); return true; }
      if (rdv) { rdv = null; dessinerTout('d:statut'); return true; }
      if (confirme) { confirme = false; dessinerTout('d:supprimer'); return true; }
      if (vueC === 'formulaire') { fermerFormulaire(true); return true; }
      const premier = onglets()[0];
      if (premier && onglet !== premier.id) { montrer(premier.id); return true; }
      return false;
    },
    // Les ordres de l'appli
    ordre(o) {
      if (!hote || !P) return;
      if (idSur(o.rubrique)) {
        const avec = onglets().find((x) => rubriquesDe(x).some((r) => r.id === o.rubrique));
        if (avec) { montrer(avec.id, `r:${o.rubrique}`); ouvrirRubrique(o.rubrique); }
        return;
      }
      if (numSur(o.enregistre)) {
        brouillon = null; form = null; vueC = 'dossier'; choisie = null;
        if (onglet === 'candidatures') dessinerTout(`c:${o.enregistre}`);
        return;
      }
      if (numSur(o.fait)) {
        rdv = null; menu = false; confirme = false;
        if (onglet === 'candidatures') dessinerTout('d:statut');
        return;
      }
      if (numSur(o.candidature)) {
        if (onglet !== 'candidatures') montrer('candidatures');
        ouvrirCandidature(o.candidature);
        return;
      }
      if (o.ajouter === true) {
        if (onglet !== 'candidatures') montrer('candidatures');
        ouvrirFormulaire(null);
        return;
      }
      if (o.statut === 'menu' || o.statut === 'entretien') {
        if (onglet !== 'candidatures') montrer('candidatures');
        const f = fiche(); if (!f || vueC !== 'dossier') return;
        if (o.statut === 'menu') { menu = true; rdv = null; dessinerTout(`m:${f.statut}`); } else { rdv = { ...(f.entretien || {}) }; menu = false; dessinerTout('e:date'); }
        return;
      }
      if (o.modifier === true) {
        if (onglet !== 'candidatures') montrer('candidatures');
        const f = fiche(); if (f) ouvrirFormulaire(f);
        return;
      }
      // (pour les captures) le calendrier de la date ouverte : celle du formulaire, sinon celle de l'entretien
      if (o.calendrier === true) {
        if (onglet !== 'candidatures') return;
        if (vueC === 'formulaire' && form) ouvrirCal('f'); else if (rdv) ouvrirCal('e');
        return;
      }
      // (pour les captures) un clic sur un élément de l'écran, par son data-k (« cal:2026-11-10 », « f:enregistrer »)
      if (typeof o.cliquer === 'string' && /^[a-z]{1,4}:[a-z0-9-]{1,12}$/.test(o.cliquer)) {
        hote.querySelector(`#jm [data-k="${o.cliquer}"]`)?.click();
        return;
      }
      if (idSur(o.onglet)) montrer(o.onglet);
    },
  });
})();
