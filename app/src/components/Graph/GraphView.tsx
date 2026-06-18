import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ForceGraph2D from "react-force-graph-2d";
import type { ForceGraphMethods, NodeObject } from "react-force-graph-2d";

import { useI18n } from "../../i18n";
import { getGraph } from "../../lib/ipc";
import type { NoteRef } from "../../lib/types";

type GraphViewProps = {
  onOpenNote: (note: NoteRef) => void;
  activePath?: string | null;
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

type ColorMode = "type" | "folder";
type SearchMode = "highlight" | "filter";
type Scope = "global" | "local";

type CanvasNode = {
  id: string;
  title: string;
  type: NodeType;
  folder: string;
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

type ContextMenu = {
  x: number;
  y: number;
  nodeId: string;
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

// ~10 distinct hues for folder-based grouping. Stable order so a given folder
// always maps to the same color across renders.
const FOLDER_PALETTE = [
  "#8f8cf5",
  "#4ea1ff",
  "#3fcbb0",
  "#e8bf69",
  "#f57aa0",
  "#7ad17a",
  "#c98cf5",
  "#f59e4e",
  "#5ad1d1",
  "#d1d15a",
];

const FOLDER_FALLBACK = "#8b8f9a";

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

// Alpha used to dim nodes/links that are not "active" (non-neighbours, search
// non-matches in highlight mode).
const DIM_ALPHA = 0.18;

// Cap on the number of labels drawn at high zoom (top-by-degree).
const MAX_LABELS = 60;

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

// Parent directory of an id path. "" when the id has no directory part.
function folderOf(id: string): string {
  const norm = id.replace(/\\/g, "/");
  const slash = norm.lastIndexOf("/");
  if (slash <= 0) {
    return "";
  }
  return norm.slice(0, slash);
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

// d3-force mutates `source`/`target` into node objects after the sim runs, so a
// link endpoint may be either an id string or a node object. Normalise it.
function linkEndId(e: string | { id?: string } | undefined): string {
  if (typeof e === "string") {
    return e;
  }
  return e?.id ?? "";
}

const LEGEND_TYPES: { type: NodeType; key: string }[] = [
  { type: "markdown", key: "graph.kind.markdown" },
  { type: "code", key: "graph.kind.code" },
  { type: "binary", key: "graph.kind.binary" },
  { type: "dangling", key: "graph.kind.dangling" },
];

export function GraphView({ onOpenNote, activePath }: GraphViewProps) {
  const { t } = useI18n();
  const stageRef = useRef<HTMLDivElement | null>(null);
  const graphRef = useRef<ForceMethods | undefined>(undefined);
  const requestIdRef = useRef(0);
  // Tracks whether the active-note centering has happened for the current
  // selection — coords are undefined until the sim settles, so we retry inside
  // the engine-stop handler.
  const centeredRef = useRef<string | null>(null);

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

  // Feature state
  const [colorBy, setColorBy] = useState<ColorMode>("type");
  const [query, setQuery] = useState("");
  const [searchMode, setSearchMode] = useState<SearchMode>("highlight");
  const [scope, setScope] = useState<Scope>("global");
  const [hops, setHops] = useState(1);
  const [localRootId, setLocalRootId] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [menu, setMenu] = useState<ContextMenu | null>(null);
  const [running, setRunning] = useState(true);

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

  // Briefly reheat the simulation. Called on any change that affects layout.
  const reheat = useCallback(() => {
    setRunning(true);
    graphRef.current?.d3ReheatSimulation();
  }, []);

  // Full graph (degree, type, folder) before any scope/search filtering.
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
      folder: folderOf(node.id),
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

  // ---- Adjacency foundation -------------------------------------------------
  // Undirected adjacency: id -> set of neighbour ids. Built once per graph and
  // reused for hover-highlight, local-scope BFS, etc.
  const adjacency = useMemo(() => {
    const map = new Map<string, Set<string>>();
    const add = (a: string, b: string) => {
      let set = map.get(a);
      if (!set) {
        set = new Set<string>();
        map.set(a, set);
      }
      set.add(b);
    };
    for (const link of graphData.links) {
      const s = linkEndId(link.source);
      const tg = linkEndId(link.target);
      if (!s || !tg) {
        continue;
      }
      add(s, tg);
      add(tg, s);
    }
    return map;
  }, [graphData.links]);

  // ---- Folder color map -----------------------------------------------------
  const folderColors = useMemo(() => {
    const map = new Map<string, string>();
    let next = 0;
    for (const node of graphData.nodes) {
      const folder = node.folder;
      if (!map.has(folder)) {
        map.set(folder, FOLDER_PALETTE[next % FOLDER_PALETTE.length] ?? FOLDER_FALLBACK);
        next += 1;
      }
    }
    return map;
  }, [graphData.nodes]);

  const colorForNode = useCallback(
    (node: NodeObject<CanvasNode>): string => {
      if (colorBy === "folder") {
        return folderColors.get(node.folder) ?? FOLDER_FALLBACK;
      }
      return TYPE_COLORS[node.type];
    },
    [colorBy, folderColors],
  );

  // ---- Search match set -----------------------------------------------------
  const matchIds = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) {
      return null;
    }
    const set = new Set<string>();
    for (const node of graphData.nodes) {
      if (node.title.toLowerCase().includes(q) || node.id.toLowerCase().includes(q)) {
        set.add(node.id);
      }
    }
    return set;
  }, [query, graphData.nodes]);

  // ---- Local-scope BFS ------------------------------------------------------
  const localIds = useMemo(() => {
    if (scope !== "local" || !localRootId) {
      return null;
    }
    const visited = new Set<string>([localRootId]);
    let frontier: string[] = [localRootId];
    for (let depth = 0; depth < hops; depth += 1) {
      const nextFrontier: string[] = [];
      for (const id of frontier) {
        const neigh = adjacency.get(id);
        if (!neigh) {
          continue;
        }
        for (const n of neigh) {
          if (!visited.has(n)) {
            visited.add(n);
            nextFrontier.push(n);
          }
        }
      }
      frontier = nextFrontier;
      if (frontier.length === 0) {
        break;
      }
    }
    return visited;
  }, [scope, localRootId, hops, adjacency]);

  // ---- Combined view data (local scope + search-filter) ---------------------
  const viewData = useMemo(() => {
    const visible = (id: string): boolean => {
      if (localIds && !localIds.has(id)) {
        return false;
      }
      if (searchMode === "filter" && matchIds && !matchIds.has(id)) {
        return false;
      }
      return true;
    };

    const nodes = graphData.nodes.filter((n) => visible(n.id));
    const visibleIds = new Set(nodes.map((n) => n.id));
    const links = graphData.links.filter((link) => {
      const s = linkEndId(link.source);
      const tg = linkEndId(link.target);
      return visibleIds.has(s) && visibleIds.has(tg);
    });

    return { nodes, links };
  }, [graphData, localIds, matchIds, searchMode]);

  // Top-by-degree set used to cap labels at high zoom on large graphs.
  const topDegreeIds = useMemo(() => {
    const sorted = [...viewData.nodes]
      .sort((a, b) => b.degree - a.degree)
      .slice(0, MAX_LABELS);
    return new Set(sorted.map((n) => n.id));
  }, [viewData.nodes]);

  // Reheat + fit whenever the view data changes.
  useEffect(() => {
    if (viewData.nodes.length === 0) {
      return;
    }
    reheat();
    const id = window.setTimeout(() => {
      graphRef.current?.zoomToFit(600, 60);
    }, 120);
    return () => window.clearTimeout(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [viewData]);

  // Set of neighbour ids of the hovered node (for highlight). Uses adjacency.
  const neighbors = useMemo(() => {
    if (!hoverId) {
      return null;
    }
    const set = new Set<string>([hoverId]);
    const neigh = adjacency.get(hoverId);
    if (neigh) {
      for (const n of neigh) {
        set.add(n);
      }
    }
    return set;
  }, [hoverId, adjacency]);

  // Active highlight set for search in highlight mode (null = no search).
  const highlightMatch = searchMode === "highlight" ? matchIds : null;

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
    setRunning(true);
  }, [applyForces, graphData]);

  // ---- Editor -> graph sync -------------------------------------------------
  const centerOnSelected = useCallback((id: string) => {
    const fg = graphRef.current;
    if (!fg) {
      return false;
    }
    const node = viewDataNodesRef.current.find((n) => n.id === id);
    if (!node || typeof node.x !== "number" || typeof node.y !== "number") {
      return false;
    }
    fg.centerAt(node.x, node.y, 600);
    fg.zoom(2.2, 600);
    centeredRef.current = id;
    return true;
    // viewDataNodesRef is a stable ref; no deps needed.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Keep a ref to the latest view nodes so centerOnSelected can read live coords.
  const viewDataNodesRef = useRef<CanvasNode[]>([]);
  viewDataNodesRef.current = viewData.nodes;

  useEffect(() => {
    if (!activePath) {
      return;
    }
    // Resolve the matching node: exact id first, then a path-suffix match.
    let resolved =
      graphData.nodes.find((n) => n.id === activePath) ??
      graphData.nodes.find((n) => n.id.endsWith(activePath));
    if (!resolved) {
      return;
    }
    setSelectedId(resolved.id);
    setLocalRootId(resolved.id);
    centeredRef.current = null;
    // Attempt to center now; if coords aren't ready it will be retried on stop.
    reheat();
    const id = window.setTimeout(() => {
      centerOnSelected(resolved!.id);
    }, 150);
    return () => window.clearTimeout(id);
  }, [activePath, graphData.nodes, reheat, centerOnSelected]);

  const handleNodeClick = useCallback(
    (node: NodeObject<CanvasNode>) => {
      setSelectedId(node.id);
      onOpenNote({ path: node.id, title: node.title });
    },
    [onOpenNote],
  );

  const handleFit = useCallback(() => {
    graphRef.current?.zoomToFit(600, 60);
  }, []);

  const handleEngineStop = useCallback(() => {
    setRunning(false);
    // Retry centering on the active selection if it hasn't happened yet.
    if (selectedId && centeredRef.current !== selectedId) {
      if (!centerOnSelected(selectedId)) {
        graphRef.current?.zoomToFit(600, 60);
      }
    } else if (!centeredRef.current) {
      graphRef.current?.zoomToFit(600, 60);
    }
  }, [selectedId, centerOnSelected]);

  const nodeRelSize = 4 * nodeScale;

  const nodeVal = useCallback(
    (node: NodeObject<CanvasNode>) => 1 + Math.sqrt(node.degree) * 1.6,
    [],
  );

  // True when a node should be dimmed (hover-neighbour or search highlight).
  const isDimmed = useCallback(
    (id: string): boolean => {
      if (neighbors && !neighbors.has(id)) {
        return true;
      }
      if (highlightMatch && !highlightMatch.has(id)) {
        return true;
      }
      return false;
    },
    [neighbors, highlightMatch],
  );

  const nodeColor = useCallback(
    (node: NodeObject<CanvasNode>) => {
      const base = colorForNode(node);
      if (isDimmed(node.id)) {
        return hexWithAlpha(base, DIM_ALPHA);
      }
      return base;
    },
    [colorForNode, isDimmed],
  );

  const linkColor = useCallback(
    (link: NodeObject<CanvasLink>) => {
      const c = LINK_COLORS[link.kind] ?? DEFAULT_LINK_COLOR;
      const s = linkEndId(link.source as string | { id?: string });
      const tg = linkEndId(link.target as string | { id?: string });
      if (neighbors) {
        const active = neighbors.has(s) || neighbors.has(tg);
        return active ? c : "rgba(167, 167, 180, 0.06)";
      }
      if (highlightMatch) {
        const active = highlightMatch.has(s) || highlightMatch.has(tg);
        return active ? c : "rgba(167, 167, 180, 0.06)";
      }
      return c;
    },
    [neighbors, highlightMatch],
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
      const base = colorForNode(node);
      const dimmed = isDimmed(node.id);
      const focused = hoverId === node.id;
      const selected = selectedId === node.id;

      // soft glow — sadece odaklı/önemli düğümler için (her-frame CPU'yu düşürür)
      if (focused || selected || node.degree >= 6) {
        ctx.beginPath();
        ctx.arc(x, y, r * 1.9, 0, 2 * Math.PI);
        ctx.fillStyle = hexWithAlpha(
          base,
          dimmed ? 0.04 : focused || selected ? 0.28 : 0.14,
        );
        ctx.fill();
      }

      // node body
      ctx.beginPath();
      ctx.arc(x, y, r, 0, 2 * Math.PI);
      ctx.fillStyle = dimmed ? hexWithAlpha(base, 0.22) : base;
      ctx.fill();

      // selected: distinct ring so the active note stands out.
      if (selected) {
        ctx.beginPath();
        ctx.arc(x, y, r + 3 / globalScale, 0, 2 * Math.PI);
        ctx.lineWidth = 2.2 / globalScale;
        ctx.strokeStyle = "#e8bf69";
        ctx.stroke();
      } else if (focused) {
        ctx.lineWidth = 1.5 / globalScale;
        ctx.strokeStyle = "rgba(255,255,255,0.9)";
        ctx.stroke();
      }

      // Zoom-tiered labels:
      //  < 1.0   -> only selected / hovered
      //  1.0–2.0 -> higher-degree nodes (capped to top-by-degree)
      //  > 2.0   -> all (still capped to MAX_LABELS top-by-degree)
      let labelEligible = false;
      if (focused || selected) {
        labelEligible = true;
      } else if (globalScale > 2.0) {
        labelEligible = topDegreeIds.has(node.id);
      } else if (globalScale >= 1.0) {
        labelEligible = node.degree >= 4 && topDegreeIds.has(node.id);
      }

      const showThis = showLabels && !dimmed && labelEligible;

      if (showThis) {
        const fontSize = Math.max(9, 11 / globalScale);
        ctx.font = `${fontSize}px -apple-system, system-ui, sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "top";
        ctx.fillStyle = focused || selected ? "#ececf1" : "rgba(236, 236, 241, 0.78)";
        ctx.fillText(node.title, x, y + r + 2 / globalScale);
      }
    },
    [colorForNode, isDimmed, hoverId, selectedId, nodeRelSize, showLabels, topDegreeIds],
  );

  // Legend: type counts, or folder counts when colouring by folder.
  const typeCounts = useMemo(() => {
    const byType: Record<NodeType, number> = {
      markdown: 0,
      code: 0,
      config: 0,
      binary: 0,
      external: 0,
      dangling: 0,
    };
    for (const node of viewData.nodes) {
      byType[node.type] += 1;
    }
    return byType;
  }, [viewData.nodes]);

  const folderLegend = useMemo(() => {
    const counts = new Map<string, number>();
    for (const node of viewData.nodes) {
      counts.set(node.folder, (counts.get(node.folder) ?? 0) + 1);
    }
    // Preserve the stable folderColors order, only include present folders.
    const rows: { folder: string; color: string; count: number }[] = [];
    for (const [folder, color] of folderColors) {
      const count = counts.get(folder) ?? 0;
      if (count > 0) {
        rows.push({ folder, color, count });
      }
    }
    return rows;
  }, [viewData.nodes, folderColors]);

  // ---- Context menu ---------------------------------------------------------
  const handleNodeRightClick = useCallback(
    (node: NodeObject<CanvasNode>, event: MouseEvent) => {
      event.preventDefault();
      const stage = stageRef.current;
      if (!stage) {
        return;
      }
      const rect = stage.getBoundingClientRect();
      setMenu({
        x: event.clientX - rect.left,
        y: event.clientY - rect.top,
        nodeId: node.id,
      });
    },
    [],
  );

  const closeMenu = useCallback(() => setMenu(null), []);

  const handleBackgroundClick = useCallback(() => {
    closeMenu();
  }, [closeMenu]);

  useEffect(() => {
    if (!menu) {
      return undefined;
    }
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        closeMenu();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [menu, closeMenu]);

  const menuNode = useMemo(
    () => (menu ? graphData.nodes.find((n) => n.id === menu.nodeId) ?? null : null),
    [menu, graphData.nodes],
  );

  const menuOpen = useCallback(() => {
    if (menuNode) {
      onOpenNote({ path: menuNode.id, title: menuNode.title });
      setSelectedId(menuNode.id);
    }
    closeMenu();
  }, [menuNode, onOpenNote, closeMenu]);

  const menuFocusLocal = useCallback(() => {
    if (menu) {
      setLocalRootId(menu.nodeId);
      setScope("local");
    }
    closeMenu();
  }, [menu, closeMenu]);

  const menuCopyPath = useCallback(() => {
    if (menu) {
      void navigator.clipboard?.writeText(menu.nodeId);
    }
    closeMenu();
  }, [menu, closeMenu]);

  const hasGraph = graphData.nodes.length > 0;
  const activeSetSmall = viewData.nodes.length <= 80;

  return (
    <section className="task-panel graph-panel" aria-labelledby="graph-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">{t("nav.graph")}</p>
          <h1 id="graph-title">{t("graph.title")}</h1>
        </div>
        <div className="toolbar graph-toolbar" aria-label={t("graph.title")}>
          <span className="badge">
            {viewData.nodes.length} {t("graph.nodes")} · {viewData.links.length}{" "}
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
            graphData={viewData}
            width={size.width}
            height={size.height}
            cooldownTicks={running ? 70 : 0}
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
            linkDirectionalParticles={(neighbors || highlightMatch) && activeSetSmall ? 2 : 0}
            linkDirectionalParticleWidth={1.6}
            linkDirectionalParticleColor={() => "rgba(143,140,245,0.7)"}
            onNodeClick={handleNodeClick}
            onNodeHover={(node) => setHoverId(node ? node.id : null)}
            onNodeRightClick={handleNodeRightClick}
            onNodeDrag={() => reheat()}
            onBackgroundClick={handleBackgroundClick}
            onEngineStop={handleEngineStop}
            showPointerCursor
          />
        ) : null}

        {menu ? (
          <div
            className="graph-menu"
            style={{
              position: "absolute",
              top: menu.y,
              left: menu.x,
              zIndex: 5,
              minWidth: 168,
              padding: 4,
              border: "1px solid var(--border)",
              borderRadius: "var(--radius)",
              background: "color-mix(in srgb, var(--bg-secondary) 94%, var(--bg))",
              boxShadow: "var(--shadow)",
              backdropFilter: "blur(8px)",
              display: "grid",
              gap: 2,
            }}
            role="menu"
            onContextMenu={(e) => e.preventDefault()}
          >
            <button className="button ghost graph-menu-item" type="button" onClick={menuOpen}>
              {t("graph.menu.open")}
            </button>
            <button
              className="button ghost graph-menu-item"
              type="button"
              onClick={menuFocusLocal}
            >
              {t("graph.menu.focusLocal")}
            </button>
            <button
              className="button ghost graph-menu-item"
              type="button"
              onClick={menuCopyPath}
            >
              {t("graph.menu.copyPath")}
            </button>
          </div>
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
                  <span>{t("graph.search")}</span>
                  <input
                    type="text"
                    className="graph-search-input"
                    placeholder={t("graph.searchPlaceholder")}
                    value={query}
                    onChange={(e) => {
                      setQuery(e.target.value);
                      reheat();
                    }}
                  />
                </label>

                <div className="graph-control">
                  <div className="graph-seg" role="group">
                    <button
                      type="button"
                      className={`graph-seg-btn${searchMode === "highlight" ? " is-active" : ""}`}
                      onClick={() => setSearchMode("highlight")}
                    >
                      {t("graph.searchHighlight")}
                    </button>
                    <button
                      type="button"
                      className={`graph-seg-btn${searchMode === "filter" ? " is-active" : ""}`}
                      onClick={() => {
                        setSearchMode("filter");
                        reheat();
                      }}
                    >
                      {t("graph.searchFilter")}
                    </button>
                  </div>
                </div>

                <div className="graph-control">
                  <span>{t("graph.colorBy")}</span>
                  <div className="graph-seg" role="group">
                    <button
                      type="button"
                      className={`graph-seg-btn${colorBy === "type" ? " is-active" : ""}`}
                      onClick={() => setColorBy("type")}
                    >
                      {t("graph.colorByType")}
                    </button>
                    <button
                      type="button"
                      className={`graph-seg-btn${colorBy === "folder" ? " is-active" : ""}`}
                      onClick={() => setColorBy("folder")}
                    >
                      {t("graph.colorByFolder")}
                    </button>
                  </div>
                </div>

                <div className="graph-control">
                  <span>{t("graph.scope")}</span>
                  <div className="graph-seg" role="group">
                    <button
                      type="button"
                      className={`graph-seg-btn${scope === "global" ? " is-active" : ""}`}
                      onClick={() => {
                        setScope("global");
                        reheat();
                      }}
                    >
                      {t("graph.scope.global")}
                    </button>
                    <button
                      type="button"
                      className={`graph-seg-btn${scope === "local" ? " is-active" : ""}`}
                      onClick={() => {
                        setScope("local");
                        reheat();
                      }}
                    >
                      {t("graph.scope.local")}
                    </button>
                  </div>
                </div>

                {scope === "local" ? (
                  <label className="graph-control">
                    <span>
                      {t("graph.hops")} · {hops}
                    </span>
                    <input
                      type="range"
                      min={1}
                      max={3}
                      step={1}
                      value={hops}
                      onChange={(e) => {
                        setHops(Number(e.target.value));
                        reheat();
                      }}
                    />
                  </label>
                ) : null}

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
                    onChange={(e) => {
                      setLinkDistance(Number(e.target.value));
                      reheat();
                    }}
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
                  {colorBy === "folder"
                    ? folderLegend.map(({ folder, color, count }) => (
                        <div className="graph-legend-row" key={folder || "(root)"}>
                          <span
                            className="graph-legend-dot"
                            style={{ background: color }}
                          />
                          <span className="graph-legend-label" title={folder}>
                            {folder === "" ? "/" : folder.split("/").pop()}
                          </span>
                          <span className="graph-legend-count">{count}</span>
                        </div>
                      ))
                    : LEGEND_TYPES.map(({ type, key }) => (
                        <div className="graph-legend-row" key={type}>
                          <span
                            className="graph-legend-dot"
                            style={{ background: TYPE_COLORS[type] }}
                          />
                          <span className="graph-legend-label">{t(key)}</span>
                          <span className="graph-legend-count">{typeCounts[type]}</span>
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
  max-height: min(70vh, 560px);
  overflow-y: auto;
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
.graph-search-input {
  width: 100%;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: calc(var(--radius) * 0.7);
  background: var(--bg);
  color: var(--text);
  font-size: 12px;
  font-weight: 500;
}
.graph-search-input:focus {
  outline: none;
  border-color: var(--accent);
}
.graph-seg {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  gap: 4px;
}
.graph-seg-btn {
  padding: 5px 6px;
  border: 1px solid var(--border);
  border-radius: calc(var(--radius) * 0.7);
  background: var(--bg);
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
}
.graph-seg-btn:hover {
  background: var(--bg-tertiary);
}
.graph-seg-btn.is-active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
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
.graph-menu-item {
  width: 100%;
  justify-content: flex-start;
  text-align: left;
  min-height: 28px;
  padding: 5px 10px;
  font-size: 12px;
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
.graph-legend-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.graph-legend-count {
  color: var(--text);
  font-variant-numeric: tabular-nums;
  font-size: 11px;
  opacity: 0.7;
}
`;
