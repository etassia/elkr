# Changelog

All notable changes to this project will be documented in this file.

## 0.1.1 (2026-11-15)

### Features

- The force-directed algorithm: inverse-square repulsion and spring
  attraction with linear cooling; the initial scatter comes from a
  fixed-seed SplitMix64 stream so the layout stays byte-identical;
  options `iterations`, `repulsion`, `temperature` (with `elk.force.*`
  aliases); preset node positions are preserved as the start; edges
  route straight, center to center by @ronaldtse
