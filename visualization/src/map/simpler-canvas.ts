import Leaflet from "leaflet";
import Logger, { Colour } from "../core/logger.ts";

const log = new Logger("simpler-canvas", Colour.FRENCH_BLUE);

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

export const CanvasV2 = Leaflet.Layer.extend({
    onAdd(map: Leaflet.Map) {
        const pane = map.getPane("overlayPane");
        //const pane = map.createPane("canvas");
        if (typeof pane === "undefined") {
            throw new Error("No such pane: overlayPane.");
        }

        const mapContainer = map.getContainer();

        const mapContainerWidth = mapContainer.offsetWidth;
        const mapContainerHeight = mapContainer.offsetHeight;

        const canvasElement = Leaflet.DomUtil.create("canvas");

        canvasElement.style.height = `${mapContainerHeight}px`;
        canvasElement.style.width = `${mapContainerWidth}px`;

        canvasElement.setAttribute("height", String(mapContainerHeight));
        canvasElement.setAttribute("width", String(mapContainerWidth));

        this.mapPane = map.getPane("mapPane");
        if (typeof this.mapPane === "undefined") {
            throw new Error("No such pane: mapPane.");
        }

        this.canvasElement = canvasElement;
        // noinspection JSUnresolvedReference
        pane.appendChild(this.canvasElement);

        // map.on("zoomend viewreset", this._update, this);
    },

    /*
    _update() {
        const mapContainerTransform = parseTransformTranslate3DOnElement(this.mapPane);
        if (mapContainerTransform === null) {
            log.error("map container transform was null!");
            return;
        }

        const inverseX = mapContainerTransform.x * -1;
        const inverseY = mapContainerTransform.y * -1;
        const inverseZ = mapContainerTransform.z * -1;

        // noinspection JSUnresolvedReference
        this.canvasElement.style.transform = `translate3D(${inverseX}px, ${inverseY}px, ${inverseZ}px)`;
    },
     */

    onRemove(_map: Leaflet.Map) {
        // noinspection JSUnresolvedReference
        this.canvasElement.remove();
    },

    getCanvasElement(): HTMLCanvasElement {
        // noinspection JSUnresolvedReference
        return this.canvasElement;
    }
});
