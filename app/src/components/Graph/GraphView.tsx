import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ForceGraph2D from "react-force-graph-2d";
import type { ForceGraphMethods, NodeObject } from "react-force-graph-2d";

import { useI18n } from "../../i18n";
import { getGraph } from "../../lib/ipc";
import type { NoteRef } from "../../lib/types";

type GraphViewProps = {
  onOpenNote: (note: NoteRef) => void;
};

// The backend get_graph() now returns a richer, cross-file graph than the
// shared types in lib/types.ts describe (it adds `kind` to nodes and links).
// We model that shape locally so this component stays type-clean without
// touching the shared types module.
type RawNode = {
  id: string;
  title?: string;
  kind?: string;
  dangling?: boolean;
};

type RawLink = {
  source: string;
  target: string;
  kind?: string;
};

type RawGraph = {
  nodes: RawNode[];
  links: RawLink[];
};

type NodeType = "markdown" | "code" | "config" | "binary" | "external" | "dangling";

type CanvasNode = {
  id: string;
  title: string;
  type: NodeType;
  degree: number;
  // injected by d3-force at runtime
  x?: number;
  y?: number;
};

type CanvasLink = {
  source: string;
  target: string;
  kind: string;
};

type GraphSize = {
  width: number;
  height: number;
};

type ForceMethods = ForceGraphMethods<CanvasNode, CanvasLink>;

const emptyGraph: RawGraph = { nodes: [], links: [] };

const fallbackSize: GraphSize = { width: 720, height: 480 };

// Calm, Obsidian-like palette. Tuned to the dark theme but readable on light.
const TYPE_COLORS: Record<NodeType, string> = {
  markdown: "#8f8cf5", // accent / violet
  code: "#4ea1ff", // blue
  config: "#3fcbb0", // teal
  binary: "#8b8f9a", // gray
  external: "#e8bf69", // amber
  dangling: "#6b7280", // muted gray
};

// Subtle per-kind link tints.
const LINK_COLORS: Record<string, string> = {
  import: "rgba(78, 161, 255, 0.30)",
  include: "rgba(63, 203, 176, 0.30)",
  use: "rgba(78, 161, 255, 0.26)",
  wikilink: "rgba(143, 140, 245, 0.34)",
  mdlink: "rgba(143, 140, 245, 0.28)",
  mention: "rgba(167, 167, 180, 0.22)",
};

const DEFAULT_LINK_COLOR = "rgba(167, 167, 180, 0.20)";

const CONFIG_EXTS = new Set([
  "json",
  "yaml",
  "yml",
  "toml",
  "ini",
  "cfg",
  "conf",
  "env",
  "lock",
  "xml",
]);

const CODE_EXTS = new Set([
  "py",
  "rs",
  "ts",
  "tsx",
  "js",
  "jsx",
  "mjs",
  "cjs",
  "c",
  "h",
  "hpp",
  "cpp",
  "cc",
  "cs",
  "go",
  "java",
  "kt",
  "swift",
  "rb",
  "php",
  "sh",
  "bash",
  "zsh",
  "lua",
  "sql",
  "scala",
  "dart",
  "vue",
  "svelte",
]);

const MARKDOWN_EXTS = new Set(["md", "markdown", "mdx", "txt", "rst"]);

const BINARY_EXTS = new Set([
  "o",
  "a",
  "so",
  "dll",
  "dylib",
  "exe",
  "bin",
  "png",
  "jpg",
  "jpeg",
  "gif",
  "webp",
  "svg",
  "ico",
  "pdf",
  "zip",
  "tar",
  "gz",
  "wasm",
  "ttf",
  "woff",
  "woff2",
]);

function extOf(id: string): string {
  const base = id.split(/[\\/]/).pop() ?? id;
  const dot = base.lastIndexOf(".");
  if (dot <= 0) {
    return "";
  }
  return base.slice(dot + 1).toLowerCase();
}

