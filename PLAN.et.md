# Gambiit — eestikeelne tööplaan

**Versioon:** 2026-11-05  
**Asukoht:** `~/projektid/gambiit`  
**Praegune seis:** desktop-UI milestone valmis; mäng on Tauri aknas mängitav OpenCode mudeli vastu.  
**Ingliskeelne/GitHubi versioon:** [`PLAN.md`](PLAN.md)

> See on eestikeelne tööversioon. `PLAN.md` on ingliskeelne versioon GitHubi jaoks.
> Sisulised otsused ja teekaart hoitakse mõlemas kooskõlas. Kui need lähevad
> lahku, on käesolev fail eestikeelseks lugemiseks ja `PLAN.md` avalikuks
> dokumentatsiooniks.

---

## Isiksuse käigustiil (2026-11-06)

Igal isiksusel on stiiliprofiil (agressiivsus, ohutus, showmanship, kaos, risk).
Enne agendi käiku hindab backend kandidaate, mis on parimale tolerantsi piires,
ja tõstab isiksusele sobivaima kandidaadi ette. Mudel peab selle käigu mängima
ja põhjendama. Vaikimisi `subtle` = 20 cp, `theatrical` = 40 cp, `strong` = 70 cp.
Külm kalkulaatoril stiili pole. Nii tulevad isiksused esile ilma kontrollimatu
tugevusekaota.

## Funktsioonide disain (2026-11-06)

### Mudelite edetabel (valmis)
Kohalik, filtreeritav tabel salvestatud mängudest. Rühmitab tulemused mudeli
kaupa: mängud, võidud/viigid/kaotused, võiduprotsent, keskmine accuracy,
blunderid/vead/ebatäpsused, Tõeobjektiivi lipukesed, keskmine vastuseaeg,
tokenid ja kulu. Accuracy ja bluff rate vajavad salvestatud analüüsi kokkuvõtet;
analüüsimata mängud annavad siiski tulemuse ja kasutuse. Difficulty ja ajakontroll
on filtrid, mitte vaikimisi eeldused.

### Isiksusemikser (valmis)
Trait-põhine kohandatud isiksus: teatraalsus, enesekindlus, soojus, huumor,
agressiivsus, sõnavõtu pikkus ja petlikkus, pluss nimi ja aktsent. Traitid
mõjutavad prompti, kommentaari ja piiratud käigustiili; Stockfishi kandidaadid
ja difficulty jäävad tugevuse ülempiiriks. Mikseris saab valida nime, aktsendi
ja seitse liugurit. Esimene versioon redigeerib üht kohandatud isiksust; kahe
isiksuse segamine on hilisem laiendus. Petlikkus on seotud Tõeobjektiivi
bluff-seadega.

### Treenerirežiim
Valikuline mänguaegne treener inimesele, vastase isiksusest ja agent-sessioonist
eraldi. Kasutab Stockfishi tõde ja ausat treeneri prompti. Tegevused: seisu
selgitamine, järkjärguline vihje (idee → kandidaat → käik) ja viimase käigu
ülevaade. Ainult nupuvajutusel, mitte automaatselt; ei muuda vastase käiku.

### Tõeobjektiiv 2.0 (valmis)
Pärastmängu Tõeobjektiiv loeb nüüd numbrilisi hinnanguid (`+0,3`), mativäiteid
(`#3`, `mate in 3`) ja hinnanguid (`winning`, `losing`, `equal`, `better`,
`worse`, pluss eesti sünonüümid). See võrdleb iga väidet Stockfishiga ja märgib
≥ 1,50 etturi erinevuse. Bluff-intensiivsuse liugur (0 aus, 50 teatraalne aga
usutav, 100 ebausaldusväärne jutustaja) läheb igasse agendi prompti ja annab
selge liialdusbudžeti. Bluff kehtib ainult hinnangutele ja plaanidele, mitte
reeglitele ega käigu legaalsusele; viigipakkumise otsus jääb ausaks. Edetabel
näitab sellest tulenevat bluff rate’i.

### Režissööri lõige
Pärastmängu narratiiv, mis luuakse analüüsi, kommentaaride ja vestluse põhjal.
Liigendatud vaatusteks (avang, pöördepunkt, finaal), valib eval-kõikumiste järgi
võtmehetked, näitab väikeseid laudu ja tsitaate ning lisab Tõeobjektiivi märkused.
Üks valikuline LLM-kutse pärast analüüsi, mitte live-režiim. Selgelt teater,
objektiivsest analüüsist eraldi.

---

## 1. Lühikokkuvõte

