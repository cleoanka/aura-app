import { Component, type ErrorInfo, type ReactNode } from "react";

import { translate, type Lang } from "../i18n";

type Props = {
  children: ReactNode;
  /** key değişince (ör. görünüm değişimi) boundary sıfırlanır */
  resetKey?: string | number;
};

type State = { error: Error | null };

// Class component hook kullanamaz → dil localStorage'dan okunur, çeviri saf translate() ile.
function currentLang(): Lang {
  try {
    const saved = localStorage.getItem("aura.lang");
    if (saved === "en" || saved === "tr") {
      return saved;
    }
  } catch {
    /* yoksay */
  }
  return "tr";
}

// Bir görünüm çökerse TÜM uygulamayı karartmak yerine kurtarılabilir bir hata
// gösterir. Görünüm değişince (resetKey) otomatik sıfırlanır.
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("AURA view crashed:", error, info.componentStack);
  }

  componentDidUpdate(prev: Props) {
    if (prev.resetKey !== this.props.resetKey && this.state.error) {
      this.setState({ error: null });
    }
  }

  render() {
    if (this.state.error) {
      const lang = currentLang();
      return (
        <div className="error-fallback" role="alert">
          <h2>{translate(lang, "error.title")}</h2>
          <p>{translate(lang, "error.body")}</p>
          <pre className="error-detail">{String(this.state.error?.message ?? this.state.error)}</pre>
          <div className="error-actions">
            <button className="button primary" onClick={() => this.setState({ error: null })} type="button">
              {translate(lang, "error.retry")}
            </button>
            <button className="button" onClick={() => window.location.reload()} type="button">
              {translate(lang, "error.reload")}
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
