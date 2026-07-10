#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    metal_bmp: Path
    vias_bmp: Path
    poly_bmp: Path
    diffusion_bmp: Path
    contacts_bmp: Path | None
    schematic_bmp: Path
    signals_txt: Path
    transistors_json: Path


def specs() -> dict[str, ChipSpec]:
    def emu(p: str) -> Path:
        return ROOT / "docs" / "emulators" / p

    def trans(p: str) -> Path:
        return ROOT / "docs" / "evidence" / "transistors" / p

    return {
        "4001": ChipSpec(
            "4001",
            metal_bmp=emu("i4001-metal.bmp"),
            vias_bmp=emu("i4001-vias.bmp"),
            poly_bmp=emu("i4001-poly.bmp"),
            diffusion_bmp=emu("i4001-diffusion.bmp"),
            contacts_bmp=None,
            schematic_bmp=emu("i4001-schematic.bmp"),
            signals_txt=emu("i4001-signals.txt"),
            transistors_json=trans("4001_poly_diffusion_transistors.json"),
        ),
        "4002": ChipSpec(
            "4002",
            metal_bmp=emu("i4002-metal.bmp"),
            vias_bmp=emu("i4002-vias.bmp"),
            poly_bmp=emu("i4002-poly.bmp"),
            diffusion_bmp=emu("i4002-diffusion.bmp"),
            contacts_bmp=None,
            schematic_bmp=emu("i4002-schematic.bmp"),
            signals_txt=emu("i4002-signals.txt"),
            transistors_json=trans("4002_poly_diffusion_transistors.json"),
        ),
        "4003": ChipSpec(
            "4003",
            metal_bmp=emu("i4003-metal.bmp"),
            vias_bmp=emu("i4003-vias.bmp"),
            poly_bmp=emu("i4003-poly.bmp"),
            diffusion_bmp=emu("i4003-diffusion.bmp"),
            contacts_bmp=None,
            schematic_bmp=emu("i4003-schematic.bmp"),
            signals_txt=emu("i4003-signals.txt"),
            transistors_json=trans("4003_poly_diffusion_transistors.json"),
        ),
        "4004": ChipSpec(
            "4004",
            metal_bmp=emu("i4004-metal.bmp"),
            vias_bmp=emu("i4004-vias.bmp"),
            poly_bmp=emu("i4004-poly.bmp"),
            diffusion_bmp=emu("i4004-diffusion.bmp"),
            contacts_bmp=emu("i4004-contacts.bmp"),
            schematic_bmp=emu("i4004-schematic.bmp"),
            signals_txt=emu("i4004-signals.txt"),
            transistors_json=trans("4004_poly_diffusion_transistors.json"),
        ),
    }


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def load_mask(path: Path, threshold: int, *, polarity: str) -> np.ndarray:
    img = Image.open(path).convert("L")
    arr = np.asarray(img)
    if polarity == "dark":
        return arr < threshold
    if polarity == "light":
        return arr > threshold
    if polarity != "auto":
        raise ValueError(f"unknown polarity: {polarity}")

    # Most i400x layer bitmaps in this repo are "ink on paper":
    # device features are dark (0), background is light (255).
    # Autodetect by choosing the minority class.
    light = arr > threshold
    dark = ~light
    return dark if dark.mean() < light.mean() else light


def connected_components(mask: np.ndarray) -> tuple[int, np.ndarray, np.ndarray, np.ndarray]:
    u8 = np.where(mask, 255, 0).astype(np.uint8)
    return cv2.connectedComponentsWithStats(u8, connectivity=8)


class DSU:
    def __init__(self, n: int):
        self.parent = list(range(n))
        self.rank = [0] * n

    def find(self, a: int) -> int:
        p = self.parent[a]
        if p != a:
            self.parent[a] = self.find(p)
        return self.parent[a]

    def union(self, a: int, b: int) -> bool:
        ra = self.find(a)
        rb = self.find(b)
        if ra == rb:
            return False
        if self.rank[ra] < self.rank[rb]:
            ra, rb = rb, ra
        self.parent[rb] = ra
        if self.rank[ra] == self.rank[rb]:
            self.rank[ra] += 1
        return True


