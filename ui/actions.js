// Dati del menu: icone e azioni per tipo di file.
// Le chiavi inviate al backend sono gli "slug" delle etichette:
//   "pdf" + "Comprimi" + "Per email"  ->  "pdf/comprimi/per-email"
// Se cambi un'etichetta qui, aggiorna la chiave in src-tauri/src/actions.rs.

window.HALO_ICONS = {
  compress: 'M4 14h6v6M3 21l7-7M20 10h-6V4M21 3l-7 7',
  meta: 'M20.6 13.4l-7.2 7.2a2 2 0 0 1-2.8 0L3 13V3h10l7.6 7.6a2 2 0 0 1 0 2.8zM7.5 7.5h.01',
  edit: 'M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6',
  annotate: 'M12 20h9M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z',
  image: 'M3 5h18v14H3zM3 16l5-5 4 4 3-3 6 6M15.5 9.5h.01',
  crop: 'M6 2v14a2 2 0 0 0 2 2h14M18 22V8a2 2 0 0 0-2-2H2',
  redact: 'M3 3l18 18M10.6 5.1A9.8 9.8 0 0 1 12 5c5 0 9 4.5 10 7-.4 1-1.2 2.3-2.4 3.5M6.6 6.6C4.4 8 2.8 10 2 12c1 2.5 5 7 10 7 1.9 0 3.6-.6 5-1.5M9.9 9.9a3 3 0 0 0 4.2 4.2',
  sparkle: 'M12 3l1.8 4.7L18.5 9.5l-4.7 1.8L12 16l-1.8-4.7L5.5 9.5l4.7-1.8z',
  scale: 'M12 3v18M5 7h14M5 7l-3 6a3 3 0 0 0 6 0zM19 7l-3 6a3 3 0 0 0 6 0zM8 21h8',
  down: 'M12 3v12M7 10l5 5 5-5M5 21h14',
  mail: 'M3 6h18v12H3zM3 7l9 6 9-6',
  globe: 'M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18zM3 12h18M12 3c3 3.5 3 14.5 0 18M12 3c-3 3.5-3 14.5 0 18',
  info: 'M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18zM12 11v6M12 7.5h.01',
  trash: 'M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3',
  pinoff: 'M12 21s-7-6.2-7-11a7 7 0 0 1 12.5-4.3M19 10c0 1.2-.4 2.5-1 3.7M3 3l18 18',
  person: 'M12 12a4 4 0 1 0 0-8a4 4 0 1 0 0 8zM4 21a8 8 0 0 1 16 0',
  calendar: 'M4 5h16v16H4zM4 10h16M8 3v4M16 3v4',
  rotate: 'M3 12a9 9 0 1 0 3-6.7M3 4v5h5',
  rotatel: 'M21 12a9 9 0 1 1-3-6.7M21 4v5h-5',
  flip: 'M12 3v18M8 7l-5 10h5zM16 7l5 10h-5z',
  resize: 'M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7',
  sun: 'M12 8a4 4 0 1 0 0 8a4 4 0 1 0 0-8zM12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4',
  half: 'M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18zM12 3v18',
  text: 'M5 5h14M12 5v14M9 19h6',
  highlight: 'M9 11l-5 5v3h3l5-5M9 11l6-6 4 4-6 6zM14 20h7',
  arrow: 'M5 19L19 5M10 5h9v9',
  shape: 'M4 4h9v9H4zM16 11a4 4 0 1 0 0 8a4 4 0 1 0 0-8z',
  sign: 'M3 17c3-6 5-9 7-9 2 0-2 9 1 9 2 0 3-4 5-4s1 4 5 4M3 21h18',
  square: 'M4 4h16v16H4z',
  drop: 'M12 3s6 6.5 6 11a6 6 0 0 1-12 0c0-4.5 6-11 6-11z',
  palette: 'M12 3a9 9 0 0 0 0 18c1.5 0 2-1 2-2s-1-2 0-3 2-1 3-1a4 4 0 0 0 4-4c0-4.4-4-8-9-8zM7.5 11h.01M10 7h.01M15 7.5h.01',
  page: 'M6 3h9l4 4v14H6zM15 3v4h4',
  r11: 'M5 5h14v14H5z',
  r169: 'M2 7h20v10H2z',
  wand: 'M15 4V2M15 10V8M11 6H9M21 6h-2M13 8L3 18l3 3L16 11',
  at: 'M16 12a4 4 0 1 1-8 0a4 4 0 1 1 8 0zM16 12v1.5a2.5 2.5 0 0 0 5 0V12a9 9 0 1 0-3.5 7.1',
  phone: 'M5 3h4l2 5-2.5 1.5a11 11 0 0 0 6 6L16 13l5 2v4a2 2 0 0 1-2 2A16 16 0 0 1 3 5a2 2 0 0 1 2-2',
  card: 'M3 6h18v12H3zM3 10h18M7 15h4',
  bar: 'M4 8h16v8H4zM8 12h8',
  music: 'M9 18V5l12-2v13M9 18a3 3 0 1 1-6 0a3 3 0 1 1 6 0zM21 16a3 3 0 1 1-6 0a3 3 0 1 1 6 0z',
  scissors: 'M6 9a3 3 0 1 0 0-6a3 3 0 1 0 0 6zM6 21a3 3 0 1 0 0-6a3 3 0 1 0 0 6zM20 4L8.1 15.9M14.5 14.5L20 20M8.1 8.1L12 12',
  volume: 'M11 5L6 9H2v6h4l5 4zM15.5 8.5a5 5 0 0 1 0 7M19 5a10 10 0 0 1 0 14',
  wave: 'M2 12h2M6 8v8M10 5v14M14 9v6M18 7v10M22 12h-2',
  play: 'M6 4l14 8-14 8z',
  film: 'M3 4h18v16H3zM7 4v16M17 4v16M3 8h4M3 12h4M3 16h4M17 8h4M17 12h4M17 16h4',
  camera: 'M3 7h4l2-3h6l2 3h4v13H3zM12 17a4 4 0 1 0 0-8a4 4 0 1 0 0 8z',
  gauge: 'M12 14l4-4M4 18a9 9 0 1 1 16 0',
  folder: 'M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z',
  zip: 'M6 3h12v18H6zM12 3v2M12 7v2M12 11v2M11 15h2v3h-2z',
  box: 'M3 7l9-4 9 4v10l-9 4-9-4zM3 7l9 4 9-4M12 11v10',
  lock: 'M5 11h14v10H5zM8 11V7a4 4 0 0 1 8 0v4',
  unlock: 'M5 11h14v10H5zM8 11V7a4 4 0 0 1 7.5-2',
  share: 'M12 3v13M7 8l5-5 5 5M5 14v6h14v-6',
  sort: 'M3 6h12M3 12h8M3 18h4M17 4v16M14 17l3 3 3-3',
  copy: 'M8 8h12v12H8zM4 16V4h12',
  rename: 'M4 20h4L19 9l-4-4L4 16zM13 7l4 4',
  pages: 'M8 3h12v14H8zM4 7v14h12',
  merge: 'M7 3v5a5 5 0 0 0 5 5 5 5 0 0 1 5 5v3M17 3v5a5 5 0 0 1-5 5M7 21v-3',
  qr: 'M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h3v3h-3zM18 18h3v3h-3zM20 14h1M14 20h1',
  subs: 'M3 5h18v14H3zM7 13h4M13 13h4M7 16h7M16 16h1',
  split: 'M12 3v18M4 8l-2 4 2 4M20 8l2 4-2 4',
  table: 'M3 4h18v16H3zM3 10h18M3 15h18M9 4v16',
  chart: 'M4 20V10M10 20V4M16 20v-8M22 20H2',
  pie: 'M12 3v9h9a9 9 0 1 1-9-9zM15 3.5a9 9 0 0 1 5.5 5.5H15z',
  line: 'M3 20l6-8 4 4 8-10M3 20h18',
  filter: 'M3 4h18l-7 9v6l-4 2v-8z',
  search: 'M11 4a7 7 0 1 0 0 14a7 7 0 1 0 0-14zM21 21l-5-5',
  translate: 'M3 5h10M8 3v2M5 5c1 4 4 7 7 8M11 5c-1 4-4 7-7 8M13 21l4-9 4 9M14.5 18h5',
  summary: 'M4 6h16M4 10h16M4 14h10M4 18h7',
  code: 'M8 7l-5 5 5 5M16 7l5 5-5 5',
  clock: 'M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18zM12 7v5l3 2',
  eye: 'M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12zM12 9a3 3 0 1 0 0 6a3 3 0 1 0 0-6z',
  check: 'M20 6L9 17l-5-5',
  list: 'M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01',
  convert: 'M17 1l4 4-4 4M3 11V9a4 4 0 0 1 4-4h14M7 23l-4-4 4-4M21 13v2a4 4 0 0 1-4 4H3'
};