Gambiit on **Rustil põhinev desktop-malemäng**, kus vastane on OpenCode'i AI-mudel.
Mängu juhtmõte pärineb `agentchess/AGENTS.md`-st:

> **Laud on tõde, hääl on vaba.**

See tähendab:

- käikude legaalsus, matt, patt, tulemus ja kellad on mängu enda poolt
  valideeritud — agent ei saa neid muuta ega nende kohta valetada;
- agent võib bluffida hinnangutega, tulevikuplaanidega, emotsioonidega ja
  psühholoogiaga;
- mäng peab olema ka vaadatav ja lõbus, mitte ainult mängitav: kommentaarid,
  isiksused, helid, animatsioonid ja pärastmängu teater.

UI on inglise keeles vaikimisi (GitHubi ja rahvusvahelise publiku jaoks), eesti
keel on valikuline. See tööplaan on eestikeelne, et sul oleks lihtsam lugeda.

---

## 2. Kinnitatud otsused

1. **Eraldi projekt** `~/projektid/gambiit`. `agentchess` jääb puutumata kui
   algne brauseriprototüüp.
2. **Stack:** Rusti südamik + **Tauri 2** + **Svelte 5**. Kogu male loogika,
   AI-integratsioon, salvestus ja API on Rustis; UI on Tauri aknas jooksev
   Svelte/CSS kiht. See annab parima disaini-, teema- ja animatsioonivabaduse.
3. **Agent režiim:** prompt-režiim on põhiline. Pärast sinu käiku näeb agent
   seisu ja teeb oma käigu. Terminalis ei pea midagi trükkima. Seadetes saab
   valida:
   - **Auto** (vaikimisi): agent vastab kohe pärast sinu käiku;
   - **Button**: agent käib alles siis, kui vajutad “Agent move”;
   - **Manual chat** (MCP, hiljem): sina kirjutad OpenCode'is agentile.
4. **Stockfish on esimese klassi osa:** hübriidmäng (Stockfish pakub
   kandidaadid, mudel valib ja põhjendab) + pärastmängu analüüs. Stockfish on
   väline GPL-binaar; mäng tuvastab selle ega paki vaikimisi kaasa.
5. **Vaikimisi inglise keel**, eesti keel valikuline. Projekt on mõeldud GitHubi.
6. **Seaded on põhifunktsioon:** mänguomadused ja teatri-funktsioonid peavad
   olema sisse/välja lülitatavad, mitte koodi sees peidetud.

---

## 3. Praegune seis (2026-11-05)

### 3.1 Phase 0 — terminaliprototüüp

Valmis:

- `gambiit-core` — malereeglid, legaalsed käigud, FEN/SAN/UCI, kolmekordne
  kordus, 50-käiguline, alistumine, perft, undo.
- `gambiit-engine` — Stockfish 19 UCI klient, MultiPV kandidaadid ja hinnangud.
- `gambiit-agent` — OpenCode prompt-transport, NDJSON-parser, range JSON-kontrakt,
  retry'd, sessiooni taas kasutamine.
- `gambiit-cli` — mängitav terminaliversioon.

Testitud:

- `perft(4)` = **197 281**.
- Stockfish 19 tuvastatud (`/usr/bin/stockfish`), UCI kandidaadid töötavad.
- OpenCode Desktop 2.2.5 CLI töötab mudeliga
  `opencode-go/deepseek-v4.1-flash`; 2-ply mäng ~6 s, korrektne käik ja
  teatraalne kommentaar.
- Kohalik Ollama `ollama/gemma4` töötab samuti (aeglasem, ~20–24 s käik).
- Vana eraldi CLI `/home/rk/.opencode/bin/opencode` 1.18.35 `auth.json` võti on
  aegunud; mäng eelistab nüüd OpenCode Desktopi `opencode-cli`-d. Vajadusel saab
  `OPENCODE_BIN`-iga muu binaari ette anda.

### 3.2 Desktop-UI milestone

Valmis:

- Tauri 2 + Svelte 5 desktop-aken (vaikimisi maksimeeritud).
- Cburnett SVG malendid, klikk-käigud, legaalsete käikude täpid, löögirõngad,
  viimase käigu highlight, check'i märk ja promotsiooni valik. Laua lahtrid on
  reaktiivsed, seega malendid uuenevad kohe nii inimese kui agendi käigu järel.
- Mängijaribad: agent ülal, sina all; võetud malendid ja materjaliede.
- Kolm teemat: **Salon**, **Tournament**, **Midnight**, vahetatavad seadetes.
- Seaded: mudel (avastati 788 mudelit), illustreeritud isiksus, Auto/Button,
  valge/must, teema, fondi suurus (90–150%, vaikimisi 120%), eraldi UI- ja
  agendikeel, heli, animatsioonid ja Stockfishi raskusaste.
