import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

// AI çıktısını düz <pre> yerine zengin markdown olarak render eder
// (başlıklar, listeler, kod blokları, tablolar) → profesyonel görünüm.
export function MarkdownView({ text }: { text: string }) {
  return (
    <div className="markdown-body">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{text}</ReactMarkdown>
    </div>
  );
}