// [etichetta, icona, [[sottoazione, icona], ...]]
// Con { direct: true } e nessuna sottoazione, lo spicchio apre subito il suo pannello (chiave tipo/azione).
window.HALO_TYPES = [
  { kind: 'pdf', tab: 'PDF', demoFile: 'Contratto_2026.pdf', band: '#D93A2B', glyph: 'page', acts: [
    ['Comprimi', 'compress', [['Alta qualità', 'sparkle'], ['Bilanciata', 'scale'], ['Massima', 'down'], ['Per email', 'mail']]],
    ['Pagine', 'pages', [['Unisci', 'merge'], ['Dividi', 'split'], ['Ruota', 'rotate'], ['Riordina', 'sort'], ['Estrai', 'filter']]],
    ['Annota', 'annotate', [['Testo', 'text'], ['Evidenzia', 'highlight'], ['Firma', 'sign'], ['Forma', 'shape']]],
    ['Converti', 'convert', [['Word', 'page'], ['Immagini', 'image'], ['Testo', 'text'], ['PDF/A', 'lock'], ['OCR', 'search']]],
    ['Traduci', 'translate', [], { direct: true }],
    ['Proteggi', 'lock', [['Password', 'lock'], ['Rimuovi password', 'unlock'], ['Filigrana', 'drop'], ['Limita stampa', 'page']]],
    ['Metadati', 'meta', [['Mostra', 'info'], ['Rimuovi tutti', 'trash'], ['Cambia autore', 'person'], ['Cambia data', 'calendar']]],
    ['Oscura dati', 'redact', [['Email', 'at'], ['Telefoni', 'phone'], ['IBAN', 'card'], ['Manuale', 'bar']]]
  ] },
  { kind: 'image', tab: 'Immagine', demoFile: 'Tramonto_Ostia.jpg', band: '#8E44D0', glyph: 'image', acts: [
    ['Modifica', 'edit', [['Ruota', 'rotate'], ['Capovolgi', 'flip'], ['Luminosità', 'sun'], ['Bianco e nero', 'half']]],
    ['Ritaglia', 'crop', [['Libero', 'crop'], ['1 : 1', 'r11'], ['16 : 9', 'r169'], ['Automatico', 'wand']]],
    ['Sfondo', 'image', [['Rimuovi', 'wand'], ['Bianco', 'square'], ['Sfocato', 'drop'], ['Colore', 'palette']]],
    ['Ridimensiona', 'resize', [['Metà', 'resize'], ['Per il web', 'globe'], ['Per email', 'mail'], ['Su misura', 'edit'], ['Upscayl', 'sparkle']]],
    ['Converti', 'convert', [['PNG', 'image'], ['HEIC', 'image'], ['WebP', 'globe'], ['PDF', 'page'], ['Testo', 'text']]],
    ['Annota', 'annotate', [['Testo', 'text'], ['Freccia', 'arrow'], ['Forma', 'shape'], ['Evidenzia', 'highlight']]],
    ['Metadati', 'meta', [['Mostra', 'info'], ['Rimuovi posizione', 'pinoff'], ['Rimuovi tutti', 'trash'], ['Cambia data', 'calendar']]]
  ] },
  { kind: 'audio', tab: 'Audio', demoFile: 'Intervista_podcast.mp3', band: '#E8495F', glyph: 'music', acts: [
    ['Taglia', 'scissors', [['Inizio', 'scissors'], ['Fine', 'scissors'], ['Rimuovi silenzi', 'wave'], ['Dividi', 'split']]],
    ['Volume', 'volume', [['Normalizza', 'gauge'], ['Aumenta', 'volume'], ['Riduci rumore', 'wave'], ['Dissolvenza', 'line']]],
    ['Converti', 'convert', [['WAV', 'wave'], ['AAC', 'music'], ['M4A', 'music'], ['FLAC', 'music']]],
    ['Tag', 'meta', [['Titolo', 'text'], ['Artista', 'person'], ['Album', 'music'], ['Copertina', 'image']]],
    ['Comprimi', 'compress', [['128 kbps', 'down'], ['192 kbps', 'scale'], ['320 kbps', 'sparkle'], ['Mono', 'volume']]]
  ] },
  { kind: 'video', tab: 'Video', demoFile: 'Demo_prodotto.mp4', band: '#2F80ED', glyph: 'play', acts: [
    ['Taglia', 'scissors', [['Inizio', 'scissors'], ['Fine', 'scissors'], ['Dividi', 'split'], ['Unisci', 'merge']]],
    ['Comprimi', 'compress', [['1080p', 'sparkle'], ['720p', 'scale'], ['Per email', 'mail'], ['Per il web', 'globe']]],
    ['Converti', 'convert', [['MOV', 'film'], ['GIF', 'image'], ['Solo audio', 'music'], ['WebM', 'globe']]],
    ['Audio', 'volume', [['Rimuovi audio', 'volume'], ['Normalizza', 'gauge'], ['Dissolvenza', 'line'], ['Sostituisci', 'music']]],
    ['Ruota', 'rotate', [['90° a destra', 'rotate'], ['90° a sinistra', 'rotatel'], ['Capovolgi', 'flip'], ['Ritaglia', 'crop']]],
    ['Fotogrammi', 'camera', [['Copertina', 'image'], ['Estrai foto', 'camera'], ['Anteprima GIF', 'play'], ['Velocità', 'clock']]],
    ['Sottotitoli', 'subs', [['Crea .srt', 'text'], ['Nel video', 'film'], ['Impressi', 'film']]]
  ] },
  { kind: 'document', tab: 'Documento', demoFile: 'Relazione_Q3.docx', band: '#2B6BE0', glyph: 'summary', acts: [
    ['Converti', 'convert', [['PDF', 'page'], ['Testo', 'text'], ['Markdown', 'code'], ['HTML', 'globe']]],
    ['Traduci', 'translate', [], { direct: true }],
    ['Comprimi', 'compress', [['Immagini interne', 'image'], ['Bilanciata', 'scale'], ['Massima', 'down'], ['Per email', 'mail']]],
    ['Proteggi', 'lock', [['Password', 'lock'], ['Sola lettura', 'eye'], ['Filigrana', 'drop'], ['Firma', 'sign']]],
    ['Metadati', 'meta', [['Mostra', 'info'], ['Rimuovi tutti', 'trash'], ['Revisioni', 'clock'], ['Cambia autore', 'person']]],
    ['Oscura dati', 'redact', [['Email', 'at'], ['Telefoni', 'phone'], ['IBAN', 'card'], ['Manuale', 'bar']]]
  ] },
  { kind: 'sheet', tab: 'Foglio', demoFile: 'Bilancio_2026.xlsx', band: '#1E8E4F', glyph: 'table', acts: [
    ['Converti', 'convert', [['CSV', 'table'], ['PDF', 'page'], ['ODS', 'table'], ['JSON', 'code']]],
    ['Pulisci', 'wand', [['Duplicati', 'copy'], ['Righe vuote', 'trash'], ['Spazi in più', 'text'], ['Formato date', 'calendar']]],
    ['Grafico', 'chart', [['Barre', 'chart'], ['Linee', 'line'], ['Torta', 'pie'], ['Pivot', 'table']]],
    ['Ordina', 'sort', [['A → Z', 'sort'], ['Z → A', 'sort'], ['Per data', 'calendar'], ['Filtra', 'filter']]],
    ['Proteggi', 'lock', [['Password', 'lock'], ['Blocca celle', 'lock'], ['Nascondi formule', 'eye'], ['Sola lettura', 'eye']]],
    ['Metadati', 'meta', [['Mostra', 'info'], ['Rimuovi tutti', 'trash'], ['Cambia autore', 'person'], ['Cambia data', 'calendar']]]
  ] },
  { kind: 'folder', tab: 'Cartella', demoFile: 'Progetto_cliente', folder: true, acts: [
    ['Comprimi', 'compress', [['ZIP', 'zip'], ['ZIP protetto', 'lock'], ['7z', 'box'], ['Dividi in parti', 'split']]],
    ['Rinomina', 'rename', [['In serie', 'rename'], ['Data di scatto', 'camera'], ['Aggiungi data', 'calendar'], ['Numera', 'sort'], ['Minuscole', 'text']]],
    ['Organizza', 'folder', [['Per tipo', 'filter'], ['Per data', 'calendar'], ['Per dimensione', 'scale'], ['Appiattisci', 'pages']]],
    ['Pulisci', 'wand', [['Duplicati', 'copy'], ['File vuoti', 'trash'], ['Temporanei', 'clock'], ['File grandi', 'gauge']]],
    ['Analizza', 'chart', [['Dimensione', 'gauge'], ['Tipi di file', 'pie'], ['Modifiche recenti', 'clock'], ['Cerca', 'search']]],
    ['Condividi', 'share', [['Link', 'globe'], ['Email', 'mail'], ['Copia percorso', 'copy'], ['Altro', 'share']]]
  ] },
  { kind: 'archive', tab: 'ZIP', demoFile: 'Foto_matrimonio.zip', band: '#7A6A58', glyph: 'zip', acts: [
    ['Estrai', 'box', [['Qui', 'down'], ['In una cartella', 'folder'], ['Scegli dove', 'folder'], ['Solo alcuni', 'filter']]],
    ['Anteprima', 'eye', [['Contenuto', 'list'], ['Dimensioni', 'gauge'], ['Cerca', 'search'], ['Verifica', 'check']]],
    ['Contenuto', 'folder', [['Aggiungi file', 'page'], ['Rimuovi file', 'trash'], ['Sostituisci', 'copy'], ['Rinomina', 'rename']]],
    ['Proteggi', 'lock', [['Password', 'lock'], ['Cifra AES', 'lock'], ['Rimuovi password', 'unlock'], ['Verifica integrità', 'check']]],
    ['Ricomprimi', 'compress', [['Massima', 'down'], ['Rapida', 'clock'], ['Dividi in parti', 'split'], ['Per email', 'mail']]],
    ['Converti', 'convert', [['7z', 'box'], ['TAR', 'box'], ['Cartella', 'folder'], ['ZIP protetto', 'lock']]]
  ] },
  { kind: 'text', tab: 'Testo', demoFile: 'Appunti_riunione.txt', band: '#6B6B6B', glyph: 'text', acts: [
    ['Converti', 'convert', [['PDF', 'page'], ['Word', 'page'], ['Markdown', 'code'], ['HTML', 'globe'], ['QR code', 'qr']]],
    ['Cerca', 'search', [['Trova', 'search'], ['Sostituisci', 'rename'], ['Conta parole', 'chart'], ['Righe doppie', 'copy']]],
    ['Formatta', 'text', [['Maiuscole', 'text'], ['Minuscole', 'text'], ['Ordina righe', 'sort'], ['Togli spazi', 'trash']]],
    ['Codifica', 'code', [['UTF-8', 'code'], ['Fine riga LF', 'text'], ['Fine riga CRLF', 'text'], ['Rileva', 'wand']]],
    ['Oscura dati', 'redact', [['Email', 'at'], ['Telefoni', 'phone'], ['IBAN', 'card'], ['Manuale', 'bar']]]
  ] }
];

