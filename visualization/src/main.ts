import "./styles/main.scss";


import p5 from "p5";
import Logger, { Colour } from "./core/logger.ts";
import IOIMap from "./map";
import { ProjectError } from "./core/errors.ts";
import { loadRoutesSnapshot, loadStationsSnapshot } from "./lpp";
import { BusArrivalPlayback } from "./lpp/replayer.ts";
import { LatLng, Point } from "leaflet";
import { GeographicalLocation } from "./lpp/models.ts";
import { clamp } from "./utilities.ts";

const log = new Logger("main", Colour.LAUREL_GREEN);


const stations = await loadStationsSnapshot("station-details_2023-10-31_17-56-15.488+UTC.json");
log.info(`Loaded stations snapshot! Got ${stations.stationDetails.length} stations.`);

const routes = await loadRoutesSnapshot("route-details_2023-10-31_17-56-15.488+UTC.json");
log.info(`Loaded routes snapshot! Got ${routes.routes.length} routes.`);


class Droplet {
    public timeSinceStart: number;
    public location: LatLng;

    constructor(location: GeographicalLocation) {
        this.timeSinceStart = 0;
        this.location = location.leafletLatLng();
    }

    tickTime(deltaTime: number) {
        this.timeSinceStart += deltaTime;
    }

    hasFinished() {
        return this.timeSinceStart >= dropletFadeOutTimeInSeconds;
    }
}

/*
 * SKETCH CONFIGURATION begin
 */

const simulationMinutesPerRealTimeSecond = 0.5;


const stationCircleColor = "#ee33ad";
const stationCircleRadius = 6;

const dropletFadeOutTimeInSeconds = 10;

let dropletInitialColor: p5.Color;
const dropletInitialRadius = 2;

let dropletFinalColor: p5.Color;
const dropletFinalRadius = 44;
/*
 * SKETCH CONFIGURATION end
 */

/*
 * SKETCH STATE begin
 */
const realTimeSecondsPerSimulatedMinute = 1 / simulationMinutesPerRealTimeSecond;
let timeSinceLastSimulatedMinute = 0;

let timeLabelHourElement: HTMLElement;
let timeLabelMinuteElement: HTMLElement;

let lastDrawTime: Date;

let map: IOIMap;
let playback: BusArrivalPlayback;
let activeDroplets: Droplet[] = [];

let showStationsOptionCheckbox: HTMLInputElement;
let showArrivalsOptionCheckbox: HTMLInputElement;
/*
 * SKETCH STATE end
 */

/*
 * SKETCH DRAW UTILITIES begin
 */

function locationToCanvasPixel(
  location: LatLng,
  mapXOffset: number,
  mapYOffset: number,
  mapPixelOrigin: Point,
) {
    let pixelLocation = map.map.project(
      location,
      map.map.getZoom()
    )
      .subtract(mapPixelOrigin);

    pixelLocation.x += mapXOffset;
    pixelLocation.y += mapYOffset;

    return pixelLocation;
}

function updateDroplets(
  timeDeltaSinceLastDraw: number,
) {
    timeSinceLastSimulatedMinute += timeDeltaSinceLastDraw;

    // Remove any finished droplets.
    for (const droplet of activeDroplets) {
        droplet.tickTime(timeDeltaSinceLastDraw);
    }

    activeDroplets = activeDroplets.filter(
      droplet => !droplet.hasFinished()
    );

    // We might need to do multiple steps if the rendering got stuck somewhere
    // or if the framerate is not fast enough.
    while (timeSinceLastSimulatedMinute >= realTimeSecondsPerSimulatedMinute) {
        timeSinceLastSimulatedMinute -= realTimeSecondsPerSimulatedMinute;

        const arrivalSet = playback.tickOneMinuteForward();
        if (arrivalSet === null) {
            continue;
        }

        timeLabelHourElement.innerText = String(arrivalSet.hour);
        timeLabelMinuteElement.innerText = String(arrivalSet.minute);

        for (const arrival of arrivalSet.arrivals) {
            let droplet = new Droplet(arrival.location);
            activeDroplets.push(droplet);
        }
    }
}

/*
 * SKETCH DRAW UTILITIES end
 */


/*
 * SKETCH DRAW FUNCTIONS begin
 */

