# AURA Desktop — v2.1 Mimari Plan (red-team sonrası, okuman için)

> macOS Apple Silicon (arm64) yerel-öncelikli, Obsidian-benzeri Markdown "ikinci beyin".
> Motor: mevcut `aura` CLI (claude=ana beyin, gemini=research, codex=impl). **Sıfırdan LLM entegrasyonu yok, motoru yeniden yazmıyoruz.**
> Felsefe: **user-friendly + bug-free** → az sayıda ölüm-kalım riskini önce kanıtla, gerisini sade tut, her şey ayarlardan configüre edilebilsin.

---

## 0. v2 taslağından NE DEĞİŞTİ (red-team + senin yeni şartların)

**Red-team (claude+codex) kesti:**
1. **`aura serve` daemon MVP'den ÇIKTI.** Uzun-ömürlü framed-IPC daemon = motoru yeniden yazmak. Yerine: her iş = kısa-ömürlü `zsh -lc "aura -p …"` süreci, Rust spawn eder, iptal = process-group kill, stream = stdout satır-satır. (Daemon ancak cold-start ölçümü gerektirirse gelir.)
2. **Auth gerçekten uygulama-içi:** gömülü **PTY paneli** (xterm.js + `portable-pty`) → `claude /login` app içinde, gerçek TTY, OAuth tarayıcıda, token doğru yere yazılır. "Harici terminal aç" sadece acil-fallback.
3. **Semantic cache → tam-eşleşme cache** (MVP): sıfır yanlış-pozitif. Embedding-yakınlık cache + eval fixture Faz 4+'a ertelendi.
4. **Faz 0 platform-riskine göre sıralandı:** #1 notarize+quarantine+token-görünürlüğü smoke, #2 cold-start latency. (Yazılım-içi spike'lar değil.)
5. **Kritik yol hafifledi:** embedding `Embedder` trait + **candle** ile başlar (mlx-rs ertelenir); graph layout **JS Web Worker** (backend-layout ertelenir); ACL sabit-string reçetelere kilitli, `osascript` çıktı.

