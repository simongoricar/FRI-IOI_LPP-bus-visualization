import "./styles/main.scss";
import { LPPBusAPI } from "./buses/api.ts";

const api = new LPPBusAPI();
let stations = await api.getAllStations({ cache: true });
console.log(stations);


import Leaflet from "leaflet";

const markerIcon = Leaflet.icon({
    iconUrl: "images/noun-bus-stop-pin-985103.svg",
    iconSize: [24, 24]
});

const leafletMap = Leaflet.map(
  "map",
  {
      attributionControl: true,
      zoomControl: false,
      center: Leaflet.latLng(46.057838, 14.509823),
      zoom: 13.5,
      maxZoom: 18,
      minZoom: 13,
      zoomSnap: 0.5,
      zoomDelta: 0.5,
      inertia: false,
      wheelPxPerZoomLevel: 90,
      maxBounds: Leaflet.latLngBounds(
        Leaflet.latLng(46.088995, 14.435850),
        Leaflet.latLng(46.009894, 14.588667)
      ),
  }
);
leafletMap.attributionControl.setPrefix(false);

const leafletMapTiles = Leaflet.tileLayer(
  "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
  {
      maxZoom: 18,
      attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
  }
);
leafletMapTiles.addTo(leafletMap);


for (const station of stations) {
    const stationLocation = station.latLng();

    const stationMarker = Leaflet.marker(stationLocation, {
        riseOnHover: false,
        icon: markerIcon,
    });

    const stationMarkerPopup = Leaflet.popup()
      .setContent(`${station.name}`);

    stationMarker.bindPopup(stationMarkerPopup);

    stationMarker.addTo(leafletMap);
}
