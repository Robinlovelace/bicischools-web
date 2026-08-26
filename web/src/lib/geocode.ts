export interface GeocodeResult {
  name: string;
  display_name: string;
  lng: number;
  lat: number;
  type?: string;
}

/**
 * Geocode an address, school, or place name using Photon (OpenStreetMap data) with fallback to Nominatim
 */
export async function geocodeLocation(query: string): Promise<GeocodeResult[]> {
  if (!query || query.trim().length < 2) return [];

  // Try Photon first (fast, generous rate limits, open-source OSM based)
  try {
    const url = `https://photon.komoot.io/api/?q=${encodeURIComponent(query.trim())}&limit=6`;
    const resp = await fetch(url);
    if (resp.ok) {
      const data = await resp.json();
      if (data && data.features && data.features.length > 0) {
        return data.features.map((f: any) => {
          const props = f.properties || {};
          const name = props.name || props.street || query;
          const parts = [
            props.name,
            props.street,
            props.city || props.town || props.village || props.district,
            props.state || props.county,
            props.country
          ].filter(Boolean);

          const display_name = Array.from(new Set(parts)).join(', ');
          const [lng, lat] = f.geometry.coordinates;

          return {
            name,
            display_name: display_name || name,
            lng,
            lat,
            type: props.osm_value || props.type
          };
        });
      }
    }
  } catch (err) {
    console.warn('Photon geocoding failed, trying Nominatim fallback:', err);
  }

  // Fallback to Nominatim
  try {
    const nomUrl = `https://nominatim.openstreetmap.org/search?q=${encodeURIComponent(query.trim())}&format=json&limit=5&addressdetails=1`;
    const resp = await fetch(nomUrl, {
      headers: {
        'Accept': 'application/json'
      }
    });
    if (resp.ok) {
      const data = await resp.json();
      return data.map((item: any) => ({
        name: item.name || item.display_name.split(',')[0],
        display_name: item.display_name,
        lng: parseFloat(item.lon),
        lat: parseFloat(item.lat),
        type: item.type
      }));
    }
  } catch (err) {
    console.error('Nominatim geocoding failed:', err);
  }

  return [];
}