**Senin yeni şartların eklendi:**
6. **İki yerel katman birden:** (a) yerel **embedding** (arama+cache), (b) yerel **üretim modeli = Lane 0** (Ollama/MLX, opsiyonel, configüre edilebilir).
7. **Bol opsiyon + Settings sistemi:** her şey ayarlardan (lane'ler, model/lane, eşikler, concurrency, cache modu, tema…). Zero-config başlar, her şey override edilebilir.
8. **aura-mode:** uygulama, şimdiki `aura` CLI gibi de çalışabilir — `plan / review / fix / ship` orkestrasyonu app içinden (not Q&A'dan ayrı, ikinci bir mod). §2.
9. **Consensus modu (opsiyon, VARSAYILAN KAPALI):** aynı görevi üç AI'a (claude+gemini+codex) yaptırıp **konsensus** üret. Settings'ten açılır. §2.4.
10. **Obsidian-benzeri güzel arayüz** birinci-sınıf hedef — ayrı UI/UX bölümü. §11.

---

## 1. Varsayımlar + Düzeltilmiş Teknoloji Yığını

**Varsayımlar**
- Hedef: macOS 26 (Tahoe sınıfı), Apple Silicon arm64. Universal binary yok.
- `aura` CLI saf-Python, kullanıcı `$PATH`'inde; alt-CLI auth'u kendi config/keychain'inde. AURA Desktop auth'u **saklamaz**, tetikler/doğrular.
- `aura serve` **yok ve MVP'de yazılmayacak**. `aura doctor --json` ise küçük net-yeni iş (Faz 1).

| Katman | Karar | "v1 → v2" gerekçe |
|---|---|---|
| Shell | Tauri v2 + Rust + React/TS/Vite | KORU |
| Subprocess | **tauri-plugin-shell + capabilities/ACL (sabit reçeteler)** | ham `std::process` → ACL'de sessiz patlar |
| **Çalıştırma modeli** | **per-job kısa-ömürlü `zsh -lc "aura -p"`; daemon YOK** | daemon = motoru yeniden yazmak; multiplex zaten Rust'ta (her job kendi child'ı) |
| İptal | **process-group kill** (`setsid`/`kill(-pgid)`) + `kill_on_drop` | Python tarafında cancel kodu gerekmez |
| Embedding | **candle (Metal)**, `Embedder` trait arkasında; mlx-rs sonra benchmark | ONNX/CoreML notarization kâbusu; candle HF ekosistemi oturmuş |
| Embedding modeli | e5-small multilingual (384-dim) | TR/EN, küçük |
| **Yerel üretim (Lane 0)** | **Ollama (HTTP) veya MLX**, opsiyonel, Settings'ten | yeni: kotasız/offline basit işler |
| Vektör + skaler DB | **sqlite-vec (tek `.sqlite`)** | LanceDB Arrow ağırlığı yok; tek ACID dosya |
| Tam-metin | **SQLite FTS5** (aynı dosya) | saf vektör tam-kelime/kod/isim kaçırır |
| Füzyon | **RRF** (k≈60) | ölçek-bağımsız, tuning kolay |
| Cache (MVP) | **tam-eşleşme** (`hash(norm_query+retrieval_fingerprint+model_ver+vault_epoch)`) | sıfır false-positive |
| Markdown/graf | pulldown-cmark + wikilink regex + petgraph + JSON snapshot | KORU |
| Graf layout | **MVP: d3-force JS Web Worker**; backend-layout eşik aşılınca | erken optimizasyon olmasın |
| Editör | CodeMirror 6 | KORU |
| Watcher | notify + debounce + reconciliation scan | tek indexer aktörü |
| Auth login | **gömülü PTY (xterm.js + portable-pty)** | WebView TTY veremez; PTY verir |
| Paketleme | uv-bundle (gerekirse) + `codesign --options runtime` + notarize + staple | C-ext imza izolasyonu |

---

## 2. İki Mod + Lane Modeli + Consensus

Uygulamanın **iki etkileşim modu** var (Settings'ten varsayılan seçilir, ⌘K ile anlık geçiş):

- **Ask mode (ikinci beyin):** notların üzerinde RAG soru-cevap → cache → retrieval → lane'ler.
- **aura mode (orkestratör):** şimdiki `aura` CLI gibi → `plan / review / fix / ship` bir proje/repo üzerinde, app içinden. Aynı motor (`aura`), aynı güvenlik (dosya değişimi yalnız açık onayla, asla commit). Çıktı app'te diff/önizleme olarak gösterilir.

### 2.1–2.3 Lane akışı (her iki modda da geçerli)
Her AI isteği sıralı kapılardan geçer; **her kapı Settings'ten açılıp kapanır / model seçilir:**

```
query
 └─► [Cache] tam-eşleşme? ──evet──► cevap (0 token, anında)
        │ hayır
        ▼
 └─► [Retrieval] hybrid (vektör + FTS5 → RRF)  → context bundle
        ▼
 └─► [Router] karmaşıklık + agent durumu →
        ├─ Lane 0  : YEREL üretim (Ollama/MLX)   ← opsiyonel, varsayılan KAPALI
        ├─ Fast    : claude (varsayılan model)
        └─ Deep    : claude --model opus --effort xhigh (+gerekirse gemini research, codex)
        ▼
 └─► sonucu cache'e yaz (provenance ile) + UI'a stream
```

- **Lane 0 (yerel üretim):** Settings'te aç → basit özet/yeniden-yazma/soru buluta hiç gitmez. Model seçilebilir (Ollama listesi veya MLX). Kapalıysa router doğrudan Fast'e gider.
- **Lane düşürme:** claude rate-limited → Deep, Fast'e veya (açıksa) Lane 0'a düşer; UI'da nazik banner.
- Router eşikleri, hangi lane'in hangi iş için, concurrency tavanları — **hepsi Settings'te.**

### 2.4 Consensus modu (opsiyon, VARSAYILAN KAPALI)
Settings'ten "Consensus" açıkken (veya ⌘K → "Ask with consensus"): aynı görev **üç AI'a paralel** gönderilir (claude+gemini+codex), sonra:
1. Üç cevap toplanır (her biri kendi process'i, §5 modeliyle).
2. **claude (ana beyin) sentezler:** anlaşılan noktalar + çelişkiler + en güçlü cevabı seçip birleştirir → tek "consensus" sonucu + kaynak-katkı rozetleri.
3. UI'da üç ham cevap + sentez yan-yana gösterilir (şeffaflık).
- **Neden varsayılan kapalı:** 3× token/kota + 3× gecikme + sık limit (bugün gördük). Açık seçilince UI net "3 ajan, ~3× maliyet" rozeti gösterir.
- Çalışır: hem Ask hem aura modunda. `aura` CLI'ında zaten kanıtlanmış desen (bu plandaki workflow'lar tam da bunu yaptı).
- Ayarlanabilir: hangi ajanlar dahil, sentezleyici (varsayılan claude), oylama vs sentez, min-anlaşma eşiği.

---

## 3. Settings Sistemi ("olabildiğince çok opsiyon")

Tek kaynak: `~/Library/Application Support/aura-desktop/settings.json` (+ DB `meta`). **Zero-config başlar** (akıllı varsayılanlar), her şey override edilebilir, her grupta "Reset to default".

**Gruplar:**
- **Vault:** kök klasör(ler), dahil/hariç glob, dosya-izleme açık/kapalı.
- **Agents (Agent Manager):** claude/gemini/codex kurulu/auth/limit durumu; kurulum/login butonları; rol gösterimi (claude=ANA BEYİN).
- **Mod:** varsayılan mod (Ask / aura); aura-mode için proje kökü, dosya-yazma onay davranışı (asla otomatik commit).
- **Lanes:** her lane aç/kapa; Lane 0 yerel model seçimi (Ollama URL+model / MLX model yolu); Fast/Deep için model+effort override; router karmaşıklık eşiği; `--fast/--deep` zorlama.
- **Consensus (varsayılan KAPALI):** aç/kapa; dahil ajanlar (claude/gemini/codex); sentezleyici (varsayılan claude); oylama mı sentez mi; min-anlaşma eşiği; "her zaman 3× maliyet rozeti göster".
- **Retrieval:** top-k (vektör), top-k (FTS), RRF k, hierarchical chunk derinliği, hangi alanlar aranır.
- **Embedding:** model (e5-small / değiştir), batch boyutu, GPU/CPU, yeniden-indeksle butonu.
- **Cache:** mod = **off / exact (varsayılan) / semantic (deneysel, uyarılı)**; semantic eşik; TTL; "şüpheli hit'i göster + tek-tık invalidate".
- **Concurrency/Limits:** agent başına `max_concurrent`, kuyruk boyutu, per-job timeout/max-bytes, retry_after davranışı.
- **Güvenlik:** RAG prompt-injection koruması açık/kapalı (varsayılan açık), untrusted-content etiketleme katılığı.
- **UI:** tema (sistem/aydınlık/karanlık), font, panel düzeni, hotkey'ler (⌘K vb.), graph görünüm parametreleri.
- **Gelişmiş/Observability:** log seviyesi, trace paneli, `~/.aura/runs` aç, telemetry (varsayılan KAPALI, tamamen yerel).

> **Bug-free ilkesi:** settings şema-versiyonlu + doğrulanır; bozuk/eksik alan → o alan varsayılana düşer (uygulama asla bozuk-config'le çökmez). Her ayarın güvenli varsayılanı var.

---

## 4. Agent Manager (birinci sınıf) + somut AUTH çözümü

Amaç: claude/gemini/codex'i uygulamadan çıkmadan **algıla → kur → giriş yap → sağlık → limit izle**. claude = ana beyin, en üstte, "ANA BEYİN" rozeti.

### 4.1 Durum modeli
```
InstallState = NotInstalled | Installed{version,path}
AuthState    = Unknown | LoggedOut | LoggedIn{account?} | RateLimited{kind,retry_after?}
HealthState  = Ok | Degraded(reason) | Down(reason)
```

### 4.2 Environment Resolver (codex red-team — kritik)
`zsh -lc`'nin auth/env'i göreceği **garanti değil** (login vs interaktif farkı, nvm/asdf). Çözüm: **tek env-resolver** — ilk kurulumda PTY'de bir bootstrap çalışır, `env -0` + `which` + `aura doctor` çıktısını yakalar → **onaylı env snapshot** üretir. Sonraki tüm `aura`/alt-CLI çağrıları bu snapshot ile yapılır. App auth **yazmaz**, sadece bu snapshot ile **doğrular**.

### 4.3 Kurulum (sertleştirilmiş)
- Reçeteler **sabit string** (parametre enjeksiyonu yok): claude=`npm i -g @anthropic-ai/claude-code`, gemini=`@google/gemini-cli`, codex=`@openai/codex` (brew fallback).
- **Preflight:** node/npm/brew var mı, arch (arm64), writable prefix, proxy → hatayı önce göster.
- **Kabul kriteri:** sadece exit-code değil → `which --version` + `aura doctor` `can_invoke=true`.
- Uzun npm logları PTY/Channel ile stream.

### 4.4 Login = gömülü PTY paneli (gerçekten uygulama-içi)
- xterm.js (frontend) + Rust `portable-pty` → `claude /login` **app içindeki terminalde**; OAuth tarayıcıda açılır, token CLI'ın beklediği yere yazılır, kullanıcı app'ten çıkmaz.
- Önce **ampirik tespit:** her CLI token'ı NEREDE tutuyor (dosya `~/.config/...` mı, keychain mi)? Entitlement kararı buna göre. Default entitlements: yalnız `com.apple.security.inherit` (gereksiz `allow-unsigned-executable-memory` YOK).
- "Harici Terminal aç" sadece fallback.

### 4.5 Health + Limit
- `zsh -lc "aura doctor --json"` (env-snapshot ile) → her agent `{install,auth,can_invoke,last_error}`; Status Bar'a yansır.
- Limit: stderr pattern → `RateLimited{retry_after}`; concurrency tavanı + kuyruk; lane düşürme; UI sayaç + oto-temizleme.

---

## 5. Çalıştırma Modeli (daemon YOK)

- Her AI işi: Rust supervisor `zsh -lc "aura -p '<task>' --lane <l>"` (env-snapshot) **kısa-ömürlü** süreç olarak spawn eder; `setsid` ile yeni process-group.
- **Multiplex:** her job kendi child'ı → doğal izolasyon, framing/handshake gerekmez.
- **Stream:** child stdout satır-satır → Tauri Channel → UI.
- **İptal:** `kill(-pgid)` + `kill_on_drop` → claude/gemini/codex alt-süreçleri de ölür.
- **Lifecycle:** per-job timeout/max-bytes/max-runtime; stderr ayrı log; app-shutdown'da grup kill (zombie/orphan yok).
- Context bundle pipe yerine app-owned `0700` temp dosya (random ad, size limit, TTL, canonical-path kontrolü).

---

## 6. Veri Katmanı + Hibrit Arama + Cache (sağlamlaştırıldı)

### 6.1 `aura.sqlite` (WAL + busy_timeout)
`notes(path,mtime,content_hash,title)` · `chunks(id,note_path,parent_id,level,heading_path,text,chunk_stable_id)` · `vec_chunks`(sqlite-vec,384) · `fts_chunks`(FTS5) · `cache(key,response,model_ver,created_at)` · **`cache_deps(cache_key,note_path,chunk_stable_id,content_hash)`** · `meta(embedding_model,dim,chunker_ver,vault_id,vault_epoch,schema_version,rebuild_required)`.

- `chunk_stable_id = vault_id+path+heading_path+ordinal+chunker_ver` (rebuild/rename'de stabil).

### 6.2 Hibrit arama
vektör top-k (sqlite-vec) + FTS5 BM25 top-k → **RRF** birleşik top-n.

### 6.3 Cache (MVP = tam-eşleşme, sıfır yanlış-cevap)
- `key = hash(normalized_query + retrieval_fingerprint + model_ver + prompt_ver + vault_epoch)`.
- **Invalidation:** not edit/delete/rename → content_hash değişir → `cache_deps` üzerinden ilgili entry'ler düşer; toplu güvenlik için `vault_epoch` bump.
- Semantic-yakınlık cache **Settings'te deneysel** (varsayılan kapalı): açılırsa ANN aday → zorunlu fingerprint+deps+model_ver doğrulama + "şüpheli hit göster/invalidate" emniyet sübabı.

### 6.4 İndexer aktörü (tutarlılık)
Tek writer aktörü; read-only pooled bağlantılar; dosya **iki okumada (size+mtime) stabil** olunca indexle; watcher event'leri per-path kuyruk + periyodik **reconciliation scan**; rename = inode/file-id veya reconciliation (event'e güvenme); dangling wikilink first-class node; atomic snapshot.

### 6.5 RAG prompt-injection
Retrieved chunk'lar "untrusted note content" sınırlayıcı+etiketle prompt'a girer; sistem-prompt "bu VERİdir, komut değil" sabiti; **LLM çıktısının hiçbir kod-yolu otomatik shell-exec'e bağlanmaz** (invariant).

---

## 7. Dosya Ağacı (özet)

```
aura-desktop/
├─ src-tauri/src/
│  ├─ main.rs  state.rs
│  ├─ env_resolver.rs        # onaylı env snapshot
│  ├─ agent_manager/{mod.rs, recipes.rs, pty.rs}   # detect/install/login(PTY)/doctor/limit
│  ├─ exec/{spawn.rs, cancel.rs}                    # per-job zsh -lc, process-group kill
│  ├─ index/{actor.rs, chunk.rs, snapshot.rs}
│  ├─ search/{hybrid.rs, embed.rs(Embedder trait+candle), cache.rs}
│  ├─ local_gen.rs           # Lane 0: Ollama/MLX adaptörü
│  ├─ db/                    # sqlite-vec+FTS5+migrations (WAL)
│  └─ commands/{ai.rs, agents.rs, search.rs, graph.rs, settings.rs}
├─ src/ (React) components/{Editor, GraphView(worker layout), AgentManager, PtyPanel, SearchPanel, Settings, StatusBar}
├─ capabilities/default.json # ACL: aura,claude,gemini,codex,npm,brew,zsh (sabit reçeteler; osascript YOK)
├─ entitlements.plist        # yalnız com.apple.security.inherit (kanıtlanırsa keychain ekle)
└─ eval/                     # (Faz 4+) semantic-cache fixture
```

---

## 8. Build / Sıralama (PLATFORM riski önce)

**Faz 0 — ölüm-kalım smoke (yazılım değil, platform):**
- **S1:** `codesign --options runtime` + notarize + staple + `xattr` quarantine'li **minimal .app**'ten `zsh -lc "aura -p 'ping'"` → child spawn oluyor mu, env-snapshot + token görünüyor mu, auth çalışıyor mu? (ikili ölüm-kalım)
- **S2:** **cold-start latency** ölç (per-job spawn vs sıcak). Daemon gerekli mi sorusunu veriyle bağlar.

**Faz 1 — Agent Manager dikey dilim:** detect → install(preflight) → **PTY login** → `aura doctor --json` (aura'ya --json ekle) → UI kartları + limit banner + env-resolver.

**Faz 2 — Index + hibrit arama:** sqlite-vec+FTS5+RRF, indexer aktörü (WAL/reconciliation), candle embed (trait), watcher. (Embedding yoksa bile FTS5-only çalışır = graceful degradation.)

**Faz 3 — AI akışı + Lane 0:** cache(exact)→router→{Lane0 yerel / Fast / Deep} per-job spawn + stream + process-group cancel + error taksonomisi + observability (trace/latency). Settings paneli buraya bağlanır.

**Faz 4 — Cila + ertelenenler:** CodeMirror editör, GraphView (önce JS-worker layout), snapshot/migration UI, semantic-yakınlık cache (eval + emniyet sübabıyla), backend-layout (büyük vault), mlx-rs benchmark.

---

## 9. Riskler + Strateji (özet)

| Risk | Strateji |
|---|---|
| **Auth/token görünmezliği (notarized .app)** | Faz 0 S1 tam-zincir smoke; env-resolver snapshot; PTY login; token konumu ampirik; doctor her açılışta doğrular; göremezse deep kilit + net uyarı |
| **Rate/session limit** | concurrency tavanı+kuyruk; pattern→retry_after; lane düşürme (Deep→Fast→Lane0); UI sayaç |
| **Cold-start latency** | Faz 0 S2 ölç; gerekirse (ancak o zaman) sıcak süreç/daemon değerlendir |
| **Cache yanlış-cevap** | MVP tam-eşleşme (sıfır FP); semantic opt-in + emniyet sübabı |
| **Tauri ACL bloğu** | capabilities whitelist (sabit reçeteler) önce + entegrasyon testi |
| **Index tutarsızlığı** | tek aktör; WAL+busy_timeout; stabilite kontrolü; reconciliation; rename=inode |
| **RAG injection** | untrusted-content etiketi + invariant: LLM çıktısı → otomatik exec YOK |
| **mlx/backend-layout olgunluk** | kritik yoldan çıkar: candle + JS-worker ile başla, sonra değiştir |
| **Bozuk settings** | şema-versiyon + alan-bazlı varsayılana düşme; asla config'le çökme |

---

## 10. User-friendly + Bug-free garantileri
- **Zero-config açılır**, çalışır; her şey Settings'ten override edilebilir, her grupta "reset".
- Hiçbir yol **sessiz yanlış-cevap** üretmez (exact cache; semantic opt-in+uyarı).
- aura/model/index/Lane0 yokken **graf + FTS5 arama çalışmaya devam eder** (graceful degradation).
- Net **error taksonomisi** (config/model/index/sidecar/network/permission) + her hata kendi log yolunu söyler.
- Tüm riskli/yeni parçalar Faz 0'da kanıtlanır; cila en sona.

---

## 11. Arayüz / UX (Obsidian-benzeri, güzel)

Hedef: Obsidian kadar tanıdık ve şık, ama AI-native. Birinci-sınıf tasarım, sonradan eklenti değil.

- **Düzen:** sol **dosya/vault gezgini** + orta **editör** + sağ **bağlam paneli** (AI cevabı / backlinks / graph mini). Hepsi **ayrılabilir/sürüklenebilir paneller** (Obsidian workspace hissi), kaydedilebilir layout.
- **Editör:** CodeMirror 6, canlı Markdown önizleme, `[[wikilink]]` otomatik-tamamlama + hover-önizleme, ⌘-tık ile nota git.
- **Graph View:** Obsidian benzeri kuvvet-yönelimli graf; local-graph (aktif notun komşuları) + global-graph; düğüm tıkla→aç; renk = klasör/etiket; dangling link gri. (MVP: JS-worker layout; büyük vault'ta backend-layout.)
- **Command Palette (⌘K):** her şey buradan — ara, sor (Ask), aura komutu (plan/fix…), consensus aç, mod değiştir. Klavye-öncelikli.
- **AI cevap paneli:** streaming token akışı; üstte **lane rozeti** (cached / Lane0-local / fast / **deep** / **consensus 3×**); kaynak chunk'lara tıklanır atıf; "buna güvenme: şüpheli cache" uyarısı gerektiğinde.
- **Status Bar:** her agent için minik sağlık/auth/limit ışığı (claude=ANA BEYİN); indeksleme ilerlemesi; aktif mod.
- **Tema:** sistem/aydınlık/karanlık, Obsidian-uyumlu renk değişkenleri; akıcı, native hisli (Tauri webview + özenli CSS, gereksiz animasyon yok).
- **Onboarding:** ilk açılışta sade sihirbaz → vault seç → Agent Manager (üç AI'ı kur/login PTY) → "hazırsın". Zero-config çalışır.
- **Erişilebilirlik & bug-free his:** klavye navigasyonu, net boş-durumlar, her hata kullanıcı diliyle + "şunu yap" satırı, asla ham traceback.

---

### Sonraki adım
Bu planı oku. Onaylarsan **Faz 0 S1 (notarize+token-görünürlüğü smoke)** ile başlarım — çünkü tüm projenin ayakta kalıp kalmayacağını o belirler. "Başla" demeden hiçbir şey kurmam.
