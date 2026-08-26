import { WasmBiciEngine, init as initWasm } from './wasm/bicischools_wasm';
import type { BiciAnalysisOutput, BiciConfig, CaseStudyPreset, OriginInput } from '../types';

let wasmInitialized = false;

export async function ensureWasmInitialized(): Promise<void> {
  if (!wasmInitialized) {
    try {
      initWasm();
    } catch {
      // already initialized
    }
    wasmInitialized = true;
  }
}

export class BiciEngineWrapper {
  private engine: WasmBiciEngine | null = null;

  async initFromOsmJson(osmJson: string): Promise<void> {
    await ensureWasmInitialized();
    this.engine = new WasmBiciEngine(osmJson);
  }

  isReady(): boolean {
    return this.engine !== null;
  }

  runAnalysis(config: BiciConfig): BiciAnalysisOutput {
    if (!this.engine) {
      throw new Error('Engine not initialized with OSM network data');
    }
    const configJson = JSON.stringify(config);
    const resultJson = this.engine.runAnalysis(configJson);
    return JSON.parse(resultJson);
  }

  generateSyntheticOrigins(
    schoolLng: number,
    schoolLat: number,
    count: number = 80,
    radiusM: number = 2500
  ): OriginInput[] {
    if (!this.engine) {
      return [];
    }
    const originsJson = this.engine.generateSyntheticOrigins(schoolLng, schoolLat, count, radiusM);
    return JSON.parse(originsJson);
  }
}

export const engineInstance = new BiciEngineWrapper();

/**
 * Fetch and load a pre-computed case study preset
 */
export async function loadPreset(presetId: string): Promise<CaseStudyPreset> {
  const base = import.meta.env.BASE_URL || './';
  const cleanBase = base.endsWith('/') ? base : `${base}/`;
  const response = await fetch(`${cleanBase}presets/${presetId}.json`);
  if (!response.ok) {
    throw new Error(`Failed to load preset ${presetId}: HTTP ${response.status}`);
  }
  return await response.json();
}
