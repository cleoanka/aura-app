# Araştırma — JS bundle code-split (2026-06-23)

## Gözlem
`vite build` tek chunk üretiyor: **index-*.js ≈ 1.57 MB (gzip 491 KB)**. Vite uyarı
veriyor (>500 KB). Desktop (Tauri, yerel dosya) için kritik değil ama:
- İlk parse/eval tek dev dosyada → pencere açılışında gereksiz iş.
- Değişen app kodu tüm vendor'ı invalidate ediyor (cache verimsiz).

## Plan
`build.rollupOptions.output.manualChunks` ile node_modules'ı **mantıksal vendor
aileleri**ne böl (sıra önemli — özel olanlar önce):
1. `graph` — react-force-graph + d3-force
2. `editor` — @codemirror / @uiw / @lezer
3. `term` — @xterm
4. `markdown` — react-markdown / remark / micromark / mdast / unist / hast
5. `react` — react / react-dom / scheduler
6. `vendor` — kalan

## Beklenen
Tek 1.57MB yerine ~6 paralel-yüklenebilir chunk; app kodu değişince vendor cache korunur. Toplam byte ~ aynı (gzip benzer).

## Risk & doğrulama
- RİSK: yanlış gruplama → boş/çift chunk ya da build hatası. Düşük (additive config).
- DOĞRULAMA (kapı): `npm run build` → tsc geçer + birden çok chunk + **tek chunk artık ~1.5MB değil**; `npm test` 10/10; soul_check ✅. Patlarsa rollback.

→ Döngü 16'da uygula.
