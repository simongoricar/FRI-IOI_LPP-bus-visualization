import "./styles/main.scss";


import p5 from "p5";
import Logger, { Colour } from "./core/logger.ts";
import IOIMap from "./map";
import { ProjectError } from "./core/errors.ts";
import { loadRoutesSnapshot, loadStationsSnapshot } from "./lpp";
import { BusArrivalPlayback, RelativeMinutes, TimeOfDay } from "./lpp/replayer.ts";
import { LatLng, Point } from "leaflet";
import {
    AllRoutesSnapshot,
    AllStationsSnapshot,
    GeographicalLocation
} from "./lpp/models.ts";
import { clamp } from "./utilities.ts";
import Data from "./data.ts";
import { StationSearcher } from "./lpp/stationSearcher.ts";


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

class StationPopup {
    public popupAnchorLocation: LatLng;
    public stationName: string;
    public stationCode: string;

    constructor(
      popupAnchorLocation: LatLng,
      stationName: string,
      stationCode: string,
    ) {
        this.popupAnchorLocation = popupAnchorLocation;
        this.stationName = stationName;
        this.stationCode = stationCode;
    }
}

async function loadAllAvailableStationSnapshots(): Promise<AllStationsSnapshot[]> {
    let stationSnapshots: AllStationsSnapshot[] = [];

    let currentSnapshotIndex = 0;
    const totalSnapshots = Data.allStationSnapshots.length;

    for (const stationSnapshotFilename of Data.allStationSnapshots) {
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
    const totalSnapshots = Data.allRouteSnapshots.length;

    for (const routeSnapshotFilename of Data.allRouteSnapshots) {
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

    playback = new BusArrivalPlayback(selectedStationSnapshot, initialSimulationTime.clone());
    stationSearcher = new StationSearcher(selectedStationSnapshot);

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
        currentRealTimeSecondsPerSimulatedMinute = fastForwardedRealTimeSecondsPerSimulatedMinute;
        isFastForwarding = true;
        fastForwardTimeToggleElement.checked = true;
    } else {
        log.info("Resetting simulation speed to normal.");
        currentRealTimeSecondsPerSimulatedMinute = realTimeSecondsPerSimulatedMinute;
        isFastForwarding = false;
        fastForwardTimeToggleElement.checked = false;
    }
}

function handleCanvasClick(event: MouseEvent) {
    if (!showStationsCheckboxElement.checked) {
        return;
    }

    const { top: mapXOffset, left: mapYOffset } = map.map.getContainer().getBoundingClientRect();
    const pixelOrigin = map.map.getPixelOrigin();

    const mouseClickPoint = new Point(event.offsetX, event.offsetY);
    const latLngOfClick = canvasPixelToLocation(
      mouseClickPoint,
      mapXOffset,
      mapYOffset,
      pixelOrigin,
    );

    log.debug(`Mouse clicked at ${event.offsetX}, ${event.offsetY}, which is at ${latLngOfClick.lat}, ${latLngOfClick.lng}`);

    const closestStationData = stationSearcher.getClosestStation(latLngOfClick);

    if (closestStationData === null) {
        log.debug("User did not click near any station!?");
        currentStationPopup = null;
        return;
    }

    const [_, closestStation] = closestStationData;

    // Project the station location back to pixels, so we can measure how far off
    // the user clicked.
    const closestStationPixelLocation = locationToCanvasPixel(
      new LatLng(closestStation.location.latitude, closestStation.location.longitude),
      mapXOffset,
      mapYOffset,
      pixelOrigin,
    );

    const pixelClickDistance = closestStationPixelLocation.distanceTo(mouseClickPoint);
    if (pixelClickDistance > stationClickDistanceToleranceInPixels) {
        log.debug("User did click, but was too far off.");
        currentStationPopup = null;
        return;
    }

    log.info(
      `User clicked near a station, will display popup: ${closestStation.name} (${closestStation.stationCode}).`
    );

    currentStationPopup = new StationPopup(
      closestStation.location.leafletLatLng(),
      closestStation.name,
      closestStation.stationCode,
    )
}

function handleKeyboardInput(event: KeyboardEvent) {
    if (event.key === "c") {
        log.info("User pressed 'c', closing station popup.");
        currentStationPopup = null;
    } else if (event.key === "p" || event.key === " ") {
        log.info("User pressed 'p'/space, pausing/un-pausing simulation.");
        toggleSimulationPause();
    } else if (event.key === "f") {
        log.info("User pressed 'f', fast-forwarding/resetting simulation speed.");
        toggleFastForwardSimulation();
    } else if (event.key === "s") {
        log.info("User pressed 's', toggling 'show stations' option.");
        showStationsCheckboxElement.checked = !showStationsCheckboxElement.checked;

        if (!showStationsCheckboxElement.checked) {
            currentStationPopup = null;
        }
    } else if (event.key === "a") {
        log.info("User pressed 'a', toggling 'show arrivals' option.");
        showArrivalsCheckboxElement.checked = !showArrivalsCheckboxElement.checked;
    } else if (event.key === "r") {
        log.info("User pressed 'r', resetting day.");
        resetDay();
    } else if (event.key === "n") {
        log.info("User pressed 'n', loading next day.");
        goToNextDay();
    } else if (event.key === "b") {
        log.info("User pressed 'b', loading previous day.");
        goToPreviousDay();
    }
}


/*
 * SKETCH CONFIGURATION begin
 */
const log = new Logger("main", Colour.LAUREL_GREEN);

const simulationMinutesPerRealTimeSecond = 2.7;
const fastForwardedSimulationMinutesPerRealTimeSecond = simulationMinutesPerRealTimeSecond * 8;

const initialSimulationTime = new TimeOfDay(3, 30);

const stationClickDistanceToleranceInPixels = 24;

const stationCircleColor = "#ee33ad";
const stationCircleRadius = 5;

const dropletLifetimeInSimulatedMinutes = 5.8;

let dropletInitialColor: p5.Color;
const dropletInitialRadius = 2;

let dropletFinalColor: p5.Color;
const dropletFinalRadius = 28;

const stationPopupFontSize = 17;

const stationPopupXOffset = 0;
const stationPopupYOffset = -34;

const stationPopupTextPadding = 10;
const stationPopupTextOnlyYOffset = 2;

const stationPopupRectRoundedBorders = 5;
const stationPopupTriangleCenterOffset = 18;
const stationPopupTriangleYOffset = -4;

let stationPopupTextColor: p5.Color;
let stationPopupBackgroundColor: p5.Color;
/*
 * SKETCH CONFIGURATION end
 */


/*
 * SKETCH STATE begin
 */
const realTimeSecondsPerSimulatedMinute = 1 / simulationMinutesPerRealTimeSecond;
const fastForwardedRealTimeSecondsPerSimulatedMinute = 1 / fastForwardedSimulationMinutesPerRealTimeSecond;

// This is modified when fast-forwarding.
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

let font: p5.Font;

let map: IOIMap;
let playback: BusArrivalPlayback;
let stationSearcher: StationSearcher;

let activeDroplets: Droplet[] = [];
let currentStationPopup: StationPopup | null = null;

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
): Point {
    let pixelLocation = map.map.project(
      location,
      map.map.getZoom()
    )
      .subtract(mapPixelOrigin);

    pixelLocation.x += mapXOffset;
    pixelLocation.y += mapYOffset;

    return pixelLocation;
}

