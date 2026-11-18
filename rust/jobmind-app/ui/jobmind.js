// L'accueil de Jobmind : la maquette C validée par Jordan (maquettes\proposition-c.html, « les dossiers ») : trois
// onglets, Candidatures, Offres et Mon profil, chacun avec une liste fine à gauche et le dossier ouvert à droite. Le
// kit (kit.js) l'appelle à la place de ses tuiles (bloc accueil.perso de l'état, voir « perso » en tête de kit.js) ;
// il garde l'en-tête, le bas, les flèches, A et B (Entrée et Échap) et les sons.
//
// Rien n'est calculé ici : Rust (src\ecran.rs) envoie les onglets, leurs listes et leurs dossiers, textes déjà écrits
// en français. Tout texte reçu est échappé (esc du kit), chaque id et chaque icône vérifiés. Ce fichier ne parle
// jamais à Rust lui-même : un changement d'onglet passe par le kit (action 'onglet', noté au journal).
//
// - un onglet (clic, A ou Entrée) le montre ; sa liste à gauche, son dossier à droite ;
// - une ligne de la liste (les rubriques de Mon profil) ouvre son dossier à droite ;
// - Retour (Échap, B) revient d'abord au premier onglet, puis à la console de Mindset ;
// - les ordres de l'appli : { kit: 'accueil', onglet: id } montre un onglet, { rubrique: id } une rubrique de Mon
//   profil.
// Étape 1 : tout est vide ; chaque onglet montre son état vide.
(() => {
  'use strict';
  const reduit = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
  const idSur = (t) => typeof t === 'string' && /^[a-z]{1,30}$/.test(t);
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
  };
  const icone = (n) => (Object.hasOwn(ICONES, n) ? ICONES[n] : '');

  let P = null;           // le dernier accueil reçu
  let O = null;           // les outils du kit (esc, son, action...)
  let hote = null;        // #ecran
  let onglet = null;      // l'onglet montré
  const rubrique = {};    // la rubrique ouverte, par onglet

  const onglets = () => (Array.isArray(P?.onglets) ? P.onglets.filter((o) => idSur(o?.id)) : []);
  const ongletDe = (id) => onglets().find((o) => o.id === id) || onglets()[0] || null;
  const rubriquesDe = (o) => (Array.isArray(o?.rubriques) ? o.rubriques.filter((r) => idSur(r?.id)) : []);
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

  function listeHTML(o, choisie) {
    const { esc } = O;
    const l = rubriquesDe(o);
    const lignes = l.map((r) => `<button type="button" class="jm-ligne f${r === choisie ? ' sel' : ''}" data-perso="rubrique" data-id="${r.id}" data-k="r:${r.id}" aria-current="${r === choisie}">
        <span class="jm-l-ic">${icone(r.icone)}</span><span class="jm-l-txt"><b>${esc(texteSur(r.nom))}</b>${r.sous ? `<span>${esc(texteSur(r.sous))}</span>` : ''}</span></button>`).join('');
    const vide = !l.length && o.vide ? `<p class="jm-vide-l">${esc(texteSur(o.vide))}</p>` : '';
    return `<div class="jm-liste-t">${esc(texteSur(o.liste))}</div>${lignes}${vide}`;
  }

  function dossierHTML(d) {
    const { esc } = O;
    if (!d || typeof d !== 'object') return '';
    return `<div class="jm-vide${O.anime ? ' entree' : ''}">
        <div class="jm-v-ic plasma-bord">${icone(d.icone)}</div>
        <h1>${esc(texteSur(d.titre))}</h1>
        ${d.texte ? `<p>${esc(texteSur(d.texte))}</p>` : ''}
        ${d.note ? `<p class="jm-note">${esc(texteSur(d.note))}</p>` : ''}
      </div>`;
  }

  function dessinerTout(focusK) {
    const o = ongletDe(onglet);
    if (!o) { hote.innerHTML = ''; return; }
    onglet = o.id;
    const r = rubriqueDe(o);
    hote.innerHTML = `<div class="jm${O.anime ? ' entree' : ''}" id="jm" data-onglet="${o.id}">
        <nav class="jm-onglets" role="tablist" aria-label="Jobmind">${ongletsHTML(o)}</nav>
        <div class="jm-corps">
          <aside class="jm-liste" aria-label="${O.esc(texteSur(o.liste))}">${listeHTML(o, r)}</aside>
          <section class="jm-dossier" aria-live="polite">${dossierHTML(r ? r.dossier : o.dossier)}</section>
        </div>
      </div>`;
    O.anime = false;
    const cible = focusK && hote.querySelector(`[data-k="${CSS.escape(focusK)}"]`);
    (cible || hote.querySelector('.jm-onglet.actif'))?.focus({ preventScroll: true });
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

  // ---------------------------------------------------------------- avec le kit
  window.kitAccueil = Object.freeze({
    // L'accueil dessiné (à chaque nouvel état et à chaque retour sur l'accueil), sur l'onglet montré
    dessiner(h, perso, outils) {
      const garde = hote === h ? document.activeElement?.closest?.('#jm [data-k]')?.dataset.k : null;
      hote = h; P = perso || {}; O = { ...outils };
      dessinerTout(outils.focus || garde);
    },
    // Un onglet ou une ligne (clic, A ou Entrée)
    valider(el) {
      const d = el.dataset;
      if (d.perso === 'onglet' && idSur(d.id)) { if (d.id !== onglet) O.son?.tic?.(); montrer(d.id); return; }
      if (d.perso === 'rubrique' && idSur(d.id)) { O.son?.ok?.(); ouvrirRubrique(d.id); }
    },
    // Retour (Échap, B, le bouton du bas) : d'abord le premier onglet ; sinon le kit revient au menu de Mindset
    retour() {
      const premier = onglets()[0];
      if (premier && onglet !== premier.id) { montrer(premier.id); return true; }
      return false;
    },
    // Les ordres de l'appli : { onglet: id } montre un onglet, { rubrique: id } une rubrique de Mon profil
    ordre(o) {
      if (!hote || !P) return;
      if (idSur(o.rubrique)) {
        const avec = onglets().find((x) => rubriquesDe(x).some((r) => r.id === o.rubrique));
        if (avec) { montrer(avec.id, `r:${o.rubrique}`); ouvrirRubrique(o.rubrique); }
      } else if (idSur(o.onglet)) montrer(o.onglet);
    },
  });
})();
