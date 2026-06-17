import { useEffect, useMemo, useState } from "react";

import { getGraph } from "../../lib/ipc";
import type { GraphData, GraphNode, NoteRef } from "../../lib/types";

type GraphViewProps = {
  onOpenNote: (note: NoteRef) => void;
};

type PositionedNode = GraphNode & {
  x: number;
  y: number;
};

const emptyGraph: GraphData = {
  nodes: [],
  links: [],
};

export function GraphView({ onOpenNote }: GraphViewProps) {
  const [graph, setGraph] = useState<GraphData>(emptyGraph);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;

    setLoading(true);
    setError(null);

    void getGraph()
      .then((data) => {
        if (alive) {
          setGraph({
            nodes: Array.isArray(data.nodes) ? data.nodes : [],
            links: Array.isArray(data.links) ? data.links : [],
          });
        }
      })
      .catch(() => {
        if (alive) {
          setError("Graf alınamadı.");
          setGraph(emptyGraph);
        }
      })
      .finally(() => {
        if (alive) {
          setLoading(false);
        }
      });

    return () => {
      alive = false;
    };
  }, []);

  const positioned = useMemo<PositionedNode[]>(() => {
    const columns = Math.max(1, Math.ceil(Math.sqrt(graph.nodes.length || 1)));
    const cellWidth = 150;
    const cellHeight = 112;

    return graph.nodes.map((node, index) => ({
      ...node,
      x: 78 + (index % columns) * cellWidth,
      y: 58 + Math.floor(index / columns) * cellHeight,
    }));
  }, [graph.nodes]);

  const nodeMap = useMemo(() => {
    return new Map(positioned.map((node) => [node.id, node]));
  }, [positioned]);

  const linkCounts = useMemo(() => {
    const counts = new Map<string, number>();

    for (const link of graph.links) {
      counts.set(link.source, (counts.get(link.source) ?? 0) + 1);
      counts.set(link.target, (counts.get(link.target) ?? 0) + 1);
    }

    return counts;
  }, [graph.links]);

  const width = Math.max(620, Math.max(...positioned.map((node) => node.x), 0) + 90);
  const height = Math.max(360, Math.max(...positioned.map((node) => node.y), 0) + 90);

  return (
    <section className="task-panel graph-panel" aria-labelledby="graph-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">Graf</p>
          <h1 id="graph-title">Bağlantılar</h1>
        </div>
        <span className="badge accent">Faz 4'te kuvvet-yönelimli</span>
      </header>

      {loading ? <p className="notice">Graf yükleniyor...</p> : null}
      {error ? <p className="notice error">{error}</p> : null}

      {!loading && positioned.length === 0 ? (
        <p className="empty-state">Graf boş.</p>
      ) : (
        <div className="graph-stage" aria-label="Not grafiği">
          <svg role="img" viewBox={`0 0 ${width} ${height}`}>
            {graph.links.map((link) => {
              const source = nodeMap.get(link.source);
              const target = nodeMap.get(link.target);

              if (!source || !target) {
                return null;
              }

              return (
                <line
                  className="graph-link"
                  key={`${link.source}->${link.target}`}
                  x1={source.x}
                  x2={target.x}
                  y1={source.y}
                  y2={target.y}
                />
              );
            })}

            {positioned.map((node) => (
              <g
                className="graph-node"
                key={node.id}
                onClick={() => onOpenNote({ path: node.id, title: node.title })}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    onOpenNote({ path: node.id, title: node.title });
                  }
                }}
                role="button"
                tabIndex={0}
              >
                <circle className={node.dangling ? "is-dangling" : ""} cx={node.x} cy={node.y} r="19" />
                <text x={node.x} y={node.y + 37}>
                  {node.title}
                </text>
              </g>
            ))}
          </svg>
        </div>
      )}

      <div className="node-list" aria-label="Graf düğümleri">
        {positioned.map((node) => (
          <button
            className="node-list-item"
            key={node.id}
            onClick={() => onOpenNote({ path: node.id, title: node.title })}
            type="button"
          >
            <span>{node.title}</span>
            <span className="badge">{linkCounts.get(node.id) ?? 0} bağlantı</span>
          </button>
        ))}
      </div>
    </section>
  );
}