- Lokaliseeritud UI + agendi keel: `src/locales/*.json` automaatne avastamine,
  inglise ja eesti kataloog; keelevaik ainult seadetes; UI- ja agendikeel on
  teineteisest sõltumatud.
- Otsitav mudelivalik pakkujate kaupa ja mudelite nimekirja värskendus.
- Valikulised pakkuja API-võtmed `~/.config/gambiit/credentials.json` failis
  (0600), edastatakse keskkonnamuutujana privaatsele OpenCode'i lapsprotsessile;
  tühi võti kasutab OpenCode'i enda autentimist.
- Automaatne salvestus iga käigu järel, viimase mängu jätkamine, mängude
  raamatukogu, PGN eksport/import kommentaaridega ja tolerantne PGN-parser.
- Pärastmängu Stockfishi analüüs: täpsus, käikude klassifikatsioon, eval-graaf,
  valitud käigu detailid ja Tõeobjektiiv.
- Tõeobjektiiv 2.0 loeb numbrilisi, mati- ja seisuväiteid; bluff-intensiivsuse
  liugur (0–100) läheb igasse agendi prompti ega luba kunagi reeglivalet.
- Kohandatud teemaredaktor live-eelvaatega ja lokaalselt salvestatud paletiga.
- Viis sisseehitatud malendikomplekti (Cburnett, Livius, Meridian, Meridian
  Shaded ja Letter) ning kohandatud kaustad `~/.local/share/gambiit/pieces/`
  all, puuduvate malendite varundusega Cburnetti komplektist.
- Seadete salvestus ei alusta uut mängu välimuse/liidese muudatuste puhul;
  mängu mõjutavad muudatused näitavad hoiatust ja valikut.
- Vastase vestlus ja ametlikud viigipakkumised, mõlemad seadetes sisse/välja
  lülitatavad.
- Valikuline malekell bullet/blitz/rapid/klassikaline/kohandatud ajakontrolliga,
  lisaaeg, lipu langemine ja PGN-i `TimeControl` väli.
- Kohalik mudelite edetabel salvestatud mängudest: võiduprotsent, accuracy,
  blunderid, bluff rate, vastuseaeg, tokenid ja kulu.
- Viis Stockfishi raskusastet — Algaja, Harrastaja, Edasijõudnu, Tugev ja
  Meister — pluss Kohandatud. Taseme valik seab depth/MultiPV liugurid; liuguri
  liigutamine valib Kohandatud. Igal preseedil on ka Stockfishi `Skill Level`.
- Illustreeritud teatri-isiksusekaardid: Teatrimeister, Külm kalkulaator, Pettur,
  Hirmunud, Treener ja Kaootiline, pluss trait-põhine kohandatud isiksus (nimi,
  aktsent ja seitse liugurit). Isiksus mõjutab kommentaari ja piiratud
  käigustiili; käik peab tulema Stockfishi kandidaatide hulgast. Avatar
  kuvatakse ka mängijaribas ja kommentaarikaartidel.
- Kommentaaripaneel: hinnang, kandidaadid koos tagasilükkamise põhjustega, plaan,
  mõtlemisaeg ja tokenikulu.
- Tagasivõtt järgib `AGENTS.md` reeglit: ainult sinu enda viimast käiku saab
  tagasi võtta; agendi käiku mitte.
- Alistumine ja uus mäng.
- Rust-käsud: `new_game`, `get_state`, `make_move`, `agent_move`, `takeback`,
  `resign`, `list_models`.
- Päris UI-mäng: `e4` → `...e5`, kommentaar ja kandidaadid kuvati.
- Screenshotid: `docs/ui-preview.png`, `docs/settings-preview.png`.

Testid:

- **74 Rust testi** rohelised (core 10, engine 2, agent 15, Tauri app 47) +
  9 ignored testi (päris-mudeli käigu-, isiksusestiili-, bluff-intensiivsuse-,
  vestluse/viigi-, MCP kahe käigu- ja eestikeelne smoke-test ning kolm
  Stockfishi analüüsi testi).
- 1 ignored **agent_smoke** test: `e4` → `...c5` 8,9 sekundiga, 522 tokenit.
- `cargo clippy --all-targets -- -D warnings` puhas.
- `pnpm check` — 0 viga, 0 hoiatust.
- Tauri debug build töötab eraldi, ilma Vite serverita.

### 3.3 Mis veel puudub