function deriveType(node: RawNode): NodeType {
  if (node.dangling) {
    return "dangling";
  }

  const kind = (node.kind ?? "").toLowerCase();
  if (kind === "external") {
    return "external";
  }
  if (kind === "markdown") {
    return "markdown";
  }
  if (kind === "binary") {
    return "binary";
  }

  const ext = extOf(node.id);

  if (MARKDOWN_EXTS.has(ext)) {
    return "markdown";
  }
  if (CONFIG_EXTS.has(ext)) {
    return "config";
  }
  if (CODE_EXTS.has(ext)) {
    return "code";
  }
  if (BINARY_EXTS.has(ext)) {
    return "binary";
  }

  // Fall back to the backend-provided kind hints.
  if (kind === "code") {
    return "code";
  }
  if (kind === "text") {
    return "markdown";
  }

  return "binary";
}

function titleOf(node: RawNode): string {
  if (node.title && node.title.trim().length > 0) {
    return node.title;
  }
  const base = node.id.split(/[\\/]/).pop();
  return base && base.length > 0 ? base : node.id;
}

function hexWithAlpha(hex: string, alpha: number): string {
  const clean = hex.replace("#", "");
  if (clean.length !== 6) {
    return hex;
  }
  const r = parseInt(clean.slice(0, 2), 16);
  const g = parseInt(clean.slice(2, 4), 16);
  const b = parseInt(clean.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

const LEGEND_TYPES: { type: NodeType; key: string }[] = [
  { type: "markdown", key: "graph.kind.markdown" },
  { type: "code", key: "graph.kind.code" },
  { type: "binary", key: "graph.kind.binary" },
  { type: "dangling", key: "graph.kind.dangling" },
];

export function GraphView({ onOpenNote }: GraphViewProps) {
  const { t } = useI18n();
  const stageRef = useRef<HTMLDivElement | null>(null);
  const graphRef = useRef<ForceMethods | undefined>(undefined);
  const requestIdRef = useRef(0);

  const [raw, setRaw] = useState<RawGraph>(emptyGraph);
  const [size, setSize] = useState<GraphSize>(fallbackSize);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Controls
  const [controlsOpen, setControlsOpen] = useState(true);
  const [nodeScale, setNodeScale] = useState(1);
  const [linkDistance, setLinkDistance] = useState(60);
  const [showLabels, setShowLabels] = useState(true);
  const [hoverId, setHoverId] = useState<string | null>(null);

  const fetchGraph = useCallback(async () => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    setLoading(true);
    setError(null);

    try {
      const data = (await getGraph()) as unknown as RawGraph;

      if (requestIdRef.current !== requestId) {
        return;
      }

      setRaw({
        nodes: Array.isArray(data.nodes) ? data.nodes : [],
        links: Array.isArray(data.links) ? data.links : [],
      });
    } catch {
      if (requestIdRef.current !== requestId) {
        return;
      }
      setError(t("common.error"));
      setRaw(emptyGraph);
    } finally {
      if (requestIdRef.current === requestId) {
        setLoading(false);
      }
    }
  }, [t]);

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
    const degree = new Map<string, number>();
    for (const link of raw.links) {
      degree.set(link.source, (degree.get(link.source) ?? 0) + 1);
      degree.set(link.target, (degree.get(link.target) ?? 0) + 1);
    }

    const valid = new Set(raw.nodes.map((n) => n.id));

    const nodes: CanvasNode[] = raw.nodes.map((node) => ({
      id: node.id,
      title: titleOf(node),
      type: deriveType(node),
      degree: degree.get(node.id) ?? 0,
    }));

    const links: CanvasLink[] = raw.links
      .filter((link) => valid.has(link.source) && valid.has(link.target))
      .map((link) => ({
        source: link.source,
        target: link.target,
        kind: (link.kind ?? "mention").toLowerCase(),
      }));

    return { nodes, links };
  }, [raw]);

  // Set of neighbour ids of the hovered node (for highlight).
  const neighbors = useMemo(() => {
    if (!hoverId) {
      return null;
    }
    const set = new Set<string>([hoverId]);
    for (const link of graphData.links) {
      const s = typeof link.source === "string" ? link.source : (link.source as CanvasNode).id;
      const tg = typeof link.target === "string" ? link.target : (link.target as CanvasNode).id;
      if (s === hoverId) {
        set.add(tg);
      } else if (tg === hoverId) {
        set.add(s);
      }
    }
    return set;
  }, [hoverId, graphData.links]);

  // Wire d3 forces from the sliders + a warm, brain-like clustering layout.
  const applyForces = useCallback(() => {
    const fg = graphRef.current;
    if (!fg) {
      return;
    }

    const linkForce = fg.d3Force("link");
    if (linkForce && typeof (linkForce as { distance?: unknown }).distance === "function") {
      (linkForce as unknown as { distance: (d: number) => unknown }).distance(linkDistance);
    }

    const chargeForce = fg.d3Force("charge");
    if (chargeForce && typeof (chargeForce as { strength?: unknown }).strength === "function") {
      // Stronger repulsion as link distance grows -> open, organic clusters.
      (chargeForce as unknown as { strength: (s: number) => unknown }).strength(
        -(40 + linkDistance * 1.6),
      );
    }

    const centerForce = fg.d3Force("center");
    if (centerForce && typeof (centerForce as { strength?: unknown }).strength === "function") {
      (centerForce as unknown as { strength: (s: number) => unknown }).strength(0.05);
    }

    fg.d3ReheatSimulation();
  }, [linkDistance]);

  useEffect(() => {
    applyForces();
  }, [applyForces, graphData]);

  const handleNodeClick = useCallback(
    (node: NodeObject<CanvasNode>) => {
      onOpenNote({ path: node.id, title: node.title });
    },
    [onOpenNote],
  );

  const handleFit = useCallback(() => {
    graphRef.current?.zoomToFit(600, 60);
  }, []);

  const handleEngineStop = useCallback(() => {
    graphRef.current?.zoomToFit(600, 60);
  }, []);

  const nodeRelSize = 4 * nodeScale;

  const nodeVal = useCallback(
    (node: NodeObject<CanvasNode>) => 1 + Math.sqrt(node.degree) * 1.6,
    [],
  );

  const nodeColor = useCallback(
    (node: NodeObject<CanvasNode>) => {
      const base = TYPE_COLORS[node.type];
      if (neighbors && !neighbors.has(node.id)) {
        return hexWithAlpha(base, 0.18);
      }
      return base;
    },
    [neighbors],
  );

  const linkColor = useCallback(
    (link: NodeObject<CanvasLink>) => {
      const c = LINK_COLORS[link.kind] ?? DEFAULT_LINK_COLOR;
      if (neighbors) {
        const s = typeof link.source === "string" ? link.source : (link.source as CanvasNode)?.id;
        const tg = typeof link.target === "string" ? link.target : (link.target as CanvasNode)?.id;
        const active = (s && neighbors.has(s)) || (tg && neighbors.has(tg));
        return active ? c : "rgba(167, 167, 180, 0.06)";
      }
      return c;
    },
    [neighbors],
  );

  const drawNode = useCallback(
    (
      node: NodeObject<CanvasNode>,
      ctx: CanvasRenderingContext2D,
      globalScale: number,
    ) => {
      const x = node.x ?? 0;
      const y = node.y ?? 0;
      const r = (1 + Math.sqrt(node.degree) * 1.6) * nodeRelSize * 0.5;
      const base = TYPE_COLORS[node.type];
      const dimmed = neighbors ? !neighbors.has(node.id) : false;
      const focused = hoverId === node.id;

      // soft glow — sadece odaklı/önemli düğümler için (her-frame CPU'yu düşürür)
      if (focused || node.degree >= 6) {
        ctx.beginPath();
        ctx.arc(x, y, r * 1.9, 0, 2 * Math.PI);
        ctx.fillStyle = hexWithAlpha(base, dimmed ? 0.04 : focused ? 0.28 : 0.14);
        ctx.fill();
      }

      // node body
      ctx.beginPath();
      ctx.arc(x, y, r, 0, 2 * Math.PI);
      ctx.fillStyle = dimmed ? hexWithAlpha(base, 0.22) : base;
      ctx.fill();

      if (focused) {
        ctx.lineWidth = 1.5 / globalScale;
        ctx.strokeStyle = "rgba(255,255,255,0.9)";
        ctx.stroke();
      }

      // labels: when enabled and either zoomed in enough, hovered, or a hub
      const showThis =
        showLabels &&
        !dimmed &&
        (focused || globalScale > 1.4 || node.degree >= 4);

      if (showThis) {
        const fontSize = Math.max(9, 11 / globalScale);
        ctx.font = `${fontSize}px -apple-system, system-ui, sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "top";
        ctx.fillStyle = focused ? "#ececf1" : "rgba(236, 236, 241, 0.78)";
        ctx.fillText(node.title, x, y + r + 2 / globalScale);
      }
    },
    [neighbors, hoverId, nodeRelSize, showLabels],
  );

  const counts = useMemo(() => {
    const byType: Record<NodeType, number> = {
      markdown: 0,
      code: 0,
      config: 0,
      binary: 0,
      external: 0,
      dangling: 0,
    };
    for (const node of graphData.nodes) {
      byType[node.type] += 1;
    }
    return byType;
  }, [graphData.nodes]);

  const hasGraph = graphData.nodes.length > 0;

  return (
    <section className="task-panel graph-panel" aria-labelledby="graph-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">{t("nav.graph")}</p>
          <h1 id="graph-title">{t("graph.title")}</h1>
        </div>
        <div className="toolbar graph-toolbar" aria-label={t("graph.title")}>
          <span className="badge">
            {graphData.nodes.length} {t("graph.nodes")} · {graphData.links.length}{" "}
            {t("graph.links")}
          </span>
          <button
            className="button"
            disabled={loading}
            onClick={() => void fetchGraph()}
            type="button"
          >
            {loading ? t("common.loading") : t("graph.refresh")}
          </button>
        </div>
      </header>

      {error ? <p className="notice error">{error}</p> : null}

      <div
        className="graph-stage"
        ref={stageRef}
        aria-label={t("graph.title")}
        style={{ position: "relative" }}
      >
        {loading && !hasGraph ? (
          <p className="graph-overlay">{t("common.loading")}</p>
        ) : null}
        {!loading && !hasGraph ? (
          <p className="graph-overlay">{t("graph.empty")}</p>
        ) : null}

        {hasGraph ? (
          <ForceGraph2D<CanvasNode, CanvasLink>
            ref={graphRef as never}
            backgroundColor="rgba(0,0,0,0)"
            graphData={graphData}
            width={size.width}
            height={size.height}
            cooldownTicks={70}
            cooldownTime={6000}
            warmupTicks={0}
            d3AlphaDecay={0.03}
            d3VelocityDecay={0.4}
            minZoom={0.2}
            maxZoom={8}
            nodeRelSize={nodeRelSize}
            nodeVal={nodeVal}
            nodeColor={nodeColor}
            nodeLabel={(node) => `${node.title}  ·  ${node.type}`}
            nodeCanvasObject={drawNode}
            nodePointerAreaPaint={(node, color, ctx) => {
              const r =
                (1 + Math.sqrt(node.degree) * 1.6) * nodeRelSize * 0.5 + 2;
              ctx.fillStyle = color;
              ctx.beginPath();
              ctx.arc(node.x ?? 0, node.y ?? 0, r, 0, 2 * Math.PI);
              ctx.fill();
            }}
            linkColor={linkColor}
            linkWidth={(link) =>
              neighbors ? 1.4 : link.kind === "wikilink" ? 1.1 : 0.8
            }
            linkDirectionalParticles={neighbors ? 2 : 0}
            linkDirectionalParticleWidth={1.6}
            linkDirectionalParticleColor={() => "rgba(143,140,245,0.7)"}
            onNodeClick={handleNodeClick}
            onNodeHover={(node) => setHoverId(node ? node.id : null)}
            onEngineStop={handleEngineStop}
            showPointerCursor
          />
        ) : null}

        {hasGraph ? (
          <div className={`graph-controls${controlsOpen ? "" : " is-collapsed"}`}>
            <button
              className="graph-controls-head"
              type="button"
              onClick={() => setControlsOpen((open) => !open)}
              aria-expanded={controlsOpen}
            >
              <span>{t("graph.controls")}</span>
              <span aria-hidden="true">{controlsOpen ? "▾" : "▸"}</span>
            </button>

            {controlsOpen ? (
              <div className="graph-controls-body">
                <label className="graph-control">
                  <span>{t("graph.nodeSize")}</span>
                  <input
                    type="range"
                    min={0.5}
                    max={2.5}
                    step={0.1}
                    value={nodeScale}
                    onChange={(e) => setNodeScale(Number(e.target.value))}
                  />
                </label>

                <label className="graph-control">
                  <span>{t("graph.linkDistance")}</span>
                  <input
                    type="range"
                    min={20}
                    max={160}
                    step={5}
                    value={linkDistance}
                    onChange={(e) => setLinkDistance(Number(e.target.value))}
                  />
                </label>

                <label className="graph-control graph-control-toggle">
                  <span>{t("graph.showLabels")}</span>
                  <input
                    type="checkbox"
                    checked={showLabels}
                    onChange={(e) => setShowLabels(e.target.checked)}
                  />
                </label>

                <button
                  className="button ghost graph-fit"
                  type="button"
                  onClick={handleFit}
                >
                  {t("graph.fit")}
                </button>

                <div className="graph-legend">
                  {LEGEND_TYPES.map(({ type, key }) => (
                    <div className="graph-legend-row" key={type}>
                      <span
                        className="graph-legend-dot"
                        style={{ background: TYPE_COLORS[type] }}
                      />
                      <span className="graph-legend-label">{t(key)}</span>
                      <span className="graph-legend-count">{counts[type]}</span>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}
          </div>
        ) : null}

        <style>{controlsCss}</style>
      </div>
    </section>
  );
}

// Scoped styling for the floating controls panel. Kept inline so the change is
// fully contained in this single component file.
const controlsCss = `
.graph-controls {
  position: absolute;
  top: 12px;
  right: 12px;
  z-index: 3;
  width: 220px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: color-mix(in srgb, var(--bg-secondary) 88%, var(--bg));
  box-shadow: var(--shadow);
  backdrop-filter: blur(8px);
  overflow: hidden;
}
.graph-controls.is-collapsed {
  width: auto;
}
.graph-controls-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  width: 100%;
  padding: 9px 12px;
  border: 0;
  background: transparent;
  color: var(--text);
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  cursor: pointer;
}
.graph-controls-head:hover {
  background: var(--bg-tertiary);
}
.graph-controls-body {
  display: grid;
  gap: 12px;
  padding: 10px 12px 14px;
  border-top: 1px solid var(--border);
}
.graph-control {
  display: grid;
  gap: 6px;
  color: var(--text-muted);
  font-size: 12px;
  font-weight: 600;
}
.graph-control input[type="range"] {
  width: 100%;
  accent-color: var(--accent);
  cursor: pointer;
}
.graph-control-toggle {
  grid-auto-flow: column;
  grid-template-columns: 1fr auto;
  align-items: center;
}
.graph-control-toggle input[type="checkbox"] {
  width: 16px;
  height: 16px;
  accent-color: var(--accent);
  cursor: pointer;
}
.graph-fit {
  width: 100%;
  min-height: 30px;
  padding: 5px 10px;
  font-size: 13px;
}
.graph-legend {
  display: grid;
  gap: 6px;
  margin-top: 2px;
  padding-top: 10px;
  border-top: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
}
.graph-legend-row {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-muted);
}
.graph-legend-dot {
  width: 9px;
  height: 9px;
  border-radius: 999px;
}
.graph-legend-count {
  color: var(--text);
  font-variant-numeric: tabular-nums;
  font-size: 11px;
  opacity: 0.7;
}
`;
