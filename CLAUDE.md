# Halo — regole del progetto

Halo è un'app desktop per **Ubuntu (GNOME su Wayland)** fatta con **Tauri 2**:
un menu radiale di azioni rapide sui file, aperto dal tasto destro di Nautilus.
Struttura e comandi sono descritti in `README.md`: leggilo all'inizio di ogni sessione.

## Regole da non violare

1. **Il design è approvato.** Non cambiare colori, dimensioni, forme degli spicchi,
   animazioni, testi, etichette o icone in `ui/`. Puoi toccare `ui/` solo per correggere
   errori o differenze di resa di WebKitGTK, e **prima** devi spiegarmi cosa cambi e perché.
2. **L'originale non si tocca mai.** Nessuna azione deve modificare, spostare, cancellare
   o sovrascrivere il file di partenza. Ogni risultato si salva accanto con un nome nuovo,
   tramite `out_path()` / `out_dir()` in `src-tauri/src/actions.rs`. I test che lo verificano
   restano e devono passare.
3. **Niente funzioni basate su intelligenza artificiale** (riassunti, trascrizioni,
   riscritture…), salvo mia richiesta esplicita.
4. **Lavora per fasi.** Alla fine di ogni fase fermati, dimmi cosa hai verificato e come,
   e aspetta il mio OK prima di continuare.
5. **Non installare pacchetti di sistema da solo.** Dimmi i comandi `apt`/`sudo` necessari:
   li lancio io.
6. **Non dichiarare "funziona" senza prova.** Per tutto ciò che riguarda la finestra o
   l'aspetto, chiedimi uno screenshot; per le azioni sui file, mostrami l'esito reale.

## Coerenza tra menu e backend

- Le azioni del menu sono in `ui/actions.js`; quelle eseguibili in `SUPPORTED` dentro
  `src-tauri/src/actions.rs`.
- La chiave è `tipo/azione/sottoazione` in minuscolo, senza accenti, con trattini
  (es. *PDF › Pagine › Dividi* → `pdf/pagine/dividi`), calcolata da `haloSlug()`.
- Se cambi un'etichetta in `actions.js`, aggiorna la chiave in `actions.rs` e viceversa.
- Una sottoazione non in `SUPPORTED` compare come "presto": non toglierla dal menu.

## Prima di dire che hai finito

- `cargo test` in `src-tauri/` passa.
- `cargo tauri build` termina senza errori.
- Nessun file originale è stato modificato durante le prove (lavora su copie in `/tmp`).
- Mi hai elencato i file cambiati, con una riga di spiegazione per ciascuno.

## Comunicazione

Rispondimi in italiano, in modo diretto. Se una mia richiesta va contro queste regole
o c'è un rischio tecnico, dimmelo prima di procedere.