function canvasPixelToLocation(
  pixel: Point,
  mapXOffset: number,
  mapYOffset: number,
  mapPixelOrigin: Point,
): LatLng {
    let pixelCloned = pixel.clone();
    pixelCloned.x += mapXOffset;
    pixelCloned.y += mapYOffset;

    return map.map.unproject(
      pixel.add(mapPixelOrigin),
      map.map.getZoom(),
    );
}

function updateDroplets(
  timeDeltaSinceLastDraw: number,
) {
    const freshArrivalSets = playback.tick(
      timeDeltaSinceLastDraw / currentRealTimeSecondsPerSimulatedMinute
    );

    for (const arrivalSet of freshArrivalSets) {
        const arrivalTime = arrivalSet.time;

        for (const arrival of arrivalSet.arrivals) {
            const droplet = new Droplet(arrivalTime, arrival.location);
            activeDroplets.push(droplet);
        }
    }

    activeDroplets = activeDroplets.filter(
      droplet => !droplet.hasFinished(playback.getTimeOfDay())
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
    p.strokeWeight(0);
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

function drawStationPopup(
  p: p5,
  mapXOffset: number,
  mapYOffset: number,
  mapPixelOrigin: Point,
) {
    if (currentStationPopup === null) {
        return;
    }

    const popupOrigin = locationToCanvasPixel(
      currentStationPopup.popupAnchorLocation,
      mapXOffset,
      mapYOffset,
      mapPixelOrigin
    );

    const popupText = `${currentStationPopup.stationName} (${currentStationPopup.stationCode})`;
    const popupTextXPosition = popupOrigin.x + stationPopupXOffset;
    const popupTextYPosition = popupOrigin.y + stationPopupYOffset;

    p.textAlign("center", "center");
    p.textSize(stationPopupFontSize);
    p.textFont(font);

    const textBoundingBox = font.textBounds(
      popupText,
      popupTextXPosition,
      popupTextYPosition + stationPopupTextOnlyYOffset
    );
    // @ts-ignore
    const { w: textWidth, h: textHeight } = textBoundingBox;


    p.strokeWeight(0);
    p.fill(stationPopupBackgroundColor);

    const triangleTopXCenter = popupTextXPosition;
    const triangleTopY = popupTextYPosition + stationPopupTextPadding;
    const triangleBottomTipX = popupOrigin.x;
    const triangleBottomTipY = popupOrigin.y + stationPopupTriangleYOffset;

    p.triangle(
      triangleTopXCenter - stationPopupTriangleCenterOffset,
      triangleTopY,
      triangleTopXCenter + stationPopupTriangleCenterOffset,
      triangleTopY,
      triangleBottomTipX,
      triangleBottomTipY,
    )

    p.rectMode("center");
    p.rect(
      popupTextXPosition,
      popupTextYPosition + stationPopupTextOnlyYOffset,
      textWidth + 2 * stationPopupTextPadding,
      textHeight + 2 * stationPopupTextPadding,
      stationPopupRectRoundedBorders
    );


    p.fill(stationPopupTextColor);
    p.text(
      popupText,
      popupTextXPosition,
      popupTextYPosition
    );
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
        dropletInitialColor = p.color("rgba(17, 158, 211, 0.95)");
        // @ts-ignore
        dropletFinalColor = p.color("rgba(210, 230, 243, 0)");

        // @ts-ignore
        stationPopupTextColor = p.color("rgb(31,31,31)");
        // @ts-ignore
        stationPopupBackgroundColor = p.color("rgb(255, 255, 255)");

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

        font = p.loadFont("./fonts/Roboto/Roboto-Regular.ttf");

        p.createCanvas(width, height, map.canvas);
        p.frameRate(24);

        p.colorMode("rgb");
        p.smooth();

        map.canvas.addEventListener("click", handleCanvasClick);

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

        const freshSimulatedTime = playback.getTimeOfDay();
        timeLabelHourElement.innerText = String(Math.floor(freshSimulatedTime.hour)).padStart(2, "0");
        timeLabelMinuteElement.innerText = String(Math.floor(freshSimulatedTime.minute)).padStart(2, "0");

        if (isShowArrivalsChecked) {
            drawDroplets(
              p,
              playback.getTimeOfDay(),
              mapXOffset,
              mapYOffset,
              pixelOrigin
            );
        }

        if (isShowStationsChecked) {
            drawStationPopup(
              p,
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

    document.body.addEventListener("keydown", handleKeyboardInput);

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
});
