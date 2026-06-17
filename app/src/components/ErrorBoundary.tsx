import { Component, type ErrorInfo, type ReactNode } from "react";

type Props = {
  children: ReactNode;
  /** key değişince (ör. görünüm değişimi) boundary sıfırlanır */
  resetKey?: string | number;
};

type State = { error: Error | null };

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
      return (
        <div className="error-fallback" role="alert">
          <h2>Bir şeyler ters gitti</h2>
          <p>Bu bölüm beklenmedik bir hatayla karşılaştı. Başka bir sekmeye geçebilir veya yeniden deneyebilirsin.</p>
          <pre className="error-detail">{String(this.state.error?.message ?? this.state.error)}</pre>
          <div className="error-actions">
            <button className="button primary" onClick={() => this.setState({ error: null })} type="button">
              Yeniden dene
            </button>
            <button className="button" onClick={() => window.location.reload()} type="button">
              Uygulamayı yenile
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