// Presentazioni e tutti gli altri file (tipi che Halo non conosce): menu essenziale.
window.HALO_TYPES.push(
  { kind: 'slides', tab: 'Presentazione', demoFile: 'Lancio_prodotto.pptx', band: '#D35400', glyph: 'pages', acts: [
    ['Converti', 'convert', [['PDF', 'page'], ['Immagini', 'image'], ['PowerPoint', 'pages'], ['ODP', 'pages']]],
    ['Comprimi', 'compress', [['Immagini interne', 'image'], ['ZIP', 'zip']]],
    ['Metadati', 'meta', [['Mostra', 'info'], ['Rimuovi tutti', 'trash']]],
    ['Condividi', 'share', [['Email', 'mail'], ['Altro', 'share'], ['Link', 'globe'], ['Copia percorso', 'copy']]]
  ] },
  { kind: 'other', tab: 'Altro', demoFile: 'Progetto.blend', band: '#6B6B6B', glyph: 'page', acts: [
    ['Comprimi', 'compress', [['ZIP', 'zip'], ['ZIP protetto', 'lock'], ['7z', 'box']]],
    ['Verifica', 'check', [['Impronta', 'check'], ['Confronta', 'copy'], ['Metadati', 'info']]],
    ['Condividi', 'share', [['Email', 'mail'], ['Altro', 'share'], ['Link', 'globe'], ['Copia percorso', 'copy']]]
  ] }
);

window.haloSlug = function (s) {
  return s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
};
