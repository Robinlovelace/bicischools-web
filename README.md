# bicischools-web 🚲

A fast, client-side web application and WebAssembly engine for **identifying, prioritizing, and generating timetables for "bike buses" (cycle trains)** to encourage active and sociable school travel.

Based on the research paper:
> **Methods for prioritising 'bike buses' for safe and sociable cycling to school**  
> *Joey Talbot, Rosa Félix, Juan Fonseca-Zamora, Francisco Lino, Camila Garcia, Catarina Marcelino, Robin Lovelace* (Journal of Transport Geography).

Hosted statically on GitHub Pages with zero server dependencies: dynamic routing, network aggregation, uptake modeling, and timetable generation all run **100% in your browser using Rust + WebAssembly**.

---

## Features

- 🌍 **Any-Area OSM Integration**: Download street, cycleway, and path networks on-the-fly for any school worldwide via the OpenStreetMap Overpass API.
- 🚴 **Quiet vs. Fast Routing**:
  - **Quiet Profile**: Penalizes high-stress arterial motor roads, prioritizes segregated cycle tracks, park paths, living streets, and residential ways.
  - **Fast Profile**: Shortest distance and travel time.
- 📈 **PCT "Go Dutch" School Uptake Modeling**: Implements the school travel distance-decay and gradient model ($P(\text{cycle}) = \text{inv\_logit}(\alpha + d_1 \cdot d + d_2 \cdot \sqrt{d} + h_1 \cdot (g + h_2))$) to quantify cycling potential.
- 🔀 **Route Network Aggregation (`overline`)**: Aggregates overlapping route segments into a unified corridor demand network.
- 🎯 **Automated Corridor Prioritization**:
  - Distance-weighted corridor scoring ($L \times \text{WeightedMeanDemand}$).
  - Greedy spatial separation filtering (preventing redundant parallel corridors).
- 🕒 **Timetable & Stop Generator**: Automatically places scheduled pick-up stops along candidate bike buses, calculating back-timed departures and arrival times based on group cycling speed (e.g. 11 km/h) and student boarding counts.
- 📊 **Built-In Paper Case Studies**:
  - **Lisbon**: *Escola Básica Adriano Correia de Oliveira* (169 enrolled students).
  - **Almada**: *Escola Básica nº 2 da Costa da Caparica* (compared directly with actual CicloExpresso schedule).
  - **Manchester**: *Manley Park Primary School* (Whalley Range).
- 💾 **GIS & Timetable Export**: One-click download of GeoJSON candidate routes, route network, matched student origins, and CSV stop timetables.

---

## Project Architecture

```
bicischools-web/
├── crates/
│   ├── bicischools-core/     # Core Rust graph router, uptake, overline, & timetable engine
│   └── bicischools-cli/      # Command-line tool for headless analysis
├── bindings/
│   └── wasm/                 # WebAssembly bindings (wasm-bindgen)
├── web/                      # Svelte 5 + TypeScript + Vite + MapLibre GL frontend
│   ├── public/presets/       # Pre-calculated case study datasets
│   └── src/
│       ├── lib/wasm/         # Generated WASM bundle
│       ├── lib/overpass.ts   # Overpass OSM API client
│       ├── lib/map/layers.ts # MapLibre GL styling
│       └── App.svelte        # Application UI
└── .github/workflows/        # Automated GitHub Pages CI/CD deployment
```

---

## Getting Started

### Prerequisites

- **Node.js** (v20+) & `npm`
- **Rust** (1.80+) with `wasm32-unknown-unknown` target
- **wasm-pack** (`curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh`)

### Build & Run Locally

1. **Clone the repository**:
   ```bash
   git clone https://github.com/Robinlovelace/bicischools-web.git
   cd bicischools-web
   ```

2. **Build the WebAssembly engine**:
   ```bash
   ./scripts/build-wasm.sh
   ```

3. **Start the development server**:
   ```bash
   cd web
   npm install
   npm run dev
   ```
   Open `http://localhost:5173` in your browser.

4. **Build static production bundle**:
   ```bash
   npm run build
   ```
   The static site will be output to `web/dist/`.

---

## CLI Usage

You can also run the core engine directly from the command line:

```bash
cargo run --release -p bicischools-cli -- \
  --osm-file data/lisbon_network.json \
  --school-lng -9.12191 \
  --school-lat 38.76714 \
  --profile quiet \
  --min-trips 3.0 \
  --max-routes 3 \
  --output results.json
```

---

## Citation & Acknowledgments

If you use this tool or methodology in your research or planning, please cite:

```bibtex
@article{talbot2025bicischools,
  title={Methods for prioritising 'bike buses' for safe and sociable cycling to school},
  author={Talbot, Joey and F{\'e}lix, Rosa and Fonseca-Zamora, Juan and Lino, Francisco and Garcia, Camila and Marcelino, Catarina and Lovelace, Robin},
  journal={Journal of Transport Geography},
  year={2025}
}
```

---

## License

MIT License (c) 2024-2026 Joey Talbot, Rosa Félix, Juan Fonseca-Zamora, Francisco Lino, Camila Garcia, Catarina Marcelino, Robin Lovelace.
