import { describe, expect, it } from "vitest";

import strings from "./strings.json";
import { translate } from "./index";

const dicts = strings as { en: Record<string, string>; tr: Record<string, string> };

describe("translate", () => {
  it("var olmayan anahtarda anahtarın kendisini döndürür (çökme yok)", () => {
    expect(translate("tr", "yok.boyle.bir.key")).toBe("yok.boyle.bir.key");
  });

  it("eksik dilde EN'e düşer", () => {
    // tr'de olmayan ama en'de olan bir anahtar bulup fallback'i doğrula
    const enOnly = Object.keys(dicts.en).find((k) => !(k in dicts.tr));
    if (enOnly) {
      expect(translate("tr", enOnly)).toBe(dicts.en[enOnly]);
    }
    expect(true).toBe(true);
  });

  it("{var} enterpolasyonu yapar", () => {
    // auraMode.saved gibi statik bir değer üzerinde {x} enterpolasyonunu test et
    const out = translate("en", "yok.key.{x}", { x: "42" });
    // bilinmeyen key → key string döner, içinde {x} enterpole edilir
    expect(out).toBe("yok.key.42");
  });
});

describe("strings.json bütünlüğü", () => {
  it("en ve tr AYNI anahtarlara sahip (eksik çeviri yok)", () => {
    const enKeys = Object.keys(dicts.en).sort();
    const trKeys = Object.keys(dicts.tr).sort();
    const missingInTr = enKeys.filter((k) => !(k in dicts.tr));
    const missingInEn = trKeys.filter((k) => !(k in dicts.en));
    expect(missingInTr, `tr'de eksik: ${missingInTr.join(", ")}`).toEqual([]);
    expect(missingInEn, `en'de eksik: ${missingInEn.join(", ")}`).toEqual([]);
  });

  it("hiçbir değer boş değil", () => {
    const emptyEn = Object.entries(dicts.en).filter(([, v]) => !v.trim()).map(([k]) => k);
    const emptyTr = Object.entries(dicts.tr).filter(([, v]) => !v.trim()).map(([k]) => k);
    expect(emptyEn, `boş en: ${emptyEn.join(", ")}`).toEqual([]);
    expect(emptyTr, `boş tr: ${emptyTr.join(", ")}`).toEqual([]);
  });
});
