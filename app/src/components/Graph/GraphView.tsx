import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ForceGraph2D from "react-force-graph-2d";
import type { NodeObject } from "react-force-graph-2d";

import { getGraph } from "../../lib/ipc";
import type { GraphData, GraphLink, GraphNode, NoteRef } from "../../lib/types";

type GraphViewProps = {
  onOpenNote: (note: NoteRef) => void;
};

type GraphCanvasNode = GraphNode & {
  id: string;
  title: string;
  dangling: boolean;
};

type GraphCanvasLink = GraphLink & {
  source: string;
  target: string;
};

type GraphSize = {
  width: number;
  height: number;
};

const emptyGraph: GraphData = {
  nodes: [],
  links: [],
};

const fallbackSize: GraphSize = {
  width: 720,
  height: 420,
};

function getCssVar(name: string, fallback: string) {
  if (typeof window === "undefined") {
    return fallback;
  }

  const value = window
    .getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();

  return value || fallback;
}

export function GraphView({ onOpenNote }: GraphViewProps) {
  const stageRef = useRef<HTMLDivElement | null>(null);
  const requestIdRef = useRef(0);
  const [graph, setGraph] = useState<GraphData>(emptyGraph);
  const [size, setSize] = useState<GraphSize>(fallbackSize);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchGraph = useCallback(async () => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    setLoading(true);
    setError(null);

    try {
      const data = await getGraph();

      if (requestIdRef.current !== requestId) {
        return;
      }

      setGraph({
        nodes: Array.isArray(data.nodes) ? data.nodes : [],
        links: Array.isArray(data.links) ? data.links : [],
      });
    } catch {
      if (requestIdRef.current !== requestId) {
        return;
      }

      setError("Graf alınamadı.");
      setGraph(emptyGraph);
    } finally {
      if (requestIdRef.current === requestId) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void fetchGraph();

    return () => {
      requestIdRef.current += 1;
    };
  }, [fetchGraph]);

  useEffect(() => {
    const stage = stageRef.current;

    if (!stage) {
      return undefined;
    }

    const resize = () => {
      const rect = stage.getBoundingClientRect();

      setSize({
        width: Math.max(320, Math.floor(rect.width)),
        height: Math.max(320, Math.floor(rect.height)),
      });
    };

    resize();

    const observer = new ResizeObserver(resize);
    observer.observe(stage);

    return () => observer.disconnect();
  }, []);

  const graphData = useMemo(() => {
    const nodes: GraphCanvasNode[] = graph.nodes.map((node) => ({
      id: node.id,
      title: node.title,
      dangling: node.dangling,
    }));
    const links: GraphCanvasLink[] = graph.links.map((link) => ({
      source: link.source,
      target: link.target,
    }));

    return { nodes, links };
  }, [graph]);

  const colors = useMemo(
    () => ({
      dangling: "#6b7280",
      link: "rgba(167, 167, 180, 0.22)",
      normal: getCssVar("--accent", "#7c6cff"),
    }),
    [],
  );

  const handleNodeClick = useCallback(
    (node: NodeObject<GraphCanvasNode>) => {
      onOpenNote({ path: node.id, title: node.title });
    },
    [onOpenNote],
  );

  const hasGraph = graphData.nodes.length > 0;

  return (
    <section className="task-panel graph-panel" aria-labelledby="graph-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">Graf</p>
          <h1 id="graph-title">Bağlantılar</h1>
        </div>
        <div className="toolbar graph-toolbar" aria-label="Graf işlemleri">
          <span className="badge">
            {graphData.nodes.length} düğüm · {graphData.links.length} bağlantı
          </span>
          <button className="button" disabled={loading} onClick={() => void fetchGraph()} type="button">
            {loading ? "Yükleniyor" : "Yenile"}
          </button>
        </div>
      </header>

      {error ? <p className="notice error">{error}</p> : null}

      <div className="graph-stage" ref={stageRef} aria-label="Not grafiği">
        {loading && !hasGraph ? <p className="graph-overlay">Graf yükleniyor...</p> : null}
        {!loading && !hasGraph ? <p className="graph-overlay">Graf boş.</p> : null}
        {hasGraph ? (
          <ForceGraph2D<GraphCanvasNode, GraphCanvasLink>
            backgroundColor="rgba(0,0,0,0)"
            cooldownTicks={100}
            graphData={graphData}
            height={size.height}
            linkColor={() => colors.link}
            linkWidth={1}
            nodeColor={(node) => (node.dangling ? colors.dangling : colors.normal)}
            nodeLabel={(node) => node.title}
            nodeRelSize={5.4}
            onNodeClick={handleNodeClick}
            showPointerCursor
            width={size.width}
          />
        ) : null}
      </div>
    </section>
  );
}