def parse_signals_txt(path: Path) -> list[dict[str, object]]:
    out: list[dict[str, object]] = []
    for raw in path.read_text(errors="replace").splitlines():
        line = raw.strip().strip("\r")
        if not line or line.startswith(";"):
            continue
        parts = [p.strip() for p in line.split(",")]
        if len(parts) != 3:
            continue
        try:
            x = int(parts[0])
            y = int(parts[1])
        except ValueError:
            continue
        out.append({"x": x, "y": y, "name": parts[2]})
    return out


def _mode_label(labels: np.ndarray) -> int | None:
    vals = labels.ravel()
    vals = vals[vals > 0]
    if vals.size == 0:
        return None
    counts = Counter(int(v) for v in vals.tolist())
    # Deterministic: break ties by smaller label id.
    return sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))[0][0]


def _top_k_labels(labels: np.ndarray, k: int) -> list[int]:
    vals = labels.ravel()
    vals = vals[vals > 0]
    if vals.size == 0:
        return []
    counts = Counter(int(v) for v in vals.tolist())
    # Deterministic: break ties by smaller label id.
    items = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))
    return [lab for lab, _cnt in items[:k]]


def main() -> int:
    p = argparse.ArgumentParser(
        description="Extract a deterministic, partial multi-layer connectivity netlist from i400x layer bitmaps (v0)."
    )
    p.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to extract (repeatable)")
    p.add_argument("--all", action="store_true", help="Extract for all supported chips")
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "netlists_v0")
    p.add_argument("--threshold", type=int, default=128, help="Threshold for layer masks")
    p.add_argument(
        "--polarity",
        choices=["auto", "dark", "light"],
        default="auto",
        help="Mask polarity: 'dark' means features are dark pixels (< threshold), 'light' means features are light pixels (> threshold).",
    )
    p.add_argument("--dilate", type=int, default=0, help="Dilation iterations for via/contact masks before stitching")
    p.add_argument(
        "--close",
        type=int,
        default=0,
        help="Morphological closing iterations for layer masks before connected-components (can heal 1px gaps).",
    )
    p.add_argument(
        "--diffusion-split",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Split diffusion by removing poly overlap (default true; disable to keep diffusion continuous under poly).",
    )
    p.add_argument("--pad", type=int, default=3, help="Padding (px) around transistor candidate bboxes when sampling terminals")
    p.add_argument("--stitch-max-labels", type=int, default=4, help="Max distinct labels per layer to stitch per via/contact blob")
    p.add_argument(
        "--stitch-policy",
        choices=["strict", "relaxed"],
        default="strict",
        help=(
            "How to stitch layers through via/contact masks. "
            "'strict' requires via/contact pixels to overlap both layers (historical v0). "
            "'relaxed' uses the via/contact component region to sample each layer separately (more robust to mask misalignment)."
        ),
    )
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        p.error("select --all or at least one --chip")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    manifest: dict[str, object] = {
        "tool": "scripts/extract_netlist_v0.py",
        "params": {
            "threshold": args.threshold,
            "polarity": args.polarity,
            "dilate": args.dilate,
            "close": args.close,
            "diffusion_split": bool(args.diffusion_split),
            "pad": args.pad,
            "stitch_max_labels": int(args.stitch_max_labels),
            "stitch_policy": str(args.stitch_policy),
        },
        "outputs": [],
    }

    for chip in sorted(selected):
        spec = specs()[chip]

        metal = load_mask(spec.metal_bmp, threshold=args.threshold, polarity=args.polarity)
        vias = load_mask(spec.vias_bmp, threshold=args.threshold, polarity=args.polarity)
        poly = load_mask(spec.poly_bmp, threshold=args.threshold, polarity=args.polarity)
        diffusion = load_mask(spec.diffusion_bmp, threshold=args.threshold, polarity=args.polarity)
        contacts = (
            load_mask(spec.contacts_bmp, threshold=args.threshold, polarity=args.polarity) if spec.contacts_bmp else None
        )

        close_iters = int(args.close)
        if close_iters > 0:
            ck = np.ones((3, 3), dtype=np.uint8)

            def close_mask(
                mask: np.ndarray, ck: np.ndarray = ck, close_iters: int = close_iters
            ) -> np.ndarray:
                u8 = np.where(mask, 255, 0).astype(np.uint8)
                u8 = cv2.morphologyEx(u8, cv2.MORPH_CLOSE, ck, iterations=close_iters)
                return u8 > 0

            metal = close_mask(metal)
            poly = close_mask(poly)
            diffusion = close_mask(diffusion)

        shape = metal.shape
        for name, arr in [
            ("vias", vias),
            ("poly", poly),
            ("diffusion", diffusion),
        ]:
            if arr.shape != shape:
                raise SystemExit(f"{chip}: shape mismatch metal={shape} {name}={arr.shape}")
        if contacts is not None and contacts.shape != shape:
            raise SystemExit(f"{chip}: shape mismatch contacts={contacts.shape} metal={shape}")

        diffusion_split = diffusion & (~poly) if args.diffusion_split else diffusion

        m_n, m_lab, m_stats, _m_cent = connected_components(metal)
        p_n, p_lab, p_stats, _p_cent = connected_components(poly)
        d_n, d_lab, d_stats, _d_cent = connected_components(diffusion_split)

        m_count = int(m_n - 1)
        p_count = int(p_n - 1)
        d_count = int(d_n - 1)

        offsets = {"metal": 0, "poly": m_count, "diffusion": m_count + p_count}
        total_comps = m_count + p_count + d_count
        dsu = DSU(total_comps)

        def gid(layer: str, label: int, offsets: dict[str, int] = offsets) -> int:
            if label <= 0:
                raise ValueError("label must be >0")
            return offsets[layer] + (label - 1)

        stitch_vias = 0
        stitch_contacts = 0
        vias_cc_ambiguous = 0
        contacts_cc_ambiguous = 0

        # Stitch rule v0:
        # - vias: connect metal <-> poly
        # - contacts (4004 only): connect metal <-> diffusion_split
        k = np.ones((3, 3), dtype=np.uint8)
        vias_u8 = np.where(vias, 255, 0).astype(np.uint8)
        if int(args.dilate) > 0:
            vias_u8 = cv2.dilate(vias_u8, k, iterations=int(args.dilate))
        vias_mask = vias_u8 > 0
        stitch_policy = str(args.stitch_policy)
        vias_cc_input = vias_mask & metal & poly if stitch_policy == "strict" else vias_mask
        v_n, v_lab, v_stats, _v_cent = connected_components(vias_cc_input)
        for v_id in range(1, int(v_n)):
            x, y, w, h, _area = (int(v) for v in v_stats[v_id].tolist())
            if w <= 0 or h <= 0:
                continue
            v_roi = v_lab[y : y + h, x : x + w] == v_id
            if stitch_policy == "relaxed":
                # Expand the via region slightly to tolerate mask misalignment between via + layer bitmaps.
                v_u8 = (v_roi.astype(np.uint8)) * 255
                v_u8 = cv2.dilate(v_u8, k, iterations=1)
                v_roi = v_u8 > 0
            max_k = int(args.stitch_max_labels)
            # In relaxed mode, sample labels under the via component region for each layer independently.
            m_ids = _top_k_labels(m_lab[y : y + h, x : x + w][v_roi], max_k + 1)
            p_ids = _top_k_labels(p_lab[y : y + h, x : x + w][v_roi], max_k + 1)
            if not m_ids or not p_ids:
                vias_cc_ambiguous += 1
                continue
            if len(m_ids) > max_k or len(p_ids) > max_k:
                vias_cc_ambiguous += 1
                m_ids = m_ids[:max_k]
                p_ids = p_ids[:max_k]
            for m_id in m_ids:
                for p_id in p_ids:
                    if dsu.union(gid("metal", m_id), gid("poly", p_id)):
                        stitch_vias += 1

        if contacts is not None:
            c_u8 = np.where(contacts, 255, 0).astype(np.uint8)
            if int(args.dilate) > 0:
                c_u8 = cv2.dilate(c_u8, k, iterations=int(args.dilate))
            c_mask = c_u8 > 0
            contacts_cc_input = c_mask & metal & diffusion_split if stitch_policy == "strict" else c_mask
            c_n, c_lab, c_stats, _c_cent = connected_components(contacts_cc_input)
            for c_id in range(1, int(c_n)):
                x, y, w, h, _area = (int(v) for v in c_stats[c_id].tolist())
                if w <= 0 or h <= 0:
                    continue
                c_roi = c_lab[y : y + h, x : x + w] == c_id
                if stitch_policy == "relaxed":
                    c_u8_roi = (c_roi.astype(np.uint8)) * 255
                    c_u8_roi = cv2.dilate(c_u8_roi, k, iterations=1)
                    c_roi = c_u8_roi > 0
                max_k = int(args.stitch_max_labels)
                m_ids = _top_k_labels(m_lab[y : y + h, x : x + w][c_roi], max_k + 1)
                d_ids = _top_k_labels(d_lab[y : y + h, x : x + w][c_roi], max_k + 1)
                if not m_ids or not d_ids:
                    contacts_cc_ambiguous += 1
                    continue
                if len(m_ids) > max_k or len(d_ids) > max_k:
                    contacts_cc_ambiguous += 1
                    m_ids = m_ids[:max_k]
                    d_ids = d_ids[:max_k]
                for m_id in m_ids:
                    for d_id in d_ids:
                        if dsu.union(gid("metal", m_id), gid("diffusion", d_id)):
                            stitch_contacts += 1

        # Canonicalize DSU roots to dense node IDs (deterministic).
        roots = sorted({dsu.find(i) for i in range(total_comps)})
        root_to_node = {r: idx for idx, r in enumerate(roots)}

        def node_of(
            layer: str,
            label: int,
            root_to_node: dict[int, int] = root_to_node,
            dsu: DSU = dsu,
        ) -> int:
            return int(root_to_node[dsu.find(gid(layer, label))])

        node_count = int(len(roots))
        node_metal_area = [0] * node_count
        node_poly_area = [0] * node_count
        node_diff_area = [0] * node_count
        node_metal_cc = [0] * node_count
        node_poly_cc = [0] * node_count
        node_diff_cc = [0] * node_count
        node_metal_bbox = [None] * node_count
        node_poly_bbox = [None] * node_count
        node_diff_bbox = [None] * node_count

        for lab in range(1, m_count + 1):
            area = int(m_stats[lab][cv2.CC_STAT_AREA])
            x = int(m_stats[lab][cv2.CC_STAT_LEFT])
            y = int(m_stats[lab][cv2.CC_STAT_TOP])
            w = int(m_stats[lab][cv2.CC_STAT_WIDTH])
            h = int(m_stats[lab][cv2.CC_STAT_HEIGHT])
            n = node_of("metal", lab)
            node_metal_area[n] += area
            node_metal_cc[n] += 1
            bb = node_metal_bbox[n]
            if bb is None:
                node_metal_bbox[n] = [x, y, x + w, y + h]
            else:
                bb[0] = min(bb[0], x)
                bb[1] = min(bb[1], y)
                bb[2] = max(bb[2], x + w)
                bb[3] = max(bb[3], y + h)
        for lab in range(1, p_count + 1):
            area = int(p_stats[lab][cv2.CC_STAT_AREA])
            x = int(p_stats[lab][cv2.CC_STAT_LEFT])
            y = int(p_stats[lab][cv2.CC_STAT_TOP])
            w = int(p_stats[lab][cv2.CC_STAT_WIDTH])
            h = int(p_stats[lab][cv2.CC_STAT_HEIGHT])
            n = node_of("poly", lab)
            node_poly_area[n] += area
            node_poly_cc[n] += 1
            bb = node_poly_bbox[n]
            if bb is None:
                node_poly_bbox[n] = [x, y, x + w, y + h]
            else:
                bb[0] = min(bb[0], x)
                bb[1] = min(bb[1], y)
                bb[2] = max(bb[2], x + w)
                bb[3] = max(bb[3], y + h)
        for lab in range(1, d_count + 1):
            area = int(d_stats[lab][cv2.CC_STAT_AREA])
            x = int(d_stats[lab][cv2.CC_STAT_LEFT])
            y = int(d_stats[lab][cv2.CC_STAT_TOP])
            w = int(d_stats[lab][cv2.CC_STAT_WIDTH])
            h = int(d_stats[lab][cv2.CC_STAT_HEIGHT])
            n = node_of("diffusion", lab)
            node_diff_area[n] += area
            node_diff_cc[n] += 1
            bb = node_diff_bbox[n]
            if bb is None:
                node_diff_bbox[n] = [x, y, x + w, y + h]
            else:
                bb[0] = min(bb[0], x)
                bb[1] = min(bb[1], y)
                bb[2] = max(bb[2], x + w)
                bb[3] = max(bb[3], y + h)

        # Load transistor candidates from existing extraction.
        trans = json.loads(spec.transistors_json.read_text(encoding="utf-8"))
        comps = trans.get("components", [])
        if not isinstance(comps, list):
            comps = []

        pad = int(args.pad)
        transistors: list[dict[str, object]] = []
        ambiguous = 0
        node_gate_degree = [0] * node_count
        node_terminal_degree = [0] * node_count
        for c in comps:
            if not isinstance(c, dict):
                continue
            bbox = c.get("bbox")
            if not isinstance(bbox, dict):
                continue
            x0 = max(0, int(bbox.get("x", 0)) - pad)
            y0 = max(0, int(bbox.get("y", 0)) - pad)
            x1 = min(shape[1], x0 + int(bbox.get("w", 0)) + pad * 2)
            y1 = min(shape[0], y0 + int(bbox.get("h", 0)) + pad * 2)
            if x1 <= x0 or y1 <= y0:
                continue

            gate_lab = _mode_label(p_lab[y0:y1, x0:x1])
            if gate_lab is None:
                ambiguous += 1
                continue

            terms = _top_k_labels(d_lab[y0:y1, x0:x1], 2)
            if len(terms) != 2:
                ambiguous += 1
                continue
            terms = sorted(terms)

            transistors.append(
                {
                    "kind": "pmos_candidate",
                    "gate_node": node_of("poly", int(gate_lab)),
                    "a_node": node_of("diffusion", int(terms[0])),
                    "b_node": node_of("diffusion", int(terms[1])),
                    "bbox": {"x": int(x0), "y": int(y0), "w": int(x1 - x0), "h": int(y1 - y0)},
                    "source_component": {"poly_diff_id": int(c.get("id", 0))},
                }
            )
            node_gate_degree[node_of("poly", int(gate_lab))] += 1
            node_terminal_degree[node_of("diffusion", int(terms[0]))] += 1
            node_terminal_degree[node_of("diffusion", int(terms[1]))] += 1

        # Map signals.txt points to a node by layer label at that coordinate.
        # NOTE: i400x_signals.txt reference points are defined on the *schematic* bitmap,
        # not the layout masks. We keep them in the output as a cross-reference only.
        # Mapping schematic reference points into the layout netlist requires a separate
        # schematic-vs-layout alignment step (future work).
        signal_ref_points = parse_signals_txt(spec.signals_txt)

        out_json = out_dir / f"{chip.lower()}_netlist_v0.json"
        schematic_img = Image.open(spec.schematic_bmp)
        schematic_w, schematic_h = schematic_img.size

        def node_uid_for(
            i: int,
            node_metal_bbox: list = node_metal_bbox,
            node_poly_bbox: list = node_poly_bbox,
            node_diff_bbox: list = node_diff_bbox,
            node_metal_area: list = node_metal_area,
            node_poly_area: list = node_poly_area,
            node_diff_area: list = node_diff_area,
            node_metal_cc: list = node_metal_cc,
            node_poly_cc: list = node_poly_cc,
            node_diff_cc: list = node_diff_cc,
            node_gate_degree: list = node_gate_degree,
            node_terminal_degree: list = node_terminal_degree,
        ) -> str:
            """
            Stable-ish content-derived identifier for a node.

            Node integer IDs are deterministic for a *given* extraction parameter set, but
            comparing across parameter sweeps (e.g. dilation/threshold changes) can reshuffle
            node numbering. A geometry-derived UID makes it easier to remap nodes between
            runs without depending on the DSU enumeration order.
            """
            fp = {
                "metal_bbox": node_metal_bbox[i],
                "poly_bbox": node_poly_bbox[i],
                "diff_bbox": node_diff_bbox[i],
                "metal_area": int(node_metal_area[i]),
                "poly_area": int(node_poly_area[i]),
                "diff_area": int(node_diff_area[i]),
                "metal_cc": int(node_metal_cc[i]),
                "poly_cc": int(node_poly_cc[i]),
                "diff_cc": int(node_diff_cc[i]),
                "gate_degree": int(node_gate_degree[i]),
                "terminal_degree": int(node_terminal_degree[i]),
            }
            raw = json.dumps(fp, sort_keys=True, separators=(",", ":")).encode("utf-8")
            return hashlib.sha1(raw, usedforsecurity=False).hexdigest()

        node_stats = [
            {
                "node": i,
                "node_uid": node_uid_for(i),
                "metal_area": int(node_metal_area[i]),
                "poly_area": int(node_poly_area[i]),
                "diffusion_area": int(node_diff_area[i]),
                "metal_components": int(node_metal_cc[i]),
                "poly_components": int(node_poly_cc[i]),
                "diffusion_components": int(node_diff_cc[i]),
                "gate_degree": int(node_gate_degree[i]),
                "terminal_degree": int(node_terminal_degree[i]),
                "metal_bbox": (
                    {"x0": int(node_metal_bbox[i][0]), "y0": int(node_metal_bbox[i][1]), "x1": int(node_metal_bbox[i][2]), "y1": int(node_metal_bbox[i][3])}
                    if node_metal_bbox[i] is not None
                    else None
                ),
                "poly_bbox": (
                    {"x0": int(node_poly_bbox[i][0]), "y0": int(node_poly_bbox[i][1]), "x1": int(node_poly_bbox[i][2]), "y1": int(node_poly_bbox[i][3])}
                    if node_poly_bbox[i] is not None
                    else None
                ),
                "diffusion_bbox": (
                    {"x0": int(node_diff_bbox[i][0]), "y0": int(node_diff_bbox[i][1]), "x1": int(node_diff_bbox[i][2]), "y1": int(node_diff_bbox[i][3])}
                    if node_diff_bbox[i] is not None
                    else None
                ),
            }
            for i in range(node_count)
        ]
        payload = {
            "chip": chip,
            "schema": {
                "version": 0,
                "description": "Partial connectivity netlist from i400x layer bitmaps (stitching via+contact; diffusion optionally split by poly).",
            },
            "inputs": {
                "metal_bmp": str(spec.metal_bmp.relative_to(ROOT)),
                "vias_bmp": str(spec.vias_bmp.relative_to(ROOT)),
                "poly_bmp": str(spec.poly_bmp.relative_to(ROOT)),
                "diffusion_bmp": str(spec.diffusion_bmp.relative_to(ROOT)),
                "contacts_bmp": str(spec.contacts_bmp.relative_to(ROOT)) if spec.contacts_bmp else None,
                "schematic_bmp": str(spec.schematic_bmp.relative_to(ROOT)),
                "signals_txt": str(spec.signals_txt.relative_to(ROOT)),
                "transistors_json": str(spec.transistors_json.relative_to(ROOT)),
                "sha256": {
                    "metal": sha256(spec.metal_bmp),
                    "vias": sha256(spec.vias_bmp),
                    "poly": sha256(spec.poly_bmp),
                    "diffusion": sha256(spec.diffusion_bmp),
                    "contacts": sha256(spec.contacts_bmp) if spec.contacts_bmp else None,
                    "schematic_bmp": sha256(spec.schematic_bmp),
                    "signals_txt": sha256(spec.signals_txt),
                    "transistors_json": sha256(spec.transistors_json),
                },
                "layout_shape": {"h": int(shape[0]), "w": int(shape[1])},
                "schematic_shape": {"h": int(schematic_h), "w": int(schematic_w)},
            },
            "params": manifest["params"],
            "counts": {
                "components": {"metal": m_count, "poly": p_count, "diffusion": d_count},
                "nodes": int(len(roots)),
                "stitches": {"vias": stitch_vias, "contacts": stitch_contacts},
                "stitches_ambiguous": {"vias": vias_cc_ambiguous, "contacts": contacts_cc_ambiguous},
                "transistors_kept": int(len(transistors)),
                "transistors_ambiguous": int(ambiguous),
                "signals_points": int(len(signal_ref_points)),
            },
            "node_stats": node_stats,
            "signals": {"space": "schematic", "schematic_reference_points": signal_ref_points},
            "devices": {"transistors": transistors},
        }
        out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest["outputs"].append({"chip": chip, "output": rel_or_abs(out_json)})

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
