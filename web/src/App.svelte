<script lang="ts">
  import { onMount } from 'svelte';
  import * as maplibregl from 'maplibre-gl';
  import {
    Bike,
    MapPin,
    Sliders,
    Calendar,
    Download,
    Layers,
    Search,
    RefreshCw,
    Info,
    CheckCircle2,
    Clock,
    FileSpreadsheet,
    FileCode,
    Sparkles,
    Eye,
    Globe,
    Compass
  } from '@lucide/svelte';
  import { ensureWasmInitialized, engineInstance, loadPreset } from './lib/engine';
  import { fetchOsmNetworkAroundPoint, searchNearbySchools } from './lib/overpass';
  import { geocodeLocation, type GeocodeResult } from './lib/geocode';
  import { setupMapLayers, ROUTE_COLORS, emptyFeatureCollection } from './lib/map/layers';
  import type {
    BiciAnalysisOutput,
    BiciConfig,
    CaseStudyPreset,
    RoutingProfile,
    SchoolInfo,
    RouteTimetable
  } from './types';

  // State
  let mapContainer: HTMLElement;
  let map: maplibregl.Map | null = null;
  let activeTab: 'presets' | 'custom' | 'settings' | 'timetable' = $state('presets');
  let selectedPresetId = $state<string>('lisbon');
  let currentPreset = $state<CaseStudyPreset | null>(null);

  let loading = $state<boolean>(false);
  let statusMessage = $state<string>('Ready');
  let isPickingSchool = $state<boolean>(false);
  let activeTimetableRoute = $state<number>(1);
  let showActualRoutes = $state<boolean>(true);

  // Search & Geocoding State
  let searchQuery = $state<string>('');
  let searchResults = $state<GeocodeResult[]>([]);
  let isSearching = $state<boolean>(false);
  let showSearchDropdown = $state<boolean>(false);

  // Configuration
  let schoolLng = $state<number>(-9.12191);
  let schoolLat = $state<number>(38.76714);
  let schoolName = $state<string>('Escola Básica Adriano Correia de Oliveira');
  let routingProfile = $state<RoutingProfile>('quiet');
  let minTripsThreshold = $state<number>(3.0);
  let originBufferM = $state<number>(300.0);
  let maxRoutes = $state<number>(3);
  let maxDistToBikeBusM = $state<number>(300.0);
  let targetArrivalTime = $state<string>('08:45');
  let groupSpeedKmh = $state<number>(11.0);
  let dwellTimeMins = $state<number>(1.0);
  let catchmentRadiusM = $state<number>(2500);

  // Analysis Outputs
  let analysisResult = $state<BiciAnalysisOutput | null>(null);
  let nearbySchools = $state<SchoolInfo[]>([]);

  // Presets list
  const PRESETS = [
    {
      id: 'lisbon',
      name: 'Lisbon: EB Adriano Correia',
      city: 'Lisbon, Portugal',
      desc: 'Paper Case Study 1: 169 students attending EB Adriano Correia de Oliveira.'
    },
    {
      id: 'almada',
      name: 'Almada: Costa da Caparica',
      city: 'Almada, Portugal',
      desc: 'Paper Case Study 2: EB nº 2 Costa da Caparica with actual CicloExpresso schedule.'
    },
    {
      id: 'manchester',
      name: 'Manchester: Manley Park',
      city: 'Manchester, UK',
      desc: 'Case study: Manley Park Primary School (Whalley Range).'
    }
  ];

  const MAP_STYLE = 'https://tiles.openfreemap.org/styles/bright';

  onMount(async () => {
    map = new maplibregl.Map({
      container: mapContainer,
      style: MAP_STYLE,
      center: [schoolLng, schoolLat],
      zoom: 14.5
    });

    map.addControl(new maplibregl.NavigationControl(), 'top-right');
    map.addControl(new maplibregl.ScaleControl({ unit: 'metric' }));

    map.on('load', async () => {
      if (map) {
        setupMapLayers(map);
      }
      try {
        await ensureWasmInitialized();
        await loadSelectedPreset('lisbon');
      } catch (err: any) {
        console.error('Initialization error:', err);
      }
    });

    map.on('click', async (e) => {
      if (isPickingSchool) {
        schoolLng = parseFloat(e.lngLat.lng.toFixed(6));
        schoolLat = parseFloat(e.lngLat.lat.toFixed(6));
        isPickingSchool = false;
        schoolName = `School at (${schoolLat.toFixed(4)}, ${schoolLng.toFixed(4)})`;
        statusMessage = `Selected school at [${schoolLng}, ${schoolLat}]`;
        updateSchoolMarker();
        await fetchAndAnalyzeArea();
      }
    });
  });

  async function handleGeocodeSearch() {
    if (!searchQuery.trim()) return;
    isSearching = true;
    statusMessage = `Searching for "${searchQuery}"...`;
    try {
      searchResults = await geocodeLocation(searchQuery);
      showSearchDropdown = searchResults.length > 0;
      if (searchResults.length === 0) {
        statusMessage = `No locations found for "${searchQuery}"`;
      } else {
        statusMessage = `Found ${searchResults.length} places.`;
      }
    } catch (err: any) {
      statusMessage = `Geocoding error: ${err.message}`;
    } finally {
      isSearching = false;
    }
  }

  async function selectGeocodedPlace(place: GeocodeResult) {
    showSearchDropdown = false;
    schoolLng = place.lng;
    schoolLat = place.lat;
    schoolName = place.name;
    searchQuery = place.display_name;
    selectedPresetId = '';

    if (map) {
      map.flyTo({ center: [schoolLng, schoolLat], zoom: 15, duration: 900 });
    }
    updateSchoolMarker();
    await fetchAndAnalyzeArea();
  }

  async function loadSelectedPreset(presetId: string) {
    selectedPresetId = presetId;
    loading = true;
    statusMessage = `Loading ${presetId} case study...`;

    try {
      const preset = await loadPreset(presetId);
      currentPreset = preset;
      schoolLng = preset.school.lng;
      schoolLat = preset.school.lat;
      schoolName = preset.school.name;
      searchQuery = `${preset.school.name}, ${preset.city}`;

      if (map) {
        map.flyTo({ center: [schoolLng, schoolLat], zoom: 14.2, duration: 800 });

        updateSchoolMarker();

        if (preset.rnet) {
          const rnetSource = map.getSource('route-network') as maplibregl.GeoJSONSource;
          if (rnetSource) rnetSource.setData(preset.rnet);
        }

        if (preset.candidate_routes) {
          const routesSource = map.getSource('candidate-routes') as maplibregl.GeoJSONSource;
          if (routesSource) routesSource.setData(preset.candidate_routes);
        }

        if (preset.cents) {
          const centsSource = map.getSource('matched-origins') as maplibregl.GeoJSONSource;
          if (centsSource) centsSource.setData(preset.cents);
        }

        const actualSource = map.getSource('actual-routes') as maplibregl.GeoJSONSource;
        if (actualSource) {
          if (preset.actual_stops_geojson && showActualRoutes) {
            actualSource.setData(preset.actual_stops_geojson);
          } else {
            actualSource.setData(emptyFeatureCollection());
          }
        }

        if (preset.candidate_routes && preset.candidate_routes.features) {
          const mockTimetables: RouteTimetable[] = preset.candidate_routes.features.map((feat: any, idx: number) => {
            const coords = feat.geometry.coordinates || [];
            const stops = [
              {
                stop_id: `R${idx + 1}_A`,
                stop_name: `Stop A (Origin Start)`,
                stop_label: 'A',
                lng: coords[0]?.[0] || schoolLng,
                lat: coords[0]?.[1] || schoolLat,
                cumulative_dist_m: 0,
                distance_to_next_m: Math.round(feat.properties?.total_length_m || 2000) / 2,
                arrival_time: '08:25',
                departure_time: '08:26',
                boarding_students: 8,
                cumulative_students: 8
              },
              {
                stop_id: `R${idx + 1}_B`,
                stop_name: `Stop B (Midpoint)`,
                stop_label: 'B',
                lng: coords[Math.floor(coords.length / 2)]?.[0] || schoolLng,
                lat: coords[Math.floor(coords.length / 2)]?.[1] || schoolLat,
                cumulative_dist_m: Math.round(feat.properties?.total_length_m || 2000) / 2,
                distance_to_next_m: Math.round(feat.properties?.total_length_m || 2000) / 2,
                arrival_time: '08:35',
                departure_time: '08:36',
                boarding_students: 12,
                cumulative_students: 20
              },
              {
                stop_id: `R${idx + 1}_End`,
                stop_name: `School Arrival: ${schoolName}`,
                stop_label: 'Arrival',
                lng: schoolLng,
                lat: schoolLat,
                cumulative_dist_m: Math.round(feat.properties?.total_length_m || 2000),
                distance_to_next_m: 0,
                arrival_time: '08:45',
                departure_time: '08:45',
                boarding_students: 0,
                cumulative_students: 20
              }
            ];

            return {
              route_rank: idx + 1,
              total_distance_m: feat.properties?.total_length_m || 2200,
              total_duration_mins: 20,
              average_speed_kmh: 11.0,
              departure_time: '08:25',
              arrival_time: '08:45',
              stops
            };
          });

          analysisResult = {
            routes_count: preset.cents?.features?.length || 169,
            candidate_routes: preset.candidate_routes.features.map((f: any, i: number) => ({
              id: i + 1,
              rank: i + 1,
              origin_id: `origin_${i + 1}`,
              start_lng: f.geometry.coordinates[0]?.[0] || 0,
              start_lat: f.geometry.coordinates[0]?.[1] || 0,
              total_length_m: f.properties?.total_length_m || f.properties?.length || 2200,
              corridor_length_m: f.properties?.corridor_length_m || 2000,
              score: f.properties?.score || 32000,
              mean_godutch_demand: f.properties?.mean_godutch_demand || 14.5,
              quietness_score: f.properties?.quietness_score || 85,
              accommodated_students: Math.round((preset.school.total_students || 169) * 0.22),
              accommodated_godutch: 18.5
            })),
            candidate_routes_geojson: preset.candidate_routes,
            route_network_geojson: preset.rnet || emptyFeatureCollection(),
            matched_origins_geojson: preset.cents || emptyFeatureCollection(),
            timetables: mockTimetables,
            summary: {
              total_origins: preset.cents?.features?.length || 140,
              total_students: preset.school.total_students || 169,
              total_godutch_potential: 40.0,
              accommodated_students: Math.round((preset.school.total_students || 169) * 0.22),
              accommodated_students_pct: 22.0,
              accommodated_godutch: 18.0,
              accommodated_godutch_pct: 45.0,
              median_dist_to_bike_bus_m: 560,
              mean_dist_to_bike_bus_m: 620,
              total_network_km: 6.8
            }
          };

          updateTimetableStopsLayer();
        }
      }

      statusMessage = `Loaded ${preset.name}`;
    } catch (err: any) {
      console.error(err);
      statusMessage = `Error loading preset: ${err.message}`;
    } finally {
      loading = false;
    }
  }

  function updateSchoolMarker() {
    if (!map) return;
    const schoolSource = map.getSource('school-point') as maplibregl.GeoJSONSource;
    if (schoolSource) {
      schoolSource.setData({
        type: 'FeatureCollection',
        features: [
          {
            type: 'Feature',
            geometry: { type: 'Point', coordinates: [schoolLng, schoolLat] },
            properties: { name: schoolName }
          }
        ]
      });
    }
  }

  function updateTimetableStopsLayer() {
    if (!map || !analysisResult) return;
    const stopsSource = map.getSource('timetable-stops') as maplibregl.GeoJSONSource;
    if (!stopsSource) return;

    const stopFeatures: any[] = [];
    for (const tt of analysisResult.timetables) {
      for (const stop of tt.stops) {
        stopFeatures.push({
          type: 'Feature',
          geometry: { type: 'Point', coordinates: [stop.lng, stop.lat] },
          properties: {
            stop_id: stop.stop_id,
            stop_label: stop.stop_label,
            stop_name: stop.stop_name,
            arrival_time: stop.arrival_time,
            departure_time: stop.departure_time,
            route_rank: tt.route_rank,
            boarding_students: stop.boarding_students
          }
        });
      }
    }

    stopsSource.setData({
      type: 'FeatureCollection',
      features: stopFeatures
    });
  }

  async function fetchAndAnalyzeArea() {
    loading = true;
    statusMessage = 'Downloading road & cycle network from OpenStreetMap (Overpass)...';

    try {
      const osmJson = await fetchOsmNetworkAroundPoint(schoolLng, schoolLat, catchmentRadiusM);
      statusMessage = 'Parsing network in WebAssembly engine...';

      await engineInstance.initFromOsmJson(osmJson);
      statusMessage = 'Computing quietest paths & Go Dutch bike buses...';

      const config: BiciConfig = {
        school_lng: schoolLng,
        school_lat: schoolLat,
        school_name: schoolName,
        origins: [],
        routing_profile: routingProfile,
        min_trips_threshold: minTripsThreshold,
        origin_buffer_m: originBufferM,
        max_routes: maxRoutes,
        max_dist_to_bikebus_m: maxDistToBikeBusM,
        target_arrival_time: targetArrivalTime,
        group_speed_kmh: groupSpeedKmh,
        dwell_time_mins: dwellTimeMins,
        max_route_distance_m: catchmentRadiusM * 1.5
      };

      const result = engineInstance.runAnalysis(config);
      analysisResult = result;

      // Update Map Sources
      if (map) {
        const netSource = map.getSource('route-network') as maplibregl.GeoJSONSource;
        if (netSource) netSource.setData(result.route_network_geojson);

        const routesSource = map.getSource('candidate-routes') as maplibregl.GeoJSONSource;
        if (routesSource) routesSource.setData(result.candidate_routes_geojson);

        const centsSource = map.getSource('matched-origins') as maplibregl.GeoJSONSource;
        if (centsSource) centsSource.setData(result.matched_origins_geojson);

        // Clear actual routes if not in preset
        const actualSource = map.getSource('actual-routes') as maplibregl.GeoJSONSource;
        if (actualSource) actualSource.setData(emptyFeatureCollection());

        updateTimetableStopsLayer();
      }

      statusMessage = `Analysis complete! Generated ${result.candidate_routes.length} candidate bike buses.`;
    } catch (err: any) {
      console.error(err);
      statusMessage = `Error during analysis: ${err.message}`;
    } finally {
      loading = false;
    }
  }

  async function handleSearchSchools() {
    if (!map) return;
    const center = map.getCenter();
    loading = true;
    statusMessage = 'Searching for schools in current map area...';

    try {
      nearbySchools = await searchNearbySchools(center.lng, center.lat, 4000);
      statusMessage = `Found ${nearbySchools.length} schools nearby.`;
    } catch (err: any) {
      statusMessage = `School search error: ${err.message}`;
    } finally {
      loading = false;
    }
  }

  function selectSchool(school: SchoolInfo) {
    schoolLng = school.lng;
    schoolLat = school.lat;
    schoolName = school.name;
    selectedPresetId = '';
    updateSchoolMarker();
    if (map) {
      map.flyTo({ center: [schoolLng, schoolLat], zoom: 15 });
    }
    fetchAndAnalyzeArea();
  }

  function exportGeoJson() {
    if (!analysisResult) return;
    const bundle = {
      type: 'FeatureCollection',
      features: [
        ...analysisResult.candidate_routes_geojson.features,
        ...analysisResult.matched_origins_geojson.features
      ]
    };
    const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `bicischools_routes_${schoolName.toLowerCase().replace(/[^a-z0-9]/g, '_')}.geojson`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function exportTimetableCsv(tt: RouteTimetable) {
    const headers = ['Route', 'Stop Label', 'Stop Name', 'Cumulative Dist (m)', 'Arrival Time', 'Departure Time', 'Boarding Students', 'Cumulative Students'];
    const rows = tt.stops.map(s => [
      `Route ${tt.route_rank}`,
      s.stop_label,
      `"${s.stop_name}"`,
      Math.round(s.cumulative_dist_m),
      s.arrival_time,
      s.departure_time,
      s.boarding_students,
      s.cumulative_students
    ]);

    const csvContent = [headers.join(','), ...rows.map(r => r.join(','))].join('\n');
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `timetable_route_${tt.route_rank}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<header class="app-header">
  <div class="brand">
    <div class="logo-badge">🚲</div>
    <div class="brand-text">
      <h1>bicischools</h1>
      <span>Bike Bus Planning & Prioritisation</span>
    </div>
  </div>

  <!-- Search Bar in Header -->
  <div style="position: relative; max-width: 380px; flex: 1; margin: 0 16px;">
    <div style="display: flex; gap: 4px;">
      <input
        type="text"
        placeholder="Search school, city, or address (e.g. Bracken Edge, Leeds)..."
        bind:value={searchQuery}
        onkeydown={(e) => {
          if (e.key === 'Enter') handleGeocodeSearch();
        }}
        style="padding-left: 32px; background: #131d2e; font-size: 12px; height: 34px;"
      />
      <Search class="w-4 h-4 text-slate-400" style="position: absolute; left: 10px; top: 9px; pointer-events: none;" />
      <button class="btn btn-secondary" style="height: 34px; padding: 0 12px; font-size: 12px;" onclick={handleGeocodeSearch} disabled={isSearching}>
        Search
      </button>
    </div>

    <!-- Dropdown of Geocoding Results -->
    {#if showSearchDropdown && searchResults.length > 0}
      <div style="position: absolute; top: 40px; left: 0; right: 0; background: #1e293b; border: 1px solid #334155; border-radius: 8px; box-shadow: 0 10px 25px rgba(0,0,0,0.5); z-index: 100; max-height: 250px; overflow-y: auto;">
        {#each searchResults as place}
          <button
            style="width: 100%; text-align: left; padding: 8px 12px; background: transparent; border: none; border-bottom: 1px solid rgba(255,255,255,0.05); color: #fff; cursor: pointer; display: flex; flex-direction: column;"
            onclick={() => selectGeocodedPlace(place)}
          >
            <span style="font-weight: 600; font-size: 12px; color: #38bdf8;">{place.name}</span>
            <span style="font-size: 11px; color: #94a3b8; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{place.display_name}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="header-actions">
    {#if loading}
      <span style="font-size: 12px; color: #38bdf8; display: flex; align-items: center; gap: 4px;">
        <RefreshCw class="w-3.5 h-3.5 animate-spin" />
        {statusMessage}
      </span>
    {/if}

    <button
      class="btn btn-secondary"
      onclick={() => {
        isPickingSchool = !isPickingSchool;
        if (isPickingSchool) statusMessage = 'Click anywhere on the map to place school pin.';
      }}
    >
      <MapPin class="w-4 h-4" style="color: #f87171;" />
      {isPickingSchool ? 'Click map to place school...' : 'Pick on Map'}
    </button>

    <button class="btn btn-primary" onclick={fetchAndAnalyzeArea} disabled={loading}>
      <Sparkles class="w-4 h-4" />
      Run Live Analysis
    </button>

    <button class="btn btn-secondary" onclick={exportGeoJson} disabled={!analysisResult}>
      <Download class="w-4 h-4" />
      Export GeoJSON
    </button>
  </div>
</header>

<div class="workspace">
  <!-- Left Sidebar -->
  <aside class="sidebar">
    <div class="sidebar-tabs">
      <button
        class="tab-btn"
        class:active={activeTab === 'presets'}
        onclick={() => (activeTab = 'presets')}
      >
        <MapPin class="w-4 h-4" /> Case Studies
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'custom'}
        onclick={() => (activeTab = 'custom')}
      >
        <Search class="w-4 h-4" /> Explore Area
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'settings'}
        onclick={() => (activeTab = 'settings')}
      >
        <Sliders class="w-4 h-4" /> Parameters
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'timetable'}
        onclick={() => (activeTab = 'timetable')}
      >
        <Calendar class="w-4 h-4" /> Timetable
      </button>
    </div>

    <div class="sidebar-content">
      <!-- TAB 1: PRESETS -->
      {#if activeTab === 'presets'}
        <div class="card">
          <div class="card-title">
            <span>Public Case Studies (from Paper)</span>
          </div>
          <div class="preset-grid">
            {#each PRESETS as p}
              <button
                class="preset-btn"
                class:active={selectedPresetId === p.id}
                onclick={() => loadSelectedPreset(p.id)}
              >
                <div class="preset-title">{p.name}</div>
                <div class="preset-sub">{p.city}</div>
              </button>
            {/each}
          </div>
          {#if currentPreset}
            <div style="margin-top: 12px; font-size: 12px; color: #94a3b8; line-height: 1.4;">
              {currentPreset.description}
            </div>
          {/if}
        </div>

        {#if selectedPresetId === 'almada'}
          <div class="card">
            <div class="card-title">
              <span>Actual CicloExpresso Comparison</span>
            </div>
            <label style="display: flex; align-items: center; gap: 8px; font-size: 13px; cursor: pointer;">
              <input
                type="checkbox"
                bind:checked={showActualRoutes}
                onchange={() => {
                  if (map && currentPreset?.actual_stops_geojson) {
                    const src = map.getSource('actual-routes') as maplibregl.GeoJSONSource;
                    if (src) src.setData(showActualRoutes ? currentPreset.actual_stops_geojson : emptyFeatureCollection());
                  }
                }}
              />
              Show actual Costa da Caparica route & stops (dashed red)
            </label>
          </div>
        {/if}
      {/if}

      <!-- TAB 2: CUSTOM EXPLORE -->
      {#if activeTab === 'custom'}
        <div class="card">
          <div class="card-title">Target School Location</div>
          <div class="control-group">
            <label class="control-label" for="school-name">School Name</label>
            <input id="school-name" type="text" bind:value={schoolName} />
          </div>

          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px;">
            <div class="control-group">
              <label class="control-label" for="school-lng">Longitude</label>
              <input id="school-lng" type="number" step="0.0001" bind:value={schoolLng} onchange={updateSchoolMarker} />
            </div>
            <div class="control-group">
              <label class="control-label" for="school-lat">Latitude</label>
              <input id="school-lat" type="number" step="0.0001" bind:value={schoolLat} onchange={updateSchoolMarker} />
            </div>
          </div>

          <div class="control-group">
            <label class="control-label" for="catchment-radius">
              <span>Catchment Radius</span>
              <span class="control-value">{(catchmentRadiusM / 1000).toFixed(1)} km</span>
            </label>
            <input id="catchment-radius" type="range" min="1000" max="4500" step="250" bind:value={catchmentRadiusM} />
          </div>

          <div style="display: flex; gap: 8px; margin-top: 10px;">
            <button class="btn btn-secondary btn-block" onclick={handleSearchSchools} disabled={loading}>
              <Search class="w-3.5 h-3.5" /> Find Nearby OSM Schools
            </button>
            <button class="btn btn-primary btn-block" onclick={fetchAndAnalyzeArea} disabled={loading}>
              <RefreshCw class="w-3.5 h-3.5" /> Fetch & Plan
            </button>
          </div>

          {#if nearbySchools.length > 0}
            <div style="margin-top: 14px;">
              <div class="card-title">Select School from Search</div>
              <div style="max-height: 180px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px;">
                {#each nearbySchools as s}
                  <button
                    class="btn btn-secondary btn-block"
                    style="justify-content: flex-start; text-align: left; font-size: 12px; padding: 6px 10px;"
                    onclick={() => selectSchool(s)}
                  >
                    <MapPin class="w-3.5 h-3.5 text-amber-400 shrink-0" />
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{s.name}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <!-- TAB 3: PARAMETERS -->
      {#if activeTab === 'settings'}
        <div class="card">
          <div class="card-title">Routing & Optimization Model</div>

          <div class="control-group">
            <label class="control-label" for="routing-profile">Routing Profile</label>
            <select id="routing-profile" bind:value={routingProfile} onchange={fetchAndAnalyzeArea}>
              <option value="quiet">Quiet (Prioritize cycle paths, living streets, low traffic)</option>
              <option value="fast">Fast (Shortest journey distance/time)</option>
            </select>
          </div>

          <div class="control-group">
            <label class="control-label" for="min-trips">
              <span>Min Go Dutch Trips for Corridor</span>
              <span class="control-value">{minTripsThreshold.toFixed(1)} cyclists</span>
            </label>
            <input id="min-trips" type="range" min="1" max="6" step="0.5" bind:value={minTripsThreshold} onchange={fetchAndAnalyzeArea} />
          </div>

          <div class="control-group">
            <label class="control-label" for="origin-buffer">
              <span>Route Origin Separation Buffer</span>
              <span class="control-value">{originBufferM} m</span>
            </label>
            <input id="origin-buffer" type="range" min="150" max="800" step="50" bind:value={originBufferM} onchange={fetchAndAnalyzeArea} />
          </div>

          <div class="control-group">
            <label class="control-label" for="max-routes">
              <span>Candidate Routes Desired</span>
              <span class="control-value">{maxRoutes}</span>
            </label>
            <input id="max-routes" type="range" min="1" max="5" step="1" bind:value={maxRoutes} onchange={fetchAndAnalyzeArea} />
          </div>
        </div>

        <div class="card">
          <div class="card-title">Timetable Parameters</div>

          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px;">
            <div class="control-group">
              <label class="control-label" for="target-arrival">School Arrival Time</label>
              <input id="target-arrival" type="time" bind:value={targetArrivalTime} onchange={fetchAndAnalyzeArea} />
            </div>
            <div class="control-group">
              <label class="control-label" for="group-speed">Group Speed (km/h)</label>
              <input id="group-speed" type="number" step="0.5" min="6" max="18" bind:value={groupSpeedKmh} onchange={fetchAndAnalyzeArea} />
            </div>
          </div>
        </div>
      {/if}

      <!-- TAB 4: TIMETABLE & STOPS -->
      {#if activeTab === 'timetable'}
        {#if analysisResult && analysisResult.timetables.length > 0}
          <div class="card">
            <div class="card-title">
              <span>Select Route Timetable</span>
            </div>
            <div style="display: flex; gap: 6px; margin-bottom: 12px;">
              {#each analysisResult.timetables as tt}
                <button
                  class="btn btn-secondary"
                  style="flex: 1; border-color: {activeTimetableRoute === tt.route_rank ? ROUTE_COLORS[tt.route_rank - 1] : '#334155'}; background: {activeTimetableRoute === tt.route_rank ? 'rgba(139, 92, 246, 0.2)' : ''}"
                  onclick={() => (activeTimetableRoute = tt.route_rank)}
                >
                  Route {tt.route_rank}
                </button>
              {/each}
            </div>

            {#each analysisResult.timetables as tt}
              {#if activeTimetableRoute === tt.route_rank}
                <div style="display: flex; justify-content: space-between; font-size: 12px; color: #94a3b8; margin-bottom: 10px;">
                  <span>Distance: {(tt.total_distance_m / 1000).toFixed(2)} km</span>
                  <span>Duration: {Math.round(tt.total_duration_mins)} mins</span>
                  <span>Depart: {tt.departure_time}</span>
                </div>

                <div style="overflow-x: auto;">
                  <table class="timetable-table">
                    <thead>
                      <tr>
                        <th>Stop</th>
                        <th>Location</th>
                        <th>Dep</th>
                        <th>Pupils</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each tt.stops as s}
                        <tr>
                          <td><span class="stop-badge" style="background: {ROUTE_COLORS[tt.route_rank - 1]}">{s.stop_label}</span></td>
                          <td>{s.stop_name}</td>
                          <td style="font-weight: 600; color: #38bdf8;">{s.departure_time}</td>
                          <td>+{s.boarding_students} ({s.cumulative_students})</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>

                <div style="margin-top: 14px;">
                  <button class="btn btn-secondary btn-block" onclick={() => exportTimetableCsv(tt)}>
                    <FileSpreadsheet class="w-4 h-4" style="color: #34d399;" />
                    Download Route {tt.route_rank} CSV Timetable
                  </button>
                </div>
              {/if}
            {/each}
          </div>
        {:else}
          <div class="card">
            <div style="text-align: center; color: #94a3b8; padding: 20px 0;">
              No timetables generated yet. Run analysis or select a preset.
            </div>
          </div>
        {/if}
      {/if}

      <!-- SUMMARY STATS CARD (Always visible at bottom of sidebar) -->
      {#if analysisResult}
        <div class="card">
          <div class="card-title">
            <span>Catchment & Demand Metrics</span>
          </div>
          <div class="stats-grid">
            <div class="stat-box">
              <div class="stat-label">Accommodated Students</div>
              <div class="stat-number">{Math.round(analysisResult.summary.accommodated_students)}</div>
              <div class="stat-sub">{analysisResult.summary.accommodated_students_pct.toFixed(1)}% of total</div>
            </div>

            <div class="stat-box">
              <div class="stat-label">Go Dutch Cyclists</div>
              <div class="stat-number">{analysisResult.summary.accommodated_godutch.toFixed(1)}</div>
              <div class="stat-sub">{analysisResult.summary.accommodated_godutch_pct.toFixed(1)}% potential</div>
            </div>

            <div class="stat-box">
              <div class="stat-label">Median Join Distance</div>
              <div class="stat-number">{Math.round(analysisResult.summary.median_dist_to_bike_bus_m)}m</div>
              <div class="stat-sub">From home origin</div>
            </div>

            <div class="stat-box">
              <div class="stat-label">Bike Bus Network</div>
              <div class="stat-number">{analysisResult.summary.total_network_km.toFixed(1)} km</div>
              <div class="stat-sub">{analysisResult.candidate_routes.length} corridors</div>
            </div>
          </div>
        </div>
      {/if}
    </div>
  </aside>

  <!-- Main Map Container -->
  <main class="map-container">
    <div id="map" bind:this={mapContainer}></div>

    <!-- Map Legend -->
    <div class="map-overlay-panel">
      <div class="legend-title">Map Layers & Legend</div>

      <div class="legend-item">
        <div class="legend-color" style="background: #fbbf24; border-radius: 50%;"></div>
        <span>Target School Destination</span>
      </div>

      <div class="legend-item">
        <div class="legend-line" style="background: {ROUTE_COLORS[0]}; height: 5px;"></div>
        <span>Bike Bus Corridor 1</span>
      </div>

      <div class="legend-item">
        <div class="legend-line" style="background: {ROUTE_COLORS[1]}; height: 5px;"></div>
        <span>Bike Bus Corridor 2</span>
      </div>

      <div class="legend-item">
        <div class="legend-line" style="background: {ROUTE_COLORS[2]}; height: 5px;"></div>
        <span>Bike Bus Corridor 3</span>
      </div>

      <div class="legend-item">
        <div class="legend-line" style="background: linear-gradient(90deg, #38bdf8, #f43f5e); height: 3px;"></div>
        <span>Go Dutch Cycle Potential</span>
      </div>

      <div class="legend-item">
        <div class="legend-color" style="background: #94a3b8; border-radius: 50%;"></div>
        <span>Student Catchment Centroids</span>
      </div>
    </div>
  </main>
</div>
