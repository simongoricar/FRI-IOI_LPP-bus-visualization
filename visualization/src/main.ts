import "./styles/main.scss";


import p5 from "p5";
import Logger, { Colour } from "./core/logger.ts";
import IOIMap from "./map";
import { ProjectError } from "./core/errors.ts";
import { loadRoutesSnapshot, loadStationsSnapshot } from "./lpp";
import { BusArrivalPlayback, RelativeMinutes, TimeOfDay } from "./lpp/replayer.ts";
import { LatLng, Point } from "leaflet";
import { AllRoutesSnapshot, AllStationsSnapshot, GeographicalLocation } from "./lpp/models.ts";
import { clamp } from "./utilities.ts";

const log = new Logger("main", Colour.LAUREL_GREEN);

class Droplet {
    public initializationTime: TimeOfDay;
    public location: LatLng;

    constructor(initializationTime: TimeOfDay, location: GeographicalLocation) {
        this.initializationTime = initializationTime;
        this.location = location.leafletLatLng();
    }

    public timeSinceInitialization(currentTime: TimeOfDay): RelativeMinutes {
        return currentTime.elapsedSince(this.initializationTime);
    }

    hasFinished(currentTime: TimeOfDay) {
        return this.timeSinceInitialization(currentTime).getTotalMinutes()
          >= dropletLifetimeInSimulatedMinutes;
    }
}

const ALL_STATION_SNAPSHOTS = [
  "station-details_2023-10-31_17-56-15.488+UTC.json",
  "station-details_2023-11-05_19-11-53.567+UTC.json"
];

const ALL_ROUTE_SNAPSHOTS = [
  "route-details_2023-10-31_17-56-15.488+UTC.json",
  "route-details_2023-11-05_19-11-53.567+UTC.json"
];

async function loadAllAvailableStationSnapshots(): Promise<AllStationsSnapshot[]> {
    let stationSnapshots: AllStationsSnapshot[] = [];

    let currentSnapshotIndex = 0;
    const totalSnapshots = ALL_STATION_SNAPSHOTS.length;

    for (const stationSnapshotFilename of ALL_STATION_SNAPSHOTS) {
        updateLoadingDetails(`postaje (${currentSnapshotIndex + 1}/${totalSnapshots})`);

        const snapshot = await loadStationsSnapshot(stationSnapshotFilename);
        stationSnapshots.push(snapshot);

        log.info(
          `Loaded stations snapshot "${stationSnapshotFilename}"`
        );

        currentSnapshotIndex += 1;
    }

    log.info(`Loaded all ${stationSnapshots.length} stations snapshots.`);

    return stationSnapshots;
}

async function loadAllAvailableRouteSnapshots(): Promise<AllRoutesSnapshot[]> {
    let routeSnapshots: AllRoutesSnapshot[] = [];

    let currentSnapshotIndex = 0;
    const totalSnapshots = ALL_ROUTE_SNAPSHOTS.length;

    for (const routeSnapshotFilename of ALL_ROUTE_SNAPSHOTS) {
        updateLoadingDetails(`avtobuse (${currentSnapshotIndex + 1}/${totalSnapshots})`);

        const snapshot = await loadRoutesSnapshot(routeSnapshotFilename);
        routeSnapshots.push(snapshot);

        log.info(
          `Loaded route snapshot "${routeSnapshotFilename}"`
        );

        currentSnapshotIndex += 1;
    }

    log.info(`Loaded all ${routeSnapshots.length} route snapshots.`);

    return routeSnapshots;
}

function selectStationSnapshot(snapshotIndex: number) {
    const snapshot = availableStationSnapshots[snapshotIndex];
    if (typeof snapshot === "undefined") {
        throw new Error("Invalid station snapshot index: out of bounds.");
    }

    // Modify visible date.
    const year = snapshot.capturedAt.getFullYear();
    const month = snapshot.capturedAt.getMonth() + 1;
    const day = snapshot.capturedAt.getDate();

    const dayOfWeek = snapshot.capturedAt.getDay();
    let dayOfWeekDescribed: string;
    switch (dayOfWeek) {
        case 0:
            dayOfWeekDescribed = "nedelja";
            break;
        case 1:
            dayOfWeekDescribed = "ponedeljek";
            break;
        case 2:
            dayOfWeekDescribed = "torek";
            break;
        case 3:
            dayOfWeekDescribed = "sreda";
            break;
        case 4:
            dayOfWeekDescribed = "četrtek";
            break;
        case 5:
            dayOfWeekDescribed = "petek";
            break;
        case 6:
            dayOfWeekDescribed = "sobota";
            break;
        default:
            log.error(`Invalid day of week: ${dayOfWeek}.`);
            dayOfWeekDescribed = "";
    }

    dateLabelElement.innerText =
      `${String(day).padStart(2, " ")}. ${String(month).padStart(2, " ")}. ${year} (${dayOfWeekDescribed})`;


    // Perform other initialization.
    selectedStationSnapshotIndex = snapshotIndex;
    selectedStationSnapshot = snapshot;

    playback = new BusArrivalPlayback(selectedStationSnapshot, initialSimulationTime);

    // Reset draw state
    activeDroplets = [];
}

