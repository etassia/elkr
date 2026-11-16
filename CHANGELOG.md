# Changelog

All notable changes to this project will be documented in this file.

## 0.2.0 (2026-11-16)

### Features

- The pipeline is complete: ports (side detection, index ordering,
  even distribution, port-anchored sections), hierarchical (nested)
  graphs under layered, per-edge orthogonal routing, the edge label
  pass (arc-length midpoint placement, stacked), and the DIM adapter
  (`elkr dim`) by @ronaldtse

### Breaking

- ElkNode carries ports, ElkEdge carries labels and layout options,
  and ElkLabel carries width and height: exhaustive struct
  initializers must add the new fields (semver-checks flagged the
  additions as breaking for that reason) by @ronaldtse

## 0.1.2 (2026-11-16)

### Features

- The registry is complete: all fifteen elkrb algorithms — layered and
  force from 0.1.1, and now box, fixed, random, vertiflex, radial,
  rectpacking, topdownpacking, stress, mrtree, disco, spore-overlap,
  spore-compaction and libavoid, each deterministic by construction and
  byte-identical on every run by @ronaldtse

## 0.1.1 (2026-11-15)

### Features

- The force-directed algorithm: inverse-square repulsion and spring
  attraction with linear cooling; the initial scatter comes from a
  fixed-seed SplitMix64 stream so the layout stays byte-identical;
  options `iterations`, `repulsion`, `temperature` (with `elk.force.*`
  aliases); preset node positions are preserved as the start; edges
  route straight, center to center by @ronaldtse