- rohkem sisseehitatud teemasid ja kureeritud malendikomplekte;
- forced-colors/high-contrast režiim ja täielik ekraanilugeja audit;
- MCP-režiim ja lokaalne Axum API;
- vestlus (chat) agendiga;
- Director's Cut, mudelite edetabel, broadcast-režiim.

---

## 4. Põhimõtted, mis tulevad `AGENTS.md`-st

| Põhimõte | Kuidas see mängus välja näeb |
|---|---|
| Laud on tõde, hääl on vaba | serveripoolne valideerimine; eval-riba vaikimisi peidetud; tõeobjektiiv alles pärast mängu |
| Loe enne käiku | agent saab enne käiku FEN-i ja legaalsed käigud; vananenud/ebaseaduslik käik lükatakse tagasi |
| Kohustuslik kommentaar | struktureeritud kaart: hinnang, kandidaadid, valik, plaan |
| Tulevikuplaanid on salajased | UI ei näita agendi mõttekäiku; ainult avalik kommentaar; vestluses võib bluffida |
| Tagasivõtmine | inimese käiku ei võeta kunagi tagasi; agendi käik ainult siis, kui inimene pakub ja agent nõustub (MCP faasis) |
| Draama ja toonimoodid | isiksusepreseedid + dünaamiline register; lõputribuudid |
| Kunagi ei valeta mehhaanika kohta | laud, staatus ja tulemus on alati tõesed; agendi eksimus on nähtav |
| Lõpukõne | tulemusekraanil üks tribuudireast + revanšinupp |
| Kogu mäng on etendus | teater, helid, animatsioonid, broadcast-režiim |

---

## 5. Tehniline stack

| Kiht | Valik | Miks |
|---|---|---|
| Keel | Rust | core, API, agent, salvestus |
| UI shell | Tauri 2 | native aken, väike binaar, Rust backend |
| Frontend | Svelte 5 + TypeScript + Vite | animatsioonid, reaktiivsus, CSS-teemad |
| Malereeglid | `shakmaty` | legaalsed käigud, FEN/SAN/UCI, tulemused |
| PGN | `pgn-reader` + oma kirjutaja | import/eksport, kommentaarid, kellad |
| Lokaalne API (järgmine) | `axum` + `tokio` | HTTP + SSE agentidele, token-kaitse |
| AI | OpenCode CLI (`opencode models`, `opencode run`) | kasutab sinu olemasolevat autentimist ja kvooti |
| MCP (hiljem) | `gambiit-mcp` / `gambiit mcp` | annab OpenCode'ile male-tööriistad |
| Analüüs | Stockfish UCI | hübriidvastane, pärastmängu analüüs |
| Salvestus | JSON + PGN; hiljem SQLite | lihtne ja portatiivne |
| Testid | `cargo test`, Vitest, Playwright, fake-agent | korrektsus + UI kindlus |
| Pakend | `tauri build`, PKGBUILD, `.desktop` | Arch/CachyOS ennekõike |

Keskkond: Rust 1.99, Node 26, pnpm 12, webkit2gtk 2.54, OpenCode Desktop 2.2.5,
Stockfish 19.

---

## 6. Arhitektuur

### 6.1 Praegune

```
Svelte UI  ──Tauri IPC──►  Session (Rust, src-tauri)
                              ├── Game (shakmaty)
                              ├── AgentSession (OpenCode prompt)
                              ├── Stockfish (UCI)
                              └── Commentary / settings / history
```

- UI saadab Tauri käsud `new_game`, `make_move`, `agent_move`, `takeback`,
  `resign`, `list_models`.
- Kõik käigud valideeritakse `gambiit-core`-s.
- Agent ei saa otse state'i muuta; ainult `agent_move` kaudu.
- `agent_move` töötab `spawn_blocking` sees, et UI ei jääks blokeerituks.

### 6.2 Sihtarhitektuur

```
Svelte UI ──Tauri IPC──┐
                       ├── GameService (Rust)  ← ainus tõde
Lokaalne HTTP API ─────┤      ├── shakmaty
   ▲        ▲          │      ├── GameStore (JSON/PGN)
   │        └─ MCP ────┘      └── OpponentOrchestrator
   │                                ├── OpenCode prompt
OpenCode 2 ────────────────────────┼── OpenCode MCP
                                   └── Stockfish
```

- Lokaalne API seotakse `127.0.0.1`-ga, token kirjutatakse
  `$XDG_RUNTIME_DIR/gambiit/session.json` faili (0600).
- MCP sild on õhuke adapter lokaalse API peal.
- Tulevikus saab sama `GameService`'i kasutada nii Tauri UI, HTTP API kui MCP.