function toggleSimulationPause() {
    if (!isSimulationPaused) {
        // Pause
        timeContainerElement.classList.add("paused");
        isSimulationPaused = true;
    } else {
        // Unpause
        timeContainerElement.classList.remove("paused");
        isSimulationPaused = false;
    }
}

function goToPreviousDay() {
    let targetSnapshotIndex = selectedStationSnapshotIndex - 1;
    if (targetSnapshotIndex < 0) {
        targetSnapshotIndex = availableStationSnapshots.length - 1;
    }

    log.info("Going to previous snapshot: " + targetSnapshotIndex);
    selectStationSnapshot(targetSnapshotIndex);
}

function goToNextDay() {
    let targetSnapshotIndex = selectedStationSnapshotIndex + 1;
    if (targetSnapshotIndex > (availableStationSnapshots.length - 1)) {
        targetSnapshotIndex = 0;
    }

    log.info("Going to next snapshot: " + targetSnapshotIndex);
    selectStationSnapshot(targetSnapshotIndex);
}

function resetDay() {
    log.info("Resetting day.");
    selectStationSnapshot(selectedStationSnapshotIndex);
}

function toggleFastForwardSimulation() {
    if (!isFastForwarding) {
        log.info("Fast-forwarding.");
        currentRealTimeSecondsPerSimulatedMinute = realTimeSecondsPerSimulatedMinute / 6;
        isFastForwarding = true;
    } else {
        log.info("Resetting simulation speed to normal.");
        currentRealTimeSecondsPerSimulatedMinute = realTimeSecondsPerSimulatedMinute;
        isFastForwarding = false;
    }
}

/*
 * SKETCH CONFIGURATION begin
 */

const simulationMinutesPerRealTimeSecond = 3;
const realTimeSecondsPerSimulatedMinute = 1 / simulationMinutesPerRealTimeSecond;

const initialSimulationTime = new TimeOfDay(4, 40);


const stationCircleColor = "#ee33ad";
const stationCircleRadius = 3;

const dropletLifetimeInSimulatedMinutes = 10;

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
// This can be modified when fast-forwarding or going backwards.
let currentRealTimeSecondsPerSimulatedMinute = realTimeSecondsPerSimulatedMinute;
let isFastForwarding = false;

let availableRouteSnapshots: AllRoutesSnapshot[];
let availableStationSnapshots: AllStationsSnapshot[];

let selectedStationSnapshotIndex: number;
let selectedStationSnapshot: AllStationsSnapshot;

let rootAppElement: HTMLElement;
let loadingHeadingElement: HTMLElement;
let loadingDetailsElement: HTMLElement;

let dateLabelElement: HTMLElement;
let previousDayButtonElement: HTMLButtonElement;
let nextDayButtonElement: HTMLButtonElement;

let timeLabelHourElement: HTMLElement;
let timeLabelMinuteElement: HTMLElement;

let resetTimeButtonElement: HTMLButtonElement;
let fastForwardTimeToggleElement: HTMLInputElement;

let timeContainerElement: HTMLElement;

let isSimulationPaused = false;

let lastDrawTime: Date;

let map: IOIMap;
let playback: BusArrivalPlayback;
let activeDroplets: Droplet[] = [];

let showStationsCheckboxElement: HTMLInputElement;
let showArrivalsCheckboxElement: HTMLInputElement;
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
    const freshArrivalSets = playback.tick(
      timeDeltaSinceLastDraw / currentRealTimeSecondsPerSimulatedMinute
    );

    const freshSimulatedTime = playback.currentDayTime;
    timeLabelHourElement.innerText = String(Math.floor(freshSimulatedTime.hour)).padStart(2, "0");
    timeLabelMinuteElement.innerText = String(Math.floor(freshSimulatedTime.minute)).padStart(2, "0");

    for (const arrivalSet of freshArrivalSets) {
        const arrivalTime = arrivalSet.time;

        for (const arrival of arrivalSet.arrivals) {
            const droplet = new Droplet(arrivalTime, arrival.location);
            activeDroplets.push(droplet);
        }
    }

    activeDroplets = activeDroplets.filter(
      droplet => !droplet.hasFinished(freshSimulatedTime)
    );
}

