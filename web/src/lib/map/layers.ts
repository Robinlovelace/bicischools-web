import type { Map as MapLibreMap } from 'maplibre-gl';
import type { FeatureCollection } from 'geojson';

export const ROUTE_COLORS = [
  '#8b5cf6', // Route 1: Purple / Indigo
  '#06b6d4', // Route 2: Cyan
  '#f59e0b', // Route 3: Amber / Orange
  '#ec4899', // Route 4: Pink
  '#10b981', // Route 5: Emerald
];

export function emptyFeatureCollection(): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: []
  };
}

export function setupMapLayers(map: MapLibreMap): void {
  // 1. Add GeoJSON Sources
  if (!map.getSource('route-network')) {
    map.addSource('route-network', {
      type: 'geojson',
      data: emptyFeatureCollection()
    });
  }

  if (!map.getSource('actual-routes')) {
    map.addSource('actual-routes', {
      type: 'geojson',
      data: emptyFeatureCollection()
    });
  }

  if (!map.getSource('candidate-routes')) {
    map.addSource('candidate-routes', {
      type: 'geojson',
      data: emptyFeatureCollection()
    });
  }

  if (!map.getSource('matched-origins')) {
    map.addSource('matched-origins', {
      type: 'geojson',
      data: emptyFeatureCollection()
    });
  }

  if (!map.getSource('timetable-stops')) {
    map.addSource('timetable-stops', {
      type: 'geojson',
      data: emptyFeatureCollection()
    });
  }

  if (!map.getSource('school-point')) {
    map.addSource('school-point', {
      type: 'geojson',
      data: emptyFeatureCollection()
    });
  }

  // 2. Add Map Layers

  // Route network (background demand heatmap with Viridis color ramp and dynamic stroke width)
  if (!map.getLayer('route-network-casing')) {
    map.addLayer({
      id: 'route-network-casing',
      type: 'line',
      source: 'route-network',
      paint: {
        'line-color': '#0f172a',
        'line-width': [
          'interpolate',
          ['linear'],
          ['zoom'],
          12,
          ['interpolate', ['linear'], ['coalesce', ['get', 'bicycle_godutch'], 0], 0, 2.0, 5, 4.5, 15, 8.0],
          16,
          ['interpolate', ['linear'], ['coalesce', ['get', 'bicycle_godutch'], 0], 0, 3.5, 5, 7.0, 15, 12.0]
        ],
        'line-opacity': 0.25
      }
    });
  }

  if (!map.getLayer('route-network-line')) {
    map.addLayer({
      id: 'route-network-line',
      type: 'line',
      source: 'route-network',
      layout: {
        'line-cap': 'round',
        'line-join': 'round'
      },
      paint: {
        'line-color': [
          'interpolate',
          ['linear'],
          ['coalesce', ['get', 'bicycle_godutch'], 0],
          0, '#64748b',
          1, '#0284c7',
          3, '#0d9488',
          6, '#10b981',
          10, '#f59e0b',
          16, '#f43f5e'
        ],
        'line-width': [
          'interpolate',
          ['linear'],
          ['zoom'],
          12,
          ['interpolate', ['linear'], ['coalesce', ['get', 'bicycle_godutch'], 0], 0, 1.2, 3, 2.8, 10, 5.0],
          16,
          ['interpolate', ['linear'], ['coalesce', ['get', 'bicycle_godutch'], 0], 0, 2.2, 3, 4.5, 10, 8.5]
        ],
        'line-opacity': 0.85
      }
    });
  }

  // Actual CicloExpresso routes (comparison dashed line)
  if (!map.getLayer('actual-routes-line')) {
    map.addLayer({
      id: 'actual-routes-line',
      type: 'line',
      source: 'actual-routes',
      paint: {
        'line-color': '#e11d48',
        'line-width': 4.5,
        'line-dasharray': [3, 2],
        'line-opacity': 0.95
      }
    });
  }

  // Candidate Bike Bus Corridors: Casing + Glow + Main Line
  if (!map.getLayer('candidate-routes-glow')) {
    map.addLayer({
      id: 'candidate-routes-glow',
      type: 'line',
      source: 'candidate-routes',
      paint: {
        'line-color': [
          'match',
          ['get', 'rank'],
          1, ROUTE_COLORS[0],
          2, ROUTE_COLORS[1],
          3, ROUTE_COLORS[2],
          4, ROUTE_COLORS[3],
          5, ROUTE_COLORS[4],
          '#8b5cf6'
        ],
        'line-width': [
          'interpolate',
          ['linear'],
          ['zoom'],
          12, 12,
          16, 18
        ],
        'line-opacity': 0.35,
        'line-blur': 4
      }
    });
  }

  if (!map.getLayer('candidate-routes-casing')) {
    map.addLayer({
      id: 'candidate-routes-casing',
      type: 'line',
      source: 'candidate-routes',
      layout: {
        'line-cap': 'round',
        'line-join': 'round'
      },
      paint: {
        'line-color': '#0f172a',
        'line-width': [
          'interpolate',
          ['linear'],
          ['zoom'],
          12, 6.5,
          16, 10.5
        ],
        'line-opacity': 0.95
      }
    });
  }

  if (!map.getLayer('candidate-routes-line')) {
    map.addLayer({
      id: 'candidate-routes-line',
      type: 'line',
      source: 'candidate-routes',
      layout: {
        'line-cap': 'round',
        'line-join': 'round'
      },
      paint: {
        'line-color': [
          'match',
          ['get', 'rank'],
          1, ROUTE_COLORS[0],
          2, ROUTE_COLORS[1],
          3, ROUTE_COLORS[2],
          4, ROUTE_COLORS[3],
          5, ROUTE_COLORS[4],
          '#8b5cf6'
        ],
        'line-width': [
          'interpolate',
          ['linear'],
          ['zoom'],
          12, 4.0,
          16, 7.0
        ],
        'line-opacity': 1.0
      }
    });
  }

  // Matched Origin dots
  if (!map.getLayer('matched-origins-circle')) {
    map.addLayer({
      id: 'matched-origins-circle',
      type: 'circle',
      source: 'matched-origins',
      paint: {
        'circle-radius': [
          'interpolate',
          ['linear'],
          ['zoom'],
          12, ['interpolate', ['linear'], ['coalesce', ['get', 'num_students'], 1], 1, 3.5, 5, 6, 20, 10],
          16, ['interpolate', ['linear'], ['coalesce', ['get', 'num_students'], 1], 1, 5.5, 5, 10, 20, 18]
        ],
        'circle-color': [
          'case',
          ['has', 'assigned_route_rank'],
          [
            'match',
            ['get', 'assigned_route_rank'],
            1, ROUTE_COLORS[0],
            2, ROUTE_COLORS[1],
            3, ROUTE_COLORS[2],
            4, ROUTE_COLORS[3],
            5, ROUTE_COLORS[4],
            '#94a3b8'
          ],
          '#94a3b8'
        ],
        'circle-stroke-width': 1.5,
        'circle-stroke-color': '#ffffff',
        'circle-opacity': 0.85
      }
    });
  }

  // Timetable Stops: Outer border + Route-colored inner circle + Label
  if (!map.getLayer('timetable-stops-outer')) {
    map.addLayer({
      id: 'timetable-stops-outer',
      type: 'circle',
      source: 'timetable-stops',
      paint: {
        'circle-radius': 12,
        'circle-color': '#ffffff',
        'circle-stroke-width': 2.5,
        'circle-stroke-color': '#0f172a'
      }
    });
  }

  if (!map.getLayer('timetable-stops-inner')) {
    map.addLayer({
      id: 'timetable-stops-inner',
      type: 'circle',
      source: 'timetable-stops',
      paint: {
        'circle-radius': 9,
        'circle-color': [
          'match',
          ['get', 'route_rank'],
          1, ROUTE_COLORS[0],
          2, ROUTE_COLORS[1],
          3, ROUTE_COLORS[2],
          4, ROUTE_COLORS[3],
          5, ROUTE_COLORS[4],
          '#3b82f6'
        ]
      }
    });
  }

  if (!map.getLayer('timetable-stops-label')) {
    map.addLayer({
      id: 'timetable-stops-label',
      type: 'symbol',
      source: 'timetable-stops',
      layout: {
        'text-field': ['get', 'stop_label'],
        'text-size': 9,
        'text-allow-overlap': true
      },
      paint: {
        'text-color': '#ffffff'
      }
    });
  }

  // School Destination Marker
  if (!map.getLayer('school-marker-circle')) {
    map.addLayer({
      id: 'school-marker-circle',
      type: 'circle',
      source: 'school-point',
      paint: {
        'circle-radius': 14,
        'circle-color': '#fbbf24',
        'circle-stroke-width': 3.5,
        'circle-stroke-color': '#0f172a'
      }
    });
  }
}