---

## 7. AI vastane

### 7.1 Auto prompt-režiim (praegu)

1. Inimene teeb käigu.
2. Mäng teeb agentuuri jaoks prompti: FEN, PGN, legaalsed käigud, staatus,
   inimene/agent värv, isiksus, keel, Stockfishi kandidaadid.
3. Käivitatakse `opencode run -m <mudel> --format json -s <sessioon> "<prompt>"`.
4. Mudel vastab range JSON-iga:

```json
{
  "move": "e7e5",
  "comment": "Positsioon on võrdne...",
  "eval": "-0.2",
  "candidates": [
    { "move": "e7e5", "why": "arendab ja hoiab pinget" },
    { "move": "c7c5", "why_rejected": "liiga terav siin" }
  ],
  "plan": "Avalik plaan, mis võib olla bluff"
}
```

5. Käik valideeritakse. Katki JSON või ebaseaduslik käik → üks paranduskatse;
   kui ikka ei õnnestu, kuvatakse viga ja valikud (proovi uuesti, vaheta mudelit,
   Stockfishi fallback).
6. Kehtiv käik rakendatakse ja kommentaar kuvatakse Teatri paneelis.

### 7.2 Auto / Button / Manual

- **Auto** — pärast sinu käiku käivitub agent automaatselt.
- **Button** — agent käivitub “Agent move” nupust.
- **Manual chat** (MCP, hiljem) — sina suhtled OpenCode'is, agent kasutab
  `chess_*` tööriistu.

### 7.3 MCP-režiim (järgmine suurem integratsioon)

Tööriistad:

- `chess_state()`
- `chess_play_move(move, comment?)`
- `chess_comment(text, structured?)`
- `chess_chat(text)`
- `chess_new_game(human_color?)`
- `chess_offer_takeback()`, `chess_accept_takeback()`, `chess_decline_takeback()`
- `chess_offer_draw()`, `chess_accept_draw()`, `chess_resign()`
- `chess_get_pgn()`

MCP sild loeb discovery-failist pordi ja tokeni ning forwardib kutsed lokaalsele
HTTP API-le. OpenCode'i agent saab ainult `gambiit_*` tööriistad; bash/edit/webfetch
keelatakse.

### 7.4 Stockfish hübriid

| Raskusaste | Käitumine |
|---|---|
| **Algaja** | Skill 0, sügavus 5, top 6 kandidaati |
| **Harrastaja** | Skill 5, sügavus 8, top 5 |
| **Edasijõudnu** (vaikimisi) | Skill 10, sügavus 11, top 4 |
| **Tugev** | Skill 15, sügavus 15, top 3 |
| **Meister** | Skill 20, sügavus 19, top 2 |
| **Kohandatud** | kasutaja depth/MultiPV, Skill 20 |

Taseme valik seab depth/MultiPV liugurid; liuguri liigutamine valib Kohandatud.
Mudel peab valima Stockfishi kandidaatide hulgast. Isiksus mõjutab kommentaari
ja piiratud lähedaste kandidaatide käigustiili, mitte kunagi käiku väljaspool
kandidaatide hulka või valitud tolerantsi.

Kui Stockfish puudub, töötab mäng edasi ilma engine'ita; kuvatakse paigaldusjuhis.

---

## 8. UI ja disain

### 8.1 Disainisuund

“Teatriprogramm”, mitte tavaline malerakendus:

- süsinikmust taust, soe pähkel/kreem laud, messingine aktsent;
- `Fraunces`/`Playfair Display` pealkirjadele, `Inter` UI-le, `JetBrains Mono`
  käikudele (praegu süsteemsed varud, fondid lisatakse hiljem);
- malelaud on “lava”, parem paneel on “kavad”: Käigud / Teater / Vestlus / Analüüs;
- mängijaribad: agendi avatar, isiksus, mudel, staatus; inimese kaart koos
  võetud malendite ja materjalieduga;
- kommentaarikaardid: hinnang, kandidaadid, tagasilükkamise põhjused, plaan;
- tulemusekraan: tulemus, põhjus, tribuudirida ja revanš.

### 8.2 Praegune paigutus

- Ülal: bränd, staatus, pööramis- ja seadenupp.
- Keskel: agent, laud, sina, tegevusnupud.
- Paremal: vahekaardid **Moves / Theater / Analysis**, keritav sisu ja
  mudeli/isiksuse jalus.
- Laius alla 1120 px: paneel läheb laua alla; laud skaleerub.
- Laud on piiratud `calc(100vh - 300px)`-ga, et mängijaribad ja nupud alati
  nähtavale jääksid.