function getElementByIdOrThrow(elementId: string): HTMLElement {
    let element = document.getElementById(elementId);
    if (element === null) {
        throw new ProjectError(`Could not find element with id ${elementId}!`);
    }

    return element;
}

function updateLoadingHeading(
  content: string
) {
    loadingHeadingElement.innerText = content;
}

function updateLoadingDetails(
  details: string
) {
    loadingDetailsElement.innerText = details;
}

function hideLoadingScreen() {
    rootAppElement.classList.remove("not-ready");
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

    for (const station of selectedStationSnapshot.stationDetails) {
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
  currentSimulationTime: TimeOfDay,
  mapXOffset: number,
  mapYOffset: number,
  mapPixelOrigin: Point,
) {
    p.strokeWeight(0);

    for (let droplet of activeDroplets) {
        const transitionPercentage = clamp(
          droplet.timeSinceInitialization(currentSimulationTime).getTotalMinutes()
            / dropletLifetimeInSimulatedMinutes,
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
        dropletInitialColor = p.color("rgba(17,158,211,0.95)");
        // @ts-ignore
        dropletFinalColor = p.color("rgba(210,230,243,0)");

        /*
         * SETUP-TIME CONFIGURATION end
         */

        dateLabelElement = getElementByIdOrThrow("show-date-span");
        timeLabelHourElement = getElementByIdOrThrow("show-time-hour-span");
        timeLabelMinuteElement = getElementByIdOrThrow("show-time-minute-span");

        showStationsCheckboxElement =
          getElementByIdOrThrow("option-show-stations-input") as HTMLInputElement;
        showArrivalsCheckboxElement =
          getElementByIdOrThrow("option-show-arrivals-input") as HTMLInputElement;

        lastDrawTime = new Date();

        updateLoadingDetails("zemljevid");
        const mapElement = getElementByIdOrThrow("map");
        map = new IOIMap(mapElement);

        updateLoadingHeading("Pripravljam");
        updateLoadingDetails("čas prihodov");
        selectStationSnapshot(0);

        const width = map.canvas.clientWidth;
        const height = map.canvas.clientHeight;

        p.createCanvas(width, height, map.canvas);
        p.frameRate(24);

        p.colorMode("rgb");
        p.smooth();

        hideLoadingScreen();
    }

    p.draw = () => {
        // @ts-ignore
        p.clear();

        const currentTime = new Date();
        // @ts-ignore
        const drawTimeDelta = (currentTime - lastDrawTime) / 1000;

        // Parse user options.
        const isShowStationsChecked = showStationsCheckboxElement.checked;
        const isShowArrivalsChecked = showArrivalsCheckboxElement.checked;

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

        // If the simulation is paused, we can still draw the droplets,
        // but we must not update (tick) them.
        if (!isSimulationPaused) {
            updateDroplets(drawTimeDelta);
        }

        if (isShowArrivalsChecked) {
            drawDroplets(
              p,
              playback.currentDayTime,
              mapXOffset,
              mapYOffset,
              pixelOrigin
            );
        }

        lastDrawTime = currentTime;
    }
};

document.addEventListener("DOMContentLoaded", async function () {
    rootAppElement = getElementByIdOrThrow("app");

    loadingHeadingElement = getElementByIdOrThrow("loading-heading-span");
    loadingDetailsElement = getElementByIdOrThrow("loading-details-span");

    previousDayButtonElement =
      getElementByIdOrThrow("previous-day-button") as HTMLButtonElement;
    nextDayButtonElement =
      getElementByIdOrThrow("next-day-button") as HTMLButtonElement;
    timeContainerElement = getElementByIdOrThrow("time-container");

    resetTimeButtonElement =
      getElementByIdOrThrow("reset-time-toggle") as HTMLButtonElement;
    fastForwardTimeToggleElement =
      getElementByIdOrThrow("fast-forward-time-toggle") as HTMLInputElement;

    previousDayButtonElement.addEventListener("click", goToPreviousDay);
    nextDayButtonElement.addEventListener("click", goToNextDay);
    timeContainerElement.addEventListener("click", toggleSimulationPause);

    resetTimeButtonElement.addEventListener("click", resetDay);
    fastForwardTimeToggleElement.addEventListener("click", toggleFastForwardSimulation);

    updateLoadingDetails("postaje");
    availableStationSnapshots = await loadAllAvailableStationSnapshots();

    updateLoadingDetails("avtobuse");
    availableRouteSnapshots = await loadAllAvailableRouteSnapshots();

    updateLoadingDetails("vizualizacijo");
    // noinspection JSPotentiallyInvalidConstructorUsage
    new p5(p5Sketch, rootAppElement);
})

