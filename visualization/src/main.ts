import "./styles/main.scss";
import { LPPBusAPI } from "./buses/api.ts";


import Leaflet from "leaflet";
import P5 from "p5";
import p5 from "p5";
import Logger, { Colour } from "./core/logger.ts";
import IOIMap from "./map";
import { ProjectError } from "./core/errors.ts";

const log = new Logger("main", Colour.LAUREL_GREEN);
const api = new LPPBusAPI();

let stations = await api.getAllStations({ cache: true });
log.info("Got all LPP bus stations.")
log.debug(stations);

let map: IOIMap;


const p5SketchInitialization = (p: p5) => {
    p.setup = () => {
        log.info("P5 sketch: initializing");

        const mapElement = document.getElementById("map");
        if (mapElement === null) {
            throw new ProjectError("Missing map element!");
        }

        map = new IOIMap(mapElement);

        // markerIcon = Leaflet.icon({
        //     iconUrl: "images/noun-bus-stop-pin-985103.svg",
        //     iconSize: [24, 24]
        // });

        const width = map.canvas.clientWidth;
        const height = map.canvas.clientHeight;

        p.createCanvas(width, height, map.canvas);
        p.frameRate(4);
    }

    p.draw = () => {
        log.debug("P5 sketch: draw");

        const { top: mapTopOffset, left: mapLeftOffset } = map.map.getContainer().getBoundingClientRect();


        for (const station of stations) {
            const stationPixelCenter = map.map.latLngToContainerPoint(station.latLng());

            p.circle(stationPixelCenter.x + mapLeftOffset, stationPixelCenter.y + mapTopOffset, 10);

            // console.log(`${station.name} -> ${station.latLng().toString()} -> ${stationPixelCenter.toString()}`);
        }

        // for (const station of stations) {
        //     const stationLocation = station.latLng();
        //     const stationPixelCenter = leafletMap.project(stationLocation, leafletMap.getZoom());
        //
        //     console.log(`${station.name} -> ${stationLocation.toString()} -> ${stationPixelCenter.toString()}`);
            // p.circle(stationPixelCenter.x, stationPixelCenter.y, 10);

            // const stationMarker = Leaflet.marker(stationLocation, {
            //     riseOnHover: false,
            //     icon: markerIcon,
            // });
            //
            // const stationMarkerPopup = Leaflet.popup({
            //     className: "station-popup",
            //     closeButton: false,
            // })
            //   .setContent(`${station.name}`);
            //
            // stationMarker.bindPopup(stationMarkerPopup);
            // stationMarker.addTo(leafletMap);
        // }

        // throw new Error();
    }
};

const appElement = document.getElementById("app");
if (appElement === null) {
    log.error("#app element is missing from the page?!");
    throw new Error("Invalid DOM.");
}

new P5(p5SketchInitialization, appElement)