### 8.3 Interaktsioon

- Klikk-klikk ja drag-and-drop (drag on järgmine lihv).
- Legaalsed sihid täppidega, löögid rõngastega, viimane käik highlight'itud,
  check punase haloga.
- Promotsiooni valik (Q/R/B/N).
- Klaviatuur: nupud on fookuskõlblikud; nooltega navigeerimine ja kiirklahvid on
  järgmine samm.
- Helid: käik, löök, check, mati-fanfaar; vaigistus seadetes.
- `prefers-reduced-motion` austab seadet.

### 8.4 Ligipääsetavus ja keel

- Ruudud on päris `<button>`-id ARIA-siltidega (“e4, valge ettur”).
- Kõrge kontrastsusega teema on plaanis; värvipimedussõbralik laud.
- Eesti/inglise UI ja agendikeel tulevad `src/locales/*.json` kataloogist;
  uue keele lisamiseks piisab uuest JSON-failist. UI- ja agendikeel valitakse
  eraldi seadetes; UI eelvaade on kohe, agent kasutab valitud keelt uues mängus.
- Fontide suurus ja paigutus kohanevad.

---

## 9. Teemad ja malendid

### Praegu

- **Salon** — pähkel/kreem, messing, tume UI (vaikimisi).
- **Tournament** — roheline/kreem, hele UI.
- **Midnight** — terasinine/süsi, neoon-aktsent.
- Cburnett SVG malendikomplekt (`src/assets/pieces/cburnett/`), litsents
  `ATTRIBUTION.md`-s.

### Järgmine

- Teemaredaktor: laua värvid, highlight'id, ääris, koordinaadid, tekstuur,
  malendikomplekt, suurus, vari, UI-aktsent, animatsiooni kiirus, helipakett.
- Live mini-laud eelvaateks, kontrastsuse hoiatused, “Randomiseeri”,
  salvesta/duplikeeri/ekspordi/impordi.
- Teemad JSON-failidena; sisseehitatud read-only, kasutaja omad
  `~/.local/share/gambiit/themes/` all.
- Kohandatud malendikomplektid: kaust `manifest.json` + SVG-d; litsentsimetaandmed.
- Tulevikus: teemagalerii ja `.gambiit-theme` failid.

---

## 10. Seaded

### Vastane

- mudel (`provider/model`, otsitav nimekiri `opencode models`-ist);
- isiksus (Theater Master, Cold Calculator, Trickster, Panicker, Coach, Chaotic);
- vastuserežiim: **Auto** / **Button** / **Manual chat** (MCP);
- teatriline viivitus (0–2 s);
- mõtlemise timeout (30–300 s);
- retry'de arv;
- fallback: paus / Stockfishi käik / juhuslik legal;
- sessiooni taas kasutamine.

### Mäng

- inimene valge/must;
- kell: puudub (vaikimisi) / ainult inimene / mõlemad + agendi mõtlemise eelarve;
- ajakontroll: relaxed, 10+5, 15+10, 30+0, custom;
- tagasivõtt: väljas / ainult inimene / pakkumine + nõustumine;
- viigipakkumised, alistumise kinnitus;
- legaalsed käigud, viimane käik, koordinaadid, võetud malendid, materjal;
- PGN eksport/import, autosave, viimase mängu jätkamine.

### Teater

- kommentaarid sisse/välja;
- pikkus: lühike / tavaline / täis;
- kandidaadid ja tagasilükkamise põhjused;
- eval-väited;
- vestlus;
- pettuse tase: off / light / full;
- draama tase: calm / theatrical / maximum;
- tögamine ja psühholoogiline sõda;
- lõputribuudid;
- pressikonverents (stretch);
- Director's Cut (stretch);
- Truth Lens (pärast mängu);
- keel: UI ja agent samast `src/locales/*.json` kataloogist (en/et; uued keeled ilma koodimuudatusteta).

### Välimus

- laua teema, malendikomplekt, UI teema (tume/hele/auto);
- laua suurus, külgpaneele tihedus, fondi suurus;
- kõrge kontrast, reduced motion;
- helipakett, helitugevus, mute.

### Engine

- Stockfishi tee (auto + custom);
- raskusaste: algaja / harrastaja / edasijõudnu / tugev / meister / kohandatud;
- depth/MultiPV liugurid; preseet seab need, liuguri muutus valib Kohandatud;
- MultiPV, depth/time;
- analüüsi depth/time;
- eval-riba mängu ajal (vaikimisi väljas);
- engine lines ainult analüüsis.

### Advanced