function drawStations(
  p: p5,
  mapXOffset: number,
  mapYOffset: number,
  mapPixelOrigin: Point,
) {
    p.fill(stationCircleColor);

    for (const station of stations.stationDetails) {
        const stationPixelPosition = locationToCanvasPixel(
          station.location.leafletLatLng(),
          mapXOffset,
          mapYOffset,
          mapPixelOrigin
        );

        // log.debug(
        //   `Station ${station.name} has lat-lng of ${stationLatLng.toString()} and pixel position ${stationPixelPosition.toString()}`
        // );

        p.circle(
          stationPixelPosition.x,
          stationPixelPosition.y,
          stationCircleRadius,
        );
    }
}

function drawDroplets(
  p: p5,
  mapXOffset: number,
  mapYOffset: number,
  mapPixelOrigin: Point,
) {
    p.strokeWeight(0);

    for (let droplet of activeDroplets) {
        const transitionPercentage = clamp(
          droplet.timeSinceStart / dropletFadeOutTimeInSeconds,
          0,
          1,
        );

        const sizeInPixels = p.lerp(dropletInitialRadius, dropletFinalRadius, transitionPercentage);
        const dropletColor = p.lerpColor(dropletInitialColor, dropletFinalColor, transitionPercentage)

        const stationPixelPosition = locationToCanvasPixel(
          droplet.location,
          mapXOffset,
          mapYOffset,
          mapPixelOrigin
        );

        p.fill(dropletColor);
        p.circle(
          stationPixelPosition.x,
          stationPixelPosition.y,
          sizeInPixels,
        );
    }
}

/*
 * SKETCH DRAW FUNCTIONS end
 */


const p5Sketch = (p: p5) => {
    p.setup = () => {
        log.info("Initializing p5.js sketch.");

        /*
         * SETUP-TIME CONFIGURATION begin
         */

        // @ts-ignore
        dropletInitialColor = p.color("rgba(60,153,240,1)");
        // @ts-ignore
        dropletFinalColor = p.color("rgba(159,194,224,0)");

        /*
         * SETUP-TIME CONFIGURATION end
         */


        const mapElement = document.getElementById("map");
        if (mapElement === null) {
            throw new ProjectError("Missing map element!");
        }

        showStationsOptionCheckbox =
          document.getElementById("option-show-stations-input") as HTMLInputElement;
        showArrivalsOptionCheckbox =
          document.getElementById("option-show-arrivals-input") as HTMLInputElement;

        lastDrawTime = new Date();


        let showTimeHour = document.getElementById("show-time-hour-span");
        if (showTimeHour === null) {
            throw new ProjectError("Missing hour span.");
        }
        timeLabelHourElement = showTimeHour;

        let showTimeMinute = document.getElementById("show-time-minute-span");
        if (showTimeMinute === null) {
            throw new ProjectError("Missing minute span.");
        }
        timeLabelMinuteElement = showTimeMinute;


        map = new IOIMap(mapElement);
        playback = new BusArrivalPlayback(stations);


        const width = map.canvas.clientWidth;
        const height = map.canvas.clientHeight;


        p.createCanvas(width, height, map.canvas);
        p.frameRate(24);

        p.colorMode("rgb");
        p.smooth();
    }

    p.draw = () => {
        // @ts-ignore
        p.clear();

        const currentTime = new Date();
        // @ts-ignore
        const drawTimeDelta = (currentTime - lastDrawTime) / 1000;

        // Parse user options.
        const isShowStationsChecked = showStationsOptionCheckbox.checked;
        const isShowArrivalsChecked = showArrivalsOptionCheckbox.checked;

        // Parse map-related stuff.
        const { top: mapXOffset, left: mapYOffset } = map.map.getContainer().getBoundingClientRect();
        const pixelOrigin = map.map.getPixelOrigin();

        // Draw any things we need to draw.
        if (isShowStationsChecked) {
            drawStations(
              p,
              mapXOffset,
              mapYOffset,
              pixelOrigin
            );
        }

        updateDroplets(drawTimeDelta);

        if (isShowArrivalsChecked) {
            drawDroplets(
              p,
              mapXOffset,
              mapYOffset,
              pixelOrigin
            );
        }

        lastDrawTime = currentTime;
    }
};

const appElement = document.getElementById("app");
if (appElement === null) {
    log.error("#app element is missing from the page?!");
    throw new Error("Invalid DOM.");
}

// noinspection JSPotentiallyInvalidConstructorUsage
new p5(p5Sketch, appElement);
