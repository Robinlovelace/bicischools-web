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
    Compass,
    PanelLeftClose,
    PanelLeftOpen,
    X,
    Shuffle,
    Edit3,
    Plus,
    Trash2,
    Save,
    Check
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
  let sidebarOpen = $state<boolean>(true);

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
  const DISTANCE_STEPS_M = [200, 300, 400, 500, 700, 1000, 2000, 3000, 4000, 5000];

  function formatDistance(m: number): string {
    if (m < 1000) {
      return `${m}m`;
    } else {
      const km = m / 1000;
      return `${km % 1 === 0 ? km.toFixed(0) : km.toFixed(1)} km`;
    }
  }

  function findClosestStepIndex(m: number): number {
    let closestIdx = 0;
    let minDiff = Infinity;
    for (let i = 0; i < DISTANCE_STEPS_M.length; i++) {
      const diff = Math.abs(DISTANCE_STEPS_M[i] - m);
      if (diff < minDiff) {
        minDiff = diff;
        closestIdx = i;
      }
    }
    return closestIdx;
  }

  let originBufferM = $state<number>(300.0);
  let maxRoutes = $state<number>(3);
  let maxDistToBikeBusM = $state<number>(300.0);
  let targetArrivalTime = $state<string>('08:45');
  let groupSpeedKmh = $state<number>(11.0);
  let dwellTimeMins = $state<number>(1.0);
  let catchmentRadiusM = $state<number>(1000);
  let circuity = $state<number>(1.25);
  let maxStraightLineDistM = $state<number>(1000);
  let maxSharedOverlapPct = $state<number>(40);
  let straightDistStepIndex = $state<number>(5);
  let catchmentStepIndex = $state<number>(5);
  let randomSeed = $state<number>(42);
  let isEditingRoute = $state<boolean>(false);

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

        const popup = new maplibregl.Popup({
          closeButton: false,
          closeOnClick: false,
          offset: 12
        });

        // 1. Candidate Route hover tooltip
        map.on('mouseenter', 'candidate-routes-line', (e) => {
          if (!map || !e.features || !e.features[0]) return;
          map.getCanvas().style.cursor = 'pointer';
          const f = e.features[0];
          const p = f.properties || {};
          const rank = p.rank || 1;
          const color = ROUTE_COLORS[rank - 1] || '#8b5cf6';
          const lenKm = ((p.total_length_m || p.length || 2000) / 1000).toFixed(2);
          const score = Math.round(p.score || 0);
          const demand = (p.mean_godutch_demand || 0).toFixed(1);
          const quiet = Math.round(p.quietness_score || 80);

          popup
            .setLngLat(e.lngLat)
            .setHTML(`
              <div style="font-family: system-ui, sans-serif; padding: 6px 8px; min-width: 170px;">
                <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 4px;">
                  <span style="display: inline-block; width: 10px; height: 10px; border-radius: 50%; background: ${color};"></span>
                  <strong style="color: #0f172a; font-size: 13px;">Bike Bus Corridor ${rank}</strong>
                </div>
                <div style="font-size: 11px; color: #475569; display: grid; grid-template-columns: 1fr 1fr; gap: 4px 8px;">
                  <span>Distance: <strong>${lenKm} km</strong></span>
                  <span>Quietness: <strong>${quiet}%</strong></span>
                  <span>Demand: <strong>${demand} pupils</strong></span>
                  <span>Score: <strong>${score}</strong></span>
                </div>
              </div>
            `)
            .addTo(map);
        });

        map.on('mouseleave', 'candidate-routes-line', () => {
          if (!map) return;
          map.getCanvas().style.cursor = '';
          popup.remove();
        });

        // 2. Timetable Stops hover tooltip
        map.on('mouseenter', 'timetable-stops-inner', (e) => {
          if (!map || !e.features || !e.features[0]) return;
          map.getCanvas().style.cursor = 'pointer';
          const f = e.features[0];
          const p = f.properties || {};
          const rank = p.route_rank || 1;
          const color = ROUTE_COLORS[rank - 1] || '#8b5cf6';
          const label = p.stop_label || 'A';
          const name = p.stop_name || `Stop ${label}`;
          const dep = p.departure_time || '08:30';
          const pupils = p.boarding_students !== undefined ? p.boarding_students : 5;

          popup
            .setLngLat(e.lngLat)
            .setHTML(`
              <div style="font-family: system-ui, sans-serif; padding: 6px 8px; min-width: 160px;">
                <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 4px;">
                  <span style="background: ${color}; color: #fff; font-size: 10px; font-weight: bold; border-radius: 4px; padding: 1px 5px;">${label}</span>
                  <strong style="color: #0f172a; font-size: 12px;">${name}</strong>
                </div>
                <div style="font-size: 11px; color: #475569;">
                  <div>Departure: <strong style="color: #0284c7;">${dep}</strong></div>
                  <div>Boarding: <strong>+${pupils} pupils</strong></div>
                </div>
              </div>
            `)
            .addTo(map);
        });

        map.on('mouseleave', 'timetable-stops-inner', () => {
          if (!map) return;
          map.getCanvas().style.cursor = '';
          popup.remove();
        });

        // 3. School Marker hover tooltip
        map.on('mouseenter', 'school-marker-circle', (e) => {
          if (!map || !e.features || !e.features[0]) return;
          map.getCanvas().style.cursor = 'pointer';
          const name = schoolName || 'Target Destination School';

          popup
            .setLngLat(e.lngLat)
            .setHTML(`
              <div style="font-family: system-ui, sans-serif; padding: 6px 8px;">
                <div style="display: flex; align-items: center; gap: 6px;">
                  <span style="font-size: 14px;">🏫</span>
                  <strong style="color: #0f172a; font-size: 12px;">${name}</strong>
                </div>
                <div style="font-size: 11px; color: #64748b; margin-top: 2px;">School Destination</div>
              </div>
            `)
            .addTo(map);
        });

        map.on('mouseleave', 'school-marker-circle', () => {
          if (!map) return;
          map.getCanvas().style.cursor = '';
          popup.remove();
        });
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

  async function handleGeocodeSearch(fromHeader: boolean = true) {
    if (!searchQuery.trim()) return;
    isSearching = true;
    statusMessage = `Searching for "${searchQuery}"...`;
    try {
      searchResults = await geocodeLocation(searchQuery);
      if (fromHeader) {
        showSearchDropdown = searchResults.length > 0;
      } else {
        showSearchDropdown = false;
      }
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
            const rank = idx + 1;
            const isLisbon = preset.id === 'lisbon';
            const isAlmada = preset.id === 'almada';

            const startName = isLisbon
              ? 'Av. Brasil near Rotunda do Relógio (Start)'
              : isAlmada
              ? 'Av. 1º de Maio near junction with R. dos Pescadores (Start)'
              : `Street Origin near Junction (Start)`;

            const midName = isLisbon
              ? 'Rua Cidade de Quelimane near Rua Cidade de Bissau'
              : isAlmada
              ? 'Av. Afonso de Albuquerque near junction with R. de Lisboa'
              : `Main Corridor, stop 1`;

            const stops = [
              {
                stop_id: `R${rank}_A`,
                stop_name: startName,
                stop_label: `${rank}A`,
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
                stop_id: `R${rank}_B`,
                stop_name: midName,
                stop_label: `${rank}B`,
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
                stop_id: `R${rank}_End`,
                stop_name: `${schoolName} (Arrival)`,
                stop_label: 'Arr',
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
        max_route_distance_m: catchmentRadiusM * 1.5,
        circuity: circuity,
        max_straight_line_dist_m: maxStraightLineDistM,
        max_shared_overlap_pct: maxSharedOverlapPct,
        seed: randomSeed
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
        if (isEditingRoute) updateEditMarkers();
      }

      statusMessage = `Analysis complete! Generated ${result.candidate_routes.length} candidate bike buses.`;
    } catch (err: any) {
      console.error(err);
      statusMessage = `Error during analysis: ${err.message}`;
    } finally {
      loading = false;
    }
  }

  function parseHhmmToSecs(hhmm: string): number {
    const parts = hhmm.trim().split(':');
    if (parts.length === 2) {
      const h = parseInt(parts[0], 10) || 8;
      const m = parseInt(parts[1], 10) || 45;
      return h * 3600 + m * 60;
    }
    return 8 * 3600 + 45 * 60;
  }

  function secsToHhmm(secs: number): string {
    const totalMins = Math.round(secs / 60);
    const positiveMins = ((totalMins % (24 * 60)) + (24 * 60)) % (24 * 60);
    const h = Math.floor(positiveMins / 60);
    const m = positiveMins % 60;
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}`;
  }

  function haversineDistanceM(lng1: number, lat1: number, lng2: number, lat2: number): number {
    const R = 6371000;
    const dLat = ((lat2 - lat1) * Math.PI) / 180;
    const dLng = ((lng2 - lng1) * Math.PI) / 180;
    const a =
      Math.sin(dLat / 2) * Math.sin(dLat / 2) +
      Math.cos((lat1 * Math.PI) / 180) *
        Math.cos((lat2 * Math.PI) / 180) *
        Math.sin(dLng / 2) *
        Math.sin(dLng / 2);
    const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
    return R * c;
  }

  let editMarkers: maplibregl.Marker[] = [];

  function updateEditMarkers() {
    editMarkers.forEach((m) => m.remove());
    editMarkers = [];
    if (!map || !isEditingRoute || !analysisResult) return;

    const currentTt = analysisResult.timetables.find((t) => t.route_rank === activeTimetableRoute);
    if (!currentTt) return;

    const rank = activeTimetableRoute;
    const color = ROUTE_COLORS[rank - 1] || '#8b5cf6';

    currentTt.stops.forEach((stop, idx) => {
      const el = document.createElement('div');
      el.className = 'edit-stop-marker';
      el.innerHTML = `<span>${stop.stop_label}</span>`;
      el.style.background = color;
      el.style.color = '#fff';
      el.style.fontWeight = 'bold';
      el.style.fontSize = '10px';
      el.style.width = '26px';
      el.style.height = '26px';
      el.style.borderRadius = '50%';
      el.style.display = 'flex';
      el.style.alignItems = 'center';
      el.style.justifyContent = 'center';
      el.style.border = '2px solid #fff';
      el.style.boxShadow = '0 2px 8px rgba(0,0,0,0.5)';
      el.style.cursor = 'grab';

      const marker = new maplibregl.Marker({ element: el, draggable: true })
        .setLngLat([stop.lng, stop.lat])
        .addTo(map!);

      marker.on('dragend', () => {
        const newLngLat = marker.getLngLat();
        stop.lng = newLngLat.lng;
        stop.lat = newLngLat.lat;
        recalculateTimetable(currentTt);
        updateTimetableStopsLayer();
      });

      editMarkers.push(marker);
    });
  }

  function recalculateTimetable(tt: RouteTimetable) {
    if (!tt || tt.stops.length < 2) return;
    const speedMps = (groupSpeedKmh * 1000.0) / 3600.0;

    let cumDist = 0;
    tt.stops[0].cumulative_dist_m = 0;
    for (let i = 0; i < tt.stops.length - 1; i++) {
      const d = haversineDistanceM(tt.stops[i].lng, tt.stops[i].lat, tt.stops[i + 1].lng, tt.stops[i + 1].lat);
      tt.stops[i].distance_to_next_m = Math.round(d);
      cumDist += d;
      tt.stops[i + 1].cumulative_dist_m = Math.round(cumDist);
    }
    tt.stops[tt.stops.length - 1].distance_to_next_m = 0;
    tt.total_distance_m = Math.round(cumDist);

    const targetSecs = parseHhmmToSecs(targetArrivalTime);
    const n = tt.stops.length;
    let currentSecs = targetSecs;
    tt.stops[n - 1].arrival_time = secsToHhmm(currentSecs);
    tt.stops[n - 1].departure_time = secsToHhmm(currentSecs);

    for (let i = n - 2; i >= 0; i--) {
      const travelSecs = tt.stops[i].distance_to_next_m / speedMps;
      currentSecs -= travelSecs;
      tt.stops[i].departure_time = secsToHhmm(currentSecs);
      if (i > 0) {
        currentSecs -= dwellTimeMins * 60;
        tt.stops[i].arrival_time = secsToHhmm(currentSecs);
      } else {
        tt.stops[i].arrival_time = tt.stops[i].departure_time;
      }
    }

    tt.departure_time = tt.stops[0].departure_time;
    tt.total_duration_mins = Math.round((targetSecs - currentSecs) / 60);

    let cumPupils = 0;
    for (const s of tt.stops) {
      cumPupils += s.boarding_students;
      s.cumulative_students = cumPupils;
    }
  }

  function toggleRouteEditor() {
    isEditingRoute = !isEditingRoute;
    updateEditMarkers();
  }

  function removeStopFromActiveRoute(stopIndex: number) {
    if (!analysisResult) return;
    const currentTt = analysisResult.timetables.find((t) => t.route_rank === activeTimetableRoute);
    if (!currentTt || currentTt.stops.length <= 2 || stopIndex >= currentTt.stops.length - 1) return;

    currentTt.stops.splice(stopIndex, 1);

    const letters = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T'];
    currentTt.stops.forEach((s, idx) => {
      if (idx === currentTt.stops.length - 1) {
        s.stop_label = 'Arr';
      } else {
        s.stop_label = `${activeTimetableRoute}${letters[idx] || 'S'}`;
      }
    });

    recalculateTimetable(currentTt);
    updateTimetableStopsLayer();
    updateEditMarkers();
  }

  function addPickupStopToActiveRoute() {
    if (!analysisResult || !map) return;
    const currentTt = analysisResult.timetables.find((t) => t.route_rank === activeTimetableRoute);
    if (!currentTt || currentTt.stops.length < 2) return;

    const center = map.getCenter();
    const insertIdx = Math.max(1, currentTt.stops.length - 1);
    const letters = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T'];

    const newStop: any = {
      stop_id: `R${activeTimetableRoute}_custom_${Date.now()}`,
      stop_name: `Custom Pickup Stop near Map Center`,
      stop_label: `${activeTimetableRoute}${letters[insertIdx] || 'S'}`,
      lng: center.lng,
      lat: center.lat,
      cumulative_dist_m: 0,
      distance_to_next_m: 0,
      arrival_time: '08:30',
      departure_time: '08:31',
      boarding_students: 5,
      cumulative_students: 0
    };

    currentTt.stops.splice(insertIdx, 0, newStop);

    currentTt.stops.forEach((s, idx) => {
      if (idx === currentTt.stops.length - 1) {
        s.stop_label = 'Arr';
      } else {
        s.stop_label = `${activeTimetableRoute}${letters[idx] || 'S'}`;
      }
    });

    recalculateTimetable(currentTt);
    updateTimetableStopsLayer();
    updateEditMarkers();
  }

  function handleRegenerate() {
    randomSeed = Math.floor(Math.random() * 100000) + 1;
    fetchAndAnalyzeArea();
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

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
    setTimeout(() => {
      if (map) map.resize();
    }, 320);
  }

  function handleWindowClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.header-search-container')) {
      showSearchDropdown = false;
    }
  }
</script>

<svelte:window onclick={handleWindowClick} />

<header class="app-header">
  <button class="brand" type="button" onclick={() => loadSelectedPreset('lisbon')} style="background: none; border: none; padding: 0; text-align: left;">
    <div class="brand-icon">
      <Bike class="w-5 h-5" />
    </div>
    <div class="brand-text">
      <h1>bici<span class="brand-accent">schools</span></h1>
      <span>Bike Bus Planning Platform</span>
    </div>
  </button>

  <div class="header-actions">
    <!-- Search Bar in Header (Right Aligned, Non-clashing) -->
    <div class="header-search-container">
      <div style="position: relative; display: flex; align-items: center;">
        <input
          type="text"
          placeholder="Search school or city (e.g. Bracken Edge, Leeds)..."
          bind:value={searchQuery}
          onfocus={() => { if (searchResults.length > 0) showSearchDropdown = true; }}
          onkeydown={(e) => {
            if (e.key === 'Enter') handleGeocodeSearch();
            if (e.key === 'Escape') showSearchDropdown = false;
          }}
          style="padding-left: 32px; padding-right: 64px; background: #131d2e; font-size: 12px; height: 34px; width: 100%; border-radius: 6px; border: 1px solid #334155; color: #f8fafc;"
        />
        <Search class="w-4 h-4 text-slate-400" style="position: absolute; left: 10px; pointer-events: none;" />
        
        {#if searchQuery}
          <button
            style="position: absolute; right: 54px; background: transparent; border: none; color: #94a3b8; cursor: pointer; display: flex; align-items: center; justify-content: center; padding: 4px;"
            onclick={() => { searchQuery = ''; showSearchDropdown = false; searchResults = []; }}
            title="Clear search"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        {/if}

        <button
          class="btn btn-secondary"
          style="position: absolute; right: 2px; height: 30px; padding: 0 10px; font-size: 11px; border-radius: 4px;"
          onclick={handleGeocodeSearch}
          disabled={isSearching}
        >
          {#if isSearching}
            <RefreshCw class="w-3 h-3 animate-spin" />
          {:else}
            Search
          {/if}
        </button>
      </div>

      <!-- Dropdown of Geocoding Results (Right Aligned over Map) -->
      {#if showSearchDropdown && searchResults.length > 0}
        <div class="search-dropdown">
          <div style="padding: 6px 12px; font-size: 11px; font-weight: 600; color: #94a3b8; border-bottom: 1px solid #334155; display: flex; justify-content: space-between; align-items: center;">
            <span>Search Locations</span>
            <button
              style="background: transparent; border: none; color: #94a3b8; cursor: pointer; font-size: 11px;"
              onclick={() => (showSearchDropdown = false)}
            >
              Close ✕
            </button>
          </div>
          {#each searchResults as place}
            <button
              class="search-result-item"
              onclick={() => selectGeocodedPlace(place)}
            >
              <span style="font-weight: 600; font-size: 12px; color: #38bdf8;">{place.name}</span>
              <span style="font-size: 11px; color: #94a3b8; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{place.display_name}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

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
      {#if loading}
        <RefreshCw class="w-4 h-4 animate-spin" />
        Calculating...
      {:else}
        <Sparkles class="w-4 h-4" />
        Run Live Analysis
      {/if}
    </button>

    <button class="btn btn-secondary" onclick={handleRegenerate} disabled={loading} title="Explore alternative bike bus corridors and pupil pickup combinations">
      <Shuffle class="w-4 h-4" style="color: #a855f7;" />
      Regenerate
    </button>

    <button class="btn btn-secondary" onclick={exportGeoJson} disabled={!analysisResult}>
      <Download class="w-4 h-4" />
      Export GeoJSON
    </button>
  </div>
</header>

{#if loading}
  <div class="top-loading-bar"></div>
{/if}

<div class="workspace">
  <!-- Left Sidebar -->
  <aside class="sidebar" class:collapsed={!sidebarOpen}>
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
          <div class="card-title">Search School or Address</div>
          <div style="display: flex; gap: 6px;">
            <input
              type="text"
              placeholder="e.g. Bracken Edge Leeds, Manley Park..."
              bind:value={searchQuery}
              onkeydown={(e) => {
                if (e.key === 'Enter') handleGeocodeSearch(false);
              }}
              style="font-size: 12px; height: 36px; background: #131d2e;"
            />
            <button class="btn btn-primary" onclick={() => handleGeocodeSearch(false)} disabled={isSearching} style="padding: 0 14px; flex-shrink: 0;">
              {#if isSearching}
                <RefreshCw class="w-3.5 h-3.5 animate-spin" />
              {:else}
                <Search class="w-3.5 h-3.5" />
              {/if}
            </button>
          </div>

          {#if searchResults.length > 0}
            <div style="margin-top: 10px; max-height: 200px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px;">
              {#each searchResults as place}
                <button
                  class="search-sidebar-item"
                  onclick={() => selectGeocodedPlace(place)}
                >
                  <div style="font-weight: 600; color: #38bdf8; font-size: 12px;">{place.name}</div>
                  <div style="font-size: 11px; color: #94a3b8; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{place.display_name}</div>
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <div class="card">
          <div class="card-title">Target School Coordinates & Catchment</div>
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
              <span class="control-value">{formatDistance(catchmentRadiusM)}</span>
            </label>
            <input
              id="catchment-radius"
              type="range"
              min="0"
              max={DISTANCE_STEPS_M.length - 1}
              step="1"
              bind:value={catchmentStepIndex}
              oninput={() => {
                catchmentRadiusM = DISTANCE_STEPS_M[catchmentStepIndex];
              }}
            />
          </div>

          <div style="display: flex; gap: 8px; margin-top: 10px;">
            <button class="btn btn-secondary btn-block" onclick={handleSearchSchools} disabled={loading}>
              <Search class="w-3.5 h-3.5" /> Find Nearby OSM Schools
            </button>
            <button class="btn btn-primary btn-block" onclick={fetchAndAnalyzeArea} disabled={loading}>
              {#if loading}
                <RefreshCw class="w-3.5 h-3.5 animate-spin" /> Calculating...
              {:else}
                <RefreshCw class="w-3.5 h-3.5" /> Fetch & Plan
              {/if}
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

          <div class="control-group">
            <label class="control-label" for="circuity">
              <span>Route Circuity / Pickup Meandering</span>
              <span class="control-value">{circuity.toFixed(2)}x</span>
            </label>
            <input id="circuity" type="range" min="1.0" max="2.2" step="0.05" bind:value={circuity} onchange={fetchAndAnalyzeArea} />
            <div style="font-size: 11px; color: #94a3b8; margin-top: 3px; line-height: 1.35;">
              Higher circuity attracts routes to detour and weave through dense residential streets to pick up more students instead of heading straight to school.
            </div>
          </div>

          <div class="control-group">
            <label class="control-label" for="max-straight-dist">
              <span>Max Straight-Line Distance from School</span>
              <span class="control-value">{formatDistance(maxStraightLineDistM)}</span>
            </label>
            <input
              id="max-straight-dist"
              type="range"
              min="0"
              max={DISTANCE_STEPS_M.length - 1}
              step="1"
              bind:value={straightDistStepIndex}
              oninput={() => {
                maxStraightLineDistM = DISTANCE_STEPS_M[straightDistStepIndex];
              }}
              onchange={() => {
                maxStraightLineDistM = DISTANCE_STEPS_M[straightDistStepIndex];
                fetchAndAnalyzeArea();
              }}
            />
            <div style="font-size: 11px; color: #94a3b8; margin-top: 3px; line-height: 1.35;">
              Euclidean radius buffer around school filtering candidate origins and route start points.
            </div>
          </div>

          <div class="control-group">
            <label class="control-label" for="max-overlap">
              <span>Max Shared Route Overlap</span>
              <span class="control-value">{maxSharedOverlapPct}%</span>
            </label>
            <input id="max-overlap" type="range" min="10" max="80" step="5" bind:value={maxSharedOverlapPct} onchange={fetchAndAnalyzeArea} />
            <div style="font-size: 11px; color: #94a3b8; margin-top: 3px; line-height: 1.35;">
              Restricts corridor duplication so candidate routes fan out across distinct neighborhoods (North, East, West).
            </div>
          </div>
        </div>

        <div class="card">
          <div class="card-title">Timetable Parameters</div>

          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin-bottom: 12px;">
            <div class="control-group">
              <label class="control-label" for="target-arrival">School Arrival Time</label>
              <input id="target-arrival" type="time" bind:value={targetArrivalTime} onchange={fetchAndAnalyzeArea} />
            </div>
            <div class="control-group">
              <label class="control-label" for="group-speed">Group Speed (km/h)</label>
              <input id="group-speed" type="number" step="0.5" min="6" max="18" bind:value={groupSpeedKmh} onchange={fetchAndAnalyzeArea} />
            </div>
          </div>

          <div class="control-group">
            <label class="control-label" for="dwell-time">
              <span>Time for collection at each stop</span>
              <span class="control-value">{dwellTimeMins < 1 ? Math.round(dwellTimeMins * 60) + ' sec' : dwellTimeMins.toFixed(1) + ' min' + (dwellTimeMins > 1 ? 's' : '')}</span>
            </label>
            <input
              id="dwell-time"
              type="range"
              min="0.5"
              max="4.0"
              step="0.5"
              bind:value={dwellTimeMins}
              onchange={fetchAndAnalyzeArea}
            />
            <div style="font-size: 11px; color: #94a3b8; margin-top: 3px; line-height: 1.35;">
              Pickup collection buffer allocated at each scheduled stop to gather, register, and safely board pupils.
            </div>
          </div>

          <div style="margin-top: 14px;">
            <button class="btn btn-secondary btn-block" onclick={handleRegenerate} disabled={loading}>
              <Shuffle class="w-4 h-4" style="color: #a855f7;" />
              Regenerate Alternative Corridors
            </button>
          </div>
        </div>
      {/if}

      <!-- TAB 4: TIMETABLE & STOPS -->
      {#if activeTab === 'timetable'}
        {#if analysisResult && analysisResult.timetables.length > 0}
          <div class="card">
            <div class="card-title" style="display: flex; justify-content: space-between; align-items: center;">
              <span>Route Timetable & Stops</span>
              <button
                class="btn btn-secondary"
                style="padding: 4px 10px; font-size: 12px; {isEditingRoute ? 'background: #0284c7; border-color: #38bdf8; color: #fff;' : ''}"
                onclick={toggleRouteEditor}
              >
                {#if isEditingRoute}
                  <Check class="w-3.5 h-3.5" /> Done Editing
                {:else}
                  <Edit3 class="w-3.5 h-3.5" /> Edit Route
                {/if}
              </button>
            </div>

            {#if isEditingRoute}
              <div style="background: rgba(2, 132, 199, 0.15); border: 1px solid #0284c7; border-radius: 8px; padding: 10px; margin-bottom: 12px; font-size: 12px; color: #bae6fd;">
                <div style="font-weight: 600; margin-bottom: 4px; display: flex; align-items: center; gap: 6px;">
                  <Edit3 class="w-4 h-4" /> Interactive Route Editor Active
                </div>
                <div>• Drag stop markers on the map to adjust pickup coordinates.</div>
                <div>• Edit stop names, departure times, or student counts below.</div>
                <div>• Remove or add stops as required.</div>
              </div>
            {/if}

            <div style="display: flex; gap: 6px; margin-bottom: 12px;">
              {#each analysisResult.timetables as tt}
                <button
                  class="btn btn-secondary"
                  style="flex: 1; border-color: {activeTimetableRoute === tt.route_rank ? ROUTE_COLORS[tt.route_rank - 1] : '#334155'}; background: {activeTimetableRoute === tt.route_rank ? 'rgba(139, 92, 246, 0.2)' : ''}"
                  onclick={() => {
                    activeTimetableRoute = tt.route_rank;
                    if (isEditingRoute) updateEditMarkers();
                  }}
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
                        {#if isEditingRoute}
                          <th>Action</th>
                        {/if}
                      </tr>
                    </thead>
                    <tbody>
                      {#each tt.stops as s, sIdx}
                        <tr>
                          <td><span class="stop-badge" style="background: {ROUTE_COLORS[tt.route_rank - 1]}">{s.stop_label}</span></td>
                          <td>
                            {#if isEditingRoute}
                              <input
                                type="text"
                                style="width: 100%; background: #0f172a; border: 1px solid #334155; border-radius: 4px; padding: 2px 6px; font-size: 11px; color: #f8fafc;"
                                bind:value={s.stop_name}
                              />
                            {:else}
                              {s.stop_name}
                            {/if}
                          </td>
                          <td style="font-weight: 600; color: #38bdf8;">
                            {s.departure_time}
                          </td>
                          <td>
                            {#if isEditingRoute}
                              <input
                                type="number"
                                min="0"
                                max="50"
                                style="width: 48px; background: #0f172a; border: 1px solid #334155; border-radius: 4px; padding: 2px 4px; font-size: 11px; color: #f8fafc;"
                                bind:value={s.boarding_students}
                                onchange={() => recalculateTimetable(tt)}
                              />
                            {:else}
                              +{s.boarding_students} ({s.cumulative_students})
                            {/if}
                          </td>
                          {#if isEditingRoute}
                            <td>
                              {#if sIdx < tt.stops.length - 1 && tt.stops.length > 2}
                                <button
                                  class="btn btn-secondary"
                                  style="padding: 2px 6px; font-size: 11px; color: #f87171; border-color: rgba(248, 113, 113, 0.4);"
                                  onclick={() => removeStopFromActiveRoute(sIdx)}
                                  title="Remove this stop"
                                >
                                  <Trash2 class="w-3 h-3" />
                                </button>
                              {/if}
                            </td>
                          {/if}
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>

                {#if isEditingRoute}
                  <div style="margin-top: 10px; display: flex; gap: 8px;">
                    <button class="btn btn-secondary" style="flex: 1; font-size: 12px;" onclick={addPickupStopToActiveRoute}>
                      <Plus class="w-3.5 h-3.5" style="color: #38bdf8;" />
                      Add Stop at Map Center
                    </button>
                    <button class="btn btn-primary" style="flex: 1; font-size: 12px;" onclick={toggleRouteEditor}>
                      <Check class="w-3.5 h-3.5" />
                      Save Edits
                    </button>
                  </div>
                {/if}

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
  <main class="map-container" style="position: relative;">
    {#if loading}
      <div class="map-loading-indicator">
        <div class="loading-spinner-ring">
          <RefreshCw class="w-5 h-5 animate-spin" />
        </div>
        <div class="loading-details">
          <div class="loading-title">Calculating Bike Buses</div>
          <div class="loading-sub">{statusMessage}</div>
        </div>
      </div>
    {/if}

    <button
      class="sidebar-toggle-btn"
      onclick={toggleSidebar}
      title={sidebarOpen ? "Collapse Sidebar (Full Map View)" : "Expand Sidebar (Controls & Timetable)"}
      aria-label="Toggle Sidebar"
    >
      {#if sidebarOpen}
        <PanelLeftClose class="w-4 h-4" />
      {:else}
        <PanelLeftOpen class="w-4 h-4" />
      {/if}
    </button>

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