- OpenCode binaari tee (`OPENCODE_BIN`);
- MCP režiim ja agent-workspace;
- logid, diagnoosimine;
- andmete kaust, varundus, seadete reset;
- eksperimentaalsed funktsioonid.

---

## 11. Funktsioonide maht

### Valmis

- kõik malereeglid;
- kohalik kahe mängija režiim (UI-s saab mängida mõlema poolega, kui agent ei
  ole seatud);
- laud, käigud, staatus, pööramine;
- agent prompt-režiim + mudelivalik;
- Auto/Button;
- kommentaarid;
- uus mäng, poole valik;
- kolm teemat + seaded;
- tagasivõtt, alistumine;
- Stockfishi hübriid.

### Järgmine

- pärastmängu analüüs (accuracy, blunderid, eval-graaf);
- teemaredaktor + kohandatud malendikomplektid;
- MCP + lokaalne Axum API;
- vestlus;
- Truth Lens, Director's Cut, mudelite edetabel, broadcast.

### Stretch

- avangurolet, handikapid, pressikonverents, grudge match;
- treenerirežiim, kohalik TTS, pusled;
- online-mäng (eraldi suur projekt).

---

## 12. Teekaart

| Etapp | Sisu | Staatus |
|---|---|---|
| **0. Riskispike** | CLI + OpenCode prompt + Stockfish | ✅ valmis |
| **1. Core + API** | core, engine, agent; lokaalne API veel puudub | osaliselt |
| **2. UI shell** | Tauri + Svelte, laud, kohalik mäng | ✅ valmis |
| **3. Agent UI-s** | Auto/Button, mudel, isiksus, kommentaarid | ✅ valmis |
| **4. Disainisüsteem** | 4 teemat ✅, teemaredaktor ✅, 5 sisseehitatud malendikomplekti ✅, kohandatud komplektid ✅, animatsioonid, helid, klaviatuuri/ekraanilugeja põhjad ✅ | ✅ valmis; rohkem teemasid hiljem |
| **5. Mängu täiuslikkus** | PGN ✅, salvestus ✅, analüüs ✅, kellad ✅, viik/alistumine ✅; tulemus | osaliselt |
| **6. Teater ja ekstra** | Vestlus ✅, viigipakkumised ✅, MCP-käigud ✅, mudelite edetabel ✅; MCP vestlus/analüüs, Director's Cut | osaliselt |
| **7. Pakend** | PKGBUILD, `.desktop`, release, docs | hiljem |

Soovitatud järjekord:

1. **Rohkem teemasid ja malendikomplekte** — disainipolish.
2. **MCP tööriistad vestlusele, viigile ja analüüsile** — MCP-režiimi laiendus.
3. **Director's Cut / edetabel / broadcast** — teatriekstra.
4. **Pakend** — release, PKGBUILD.

---

## 13. Projektistruktuur

```
gambiit/
├─ Cargo.toml                 # workspace
├─ crates/
│  ├─ gambiit-core/           # reeglid, mänguseis, kellad, PGN, undo
│  ├─ gambiit-engine/         # Stockfish UCI
│  ├─ gambiit-agent/          # OpenCode prompt + JSON + retry
│  └─ gambiit-cli/            # terminaliprototüüp
├─ src-tauri/                 # Tauri shell, käsud, sessioon
│  ├─ src/main.rs
│  ├─ tauri.conf.json
│  ├─ capabilities/
│  └─ icons/
├─ src/                       # Svelte UI
│  ├─ App.svelte
│  ├─ lib/Board.svelte
│  ├─ lib/PlayerBar.svelte
│  ├─ lib/MoveList.svelte
│  ├─ lib/Commentary.svelte
│  ├─ lib/SettingsPanel.svelte
│  ├─ lib/*.ts
│  ├─ locales/en.json         # UI + agendi keel
│  ├─ locales/et.json
│  └─ assets/pieces/cburnett/
├─ docs/
│  ├─ LOCALES.md              # uue keele lisamise juhend
│  ├─ ui-preview.png
│  ├─ ui-preview-et.png
│  └─ settings-preview.png
├─ PLAN.md                    # ingliskeelne/GitHubi plaan
└─ PLAN.et.md                 # see fail
```

Andmed (tulevikus):

```
~/.config/gambiit/settings.json
~/.local/share/gambiit/games/<uuid>.json
~/.local/share/gambiit/themes/
~/.local/share/gambiit/agent-workspace/
$XDG_RUNTIME_DIR/gambiit/session.json   # port + token, 0600
```

---

## 14. Testimine, riskid, turvalisus

### Testid

