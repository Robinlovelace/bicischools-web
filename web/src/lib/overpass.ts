import type { OriginInput, SchoolInfo } from '../types';

export const OVERPASS_ENDPOINT = 'https://overpass-api.de/api/interpreter';

/**
 * Fetch routable OSM road and cycle network for a given bounding box
 */
export async function fetchOsmNetworkForBbox(
  minLng: number,
  minLat: number,
  maxLng: number,
  maxLat: number
): Promise<string> {
  const query = `[out:json][timeout:35];
(
  way["highway"]["highway"!~"motorway|motorway_link|proposed|construction|corridor|steps"](${minLat},${minLng},${maxLat},${maxLng});
  way["cycleway"](${minLat},${minLng},${maxLat},${maxLng});
);
out body;
>;
out skel qt;`;

  const response = await fetch(OVERPASS_ENDPOINT, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded'
    },
    body: `data=${encodeURIComponent(query)}`
  });

  if (!response.ok) {
    throw new Error(`Overpass API query failed with status HTTP ${response.status}`);
  }

  return await response.text();
}

/**
 * Fetch OSM network around a point within a radius (in meters)
 */
export async function fetchOsmNetworkAroundPoint(
  lng: number,
  lat: number,
  radiusMeters: number = 2500
): Promise<string> {
  const query = `[out:json][timeout:35];
(
  way["highway"]["highway"!~"motorway|motorway_link|proposed|construction|corridor|steps"](around:${radiusMeters},${lat},${lng});
  way["cycleway"](around:${radiusMeters},${lat},${lng});
);
out body;
>;
out skel qt;`;

  const response = await fetch(OVERPASS_ENDPOINT, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded'
    },
    body: `data=${encodeURIComponent(query)}`
  });

  if (!response.ok) {
    throw new Error(`Overpass API query failed with status HTTP ${response.status}`);
  }

  return await response.text();
}

/**
 * Search for schools near coordinates
 */
export async function searchNearbySchools(
  lng: number,
  lat: number,
  radiusMeters: number = 3000
): Promise<SchoolInfo[]> {
  const query = `[out:json][timeout:20];
(
  nwr["amenity"="school"](around:${radiusMeters},${lat},${lng});
);
out center 25;`;

  const response = await fetch(OVERPASS_ENDPOINT, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded'
    },
    body: `data=${encodeURIComponent(query)}`
  });

  if (!response.ok) {
    return [];
  }

  const json = await response.json();
  const schools: SchoolInfo[] = [];

  for (const el of json.elements || []) {
    const name = el.tags?.name || el.tags?.['name:en'] || el.tags?.['official_name'];
    if (!name) continue;

    let elLng = el.lon;
    let elLat = el.lat;

    if (el.center) {
      elLng = el.center.lon;
      elLat = el.center.lat;
    }

    if (elLng && elLat) {
      schools.push({
        name,
        dgeec_id: el.tags?.ref || el.id,
        lng: elLng,
        lat: elLat
      });
    }
  }

  return schools;
}
