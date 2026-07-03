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
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