- **Core:** perft 1–5, SAN/FEN/UCI round-trip, castling, en passant, promotion,
  50-move, threefold, insufficient material, undo.
- **Engine:** Stockfishi info-parsimine.
- **Agent:** JSON proosa/fence'i sees, kandidaadid, puuduv move, retry.
- **Tauri:** snapshot, inimene/agent kord, takeback, resign, musta poole algus.
- **Ignored smoke:** päris mudeliga üks käik.
- **UI:** `svelte-check`, Vite build; Playwright/visuaalsed testid tulevikus.
- **Käsitsi:** päris mäng, teema vahetus, seaded, promotsioon, tagasivõtt.

### Riskid

| Risk | Mõju | Leevendus |
|---|---|---|
| Tauri/WebKitGTK probleemid Waylandil | UI ei käivitu | testitud; brauseri varuplaan `gambiit serve` tulevikus |
| LLM ebaseaduslik käik | mäng takerdub | serverivalideerimine, retry, paus/fallback |
| Mudel aeglane/kallis | halb kogemus | async UI, timeout, mudelivalik, kohalikud mudelid |
| MCP õigused | MCP-režiim ei tööta | prompt-režiim jääb põhiliseks; MCP eraldi test |
| Stockfish puudub | analüüs/hübriid ei tööta | tuvastus + paigaldusjuhis; legal-only fallback |
| Malendite/teemade litsentsid | juriidiline risk | CC0/CC BY eelistus; `ATTRIBUTION.md` |
| Lokaalne API kuritarvitamine | turvarisk | localhost + token + CORS + CSP |
| Kellad LLM-i vastu | ebaõiglane | vaikimisi kelladeta; mudeli mõtlemisaeg kulub agendi enda kellast |
| Skope paisub | projekt venib | MVP enne ekstra; väikesed vertikaalsed viilud |

### Turvalisus

- API ainult `127.0.0.1`, juhuslik token, fail 0600.
- CSP ja sanitiseeritud kommentaarid: agendi tekst renderdatakse plain text /
  markdownina, mitte `{@html}`-ina.
- PGN/teema import valideeritakse; suuruse piirid; ei mingit käivitatavat koodi.
- Agent-workspace'is keelatud bash/edit/webfetch; ainult `gambiit_*` tööriistad.
- Rakendus ei loe ega salvesta API-võtmeid; OpenCode haldab autentimist.
- Logid redigeerivad tokenid.

---

## 15. Minu ideed

1. **Tõeobjektiiv (Truth Lens)** — pärast mängu kõrvutab rakendus agendi
   väidetud hinnangud Stockfishi tegelike hinnangutega. Ainult objektiivselt
   kontrollitavad väited; tulevikuplaanid on lubatud bluff.
2. **Režissööri lõige (Director's Cut)** — pärast mängu genereerib agent PGN-i +
   pöördepunktide põhjal jutustuse; laudadega ajajoon, eksport Markdown/HTML.
3. **Mudelite edetabel** — kohalik W/L/D, täpsus, keskmine mõtlemisaeg,
   “trash-talk score” mudelite/isiksuste kaupa.
4. **Isiksusemikser** — liugurid: agressiivsus, pettus, sõnavõtt, draama,
   huumor, keel; preseedid + avatar/aktsent.
5. **Broadcast-režiim** — suur laud, kommentaariticker, peidetud paneelid;
   OBS-ile sobiv.
6. **Avangurolet** — juhuslik avang kureeritud raamatust või temaatiline matš.
7. **Handikapid** — agent ilma liputa, inimesele lisaaeg jne.
8. **Pressikonverents** — pärast mängu küsimused-vastused rollis.
9. **Grudge match** — sama mudel/isiksus mäletab eelmist mängu.
10. **Treenerirežiim** — aus Stockfishi + LLM analüüs.
11. **Kohalik TTS** — Piper loeb kommentaare; valikuline.
12. **Teemastuudio jagamine** — `.gambiit-theme` failid, galerii.

Top 5: Truth Lens, Director's Cut, isiksusemikser, mudelite edetabel,
broadcast-režiim.

---

## 16. Järgmised sammud

1. **Rohkem teemasid ja malendikomplekte**.
2. **MCP tööriistad vestlusele, viigile ja analüüsile**.
3. **Director's Cut / edetabel / broadcast**.
4. **Pakend** (PKGBUILD, `.desktop`, release).

Salvestamine, jätkamine, mängude raamatukogu, PGN eksport/import, pärastmängu
analüüs, teemaredaktor, kohandatud malendikomplektid, vestlus, viigipakkumised,
malekellad ja MCP-käigud on nüüd valmis.
