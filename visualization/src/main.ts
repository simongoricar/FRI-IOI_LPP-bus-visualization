// noinspection JSPotentiallyInvalidConstructorUsage

import "./styles/main.scss";


// import Leaflet from "leaflet";
import p5 from "p5";
import Logger, { Colour } from "./core/logger.ts";
import IOIMap from "./map";
import { ProjectError } from "./core/errors.ts";
import { loadRoutesSnapshot, loadStationsSnapshot } from "./lpp";
import { BusArrivalReplayer } from "./lpp/replayer.ts";
import * as repl from "repl";
import { Point } from "leaflet";

const log = new Logger("main", Colour.LAUREL_GREEN);

const stations = await loadStationsSnapshot("station-details_2023-10-31_17-56-15.488+UTC.json");
log.info(`Loaded stations snapshot! Got ${stations.stationDetails.length} stations.`);

const routes = await loadRoutesSnapshot("route-details_2023-10-31_17-56-15.488+UTC.json");
log.info(`Loaded routes snapshot! Got ${routes.routes.length} routes.`);



class Translate3D {
    public x: number;
    public y: number;
    public z: number;

    public constructor(x: number, y: number, z: number) {
        this.x = x;
        this.y = y;
        this.z = z;
    }
}

function parseTransformTranslate3DOnElement(element: HTMLElement): Translate3D | null {
    const rawTransformStyle = element.style.transform;

    const matches = rawTransformStyle.match(/translate3d\((-?\d+)px, (-?\d+)px, (-?\d+)px\)/);
    if (matches === null) {
        return null;
    }

    return new Translate3D(Number(matches[1]), Number(matches[2]), Number(matches[3]));
}

// SKETCH STATE begin
let map: IOIMap;
let replayer: BusArrivalReplayer;

let showStationsOptionCheckbox: HTMLInputElement;
// SKETCH STATE end

function drawStations(
  p: p5,
  canvasTransform: Translate3D,
  mapLeftOffset: number,
  mapTopOffset: number,
  mapPixelOrigin: Point,
) {
    p.stroke("#F0A067");
    p.strokeWeight(0);
    p.fill("#F01067");

    for (const station of stations.stationDetails) {
        // TODO Fix p.circle not drawing at the correct point.

        // DEBUGONLY
        // if (station.stationCode !== "100021") {
        //     continue;
        // }

        const stationLatLng = station.location.leafletLatLng();
        const stationPixelPosition = map.map.project(
          stationLatLng,
          map.map.getZoom()
        )
          .subtract(mapPixelOrigin);

        // log.debug(
        //   `Station ${station.name} has lat-lng of ${stationLatLng.toString()} and pixel position ${stationPixelPosition.toString()}`
        // );

        p.circle(
          stationPixelPosition.x + mapLeftOffset - canvasTransform.x,
          stationPixelPosition.y + mapTopOffset - canvasTransform.y,
          4,
        );

        // p.circle(stationPixelCenter.x + mapLeftOffset, stationPixelCenter.y + mapTopOffset, 10);
        // console.log(`${station.name} -> ${station.latLng().toString()} -> ${stationPixelCenter.toString()}`);
    }
}

const p5SketchInitialization = (p: p5) => {
    p.setup = () => {
        log.info("P5 sketch: initializing");

        const mapElement = document.getElementById("map");
        if (mapElement === null) {
            throw new ProjectError("Missing map element!");
        }

        showStationsOptionCheckbox =
          document.getElementById("option-show-stations-input") as HTMLInputElement;

        map = new IOIMap(mapElement);
        replayer = new BusArrivalReplayer(stations);

        // markerIcon = Leaflet.icon({
        //     iconUrl: "images/noun-bus-stop-pin-985103.svg",
        //     iconSize: [24, 24]
        // });

        const width = map.canvas.clientWidth;
        const height = map.canvas.clientHeight;

        p.createCanvas(width, height, map.canvas);
        p.frameRate(6);

        p.colorMode("rgb");
        p.smooth();
    }

    p.draw = () => {
        p.clear();
        
        const { top: mapTopOffset, left: mapLeftOffset } = map.map.getContainer().getBoundingClientRect();
        const pixelOrigin = map.map.getPixelOrigin();
        
        let canvasTransform = parseTransformTranslate3DOnElement(map.canvas);
        if (canvasTransform === null) {
            log.warn(
              "Failed to get 3D translation on canvas, skipping render."
            );
            return;
        }

        const isShowStationsChecked = showStationsOptionCheckbox.checked;

        if (isShowStationsChecked) {
            drawStations(
              p,
              canvasTransform,
              mapLeftOffset,
              mapTopOffset,
              pixelOrigin
            );
        }
    }
};

const appElement = document.getElementById("app");
if (appElement === null) {
    log.error("#app element is missing from the page?!");
    throw new Error("Invalid DOM.");
}

new p5(p5SketchInitialization, appElement);
