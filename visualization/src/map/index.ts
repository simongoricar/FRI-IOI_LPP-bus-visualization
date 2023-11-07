import Leaflet from "leaflet";
import "leaflet-providers";
import { CanvasV2 } from "./simpler-canvas.ts";
import Secrets from "../secrets.ts";

const jawgAccessToken = Secrets.jawgLabsSecret;

export const DEFAULT_LEAFLET_MAP_OPTIONS: Leaflet.MapOptions = {
    attributionControl: true,
    zoomControl: false,
    center: Leaflet.latLng(46.057838, 14.509823),
    zoom: 13,
    maxZoom: 16,
    minZoom: 13.5,
    zoomSnap: 0.5,
    zoomDelta: 0.5,
    inertia: false,
    wheelPxPerZoomLevel: 90,
    keyboard: false,
    maxBounds: Leaflet.latLngBounds(
      Leaflet.latLng(46.088995, 14.435850),
      Leaflet.latLng(46.009894, 14.588667)
    ),
    // renderer: Leaflet.canvas(),
};

export const DEFAULT_LEAFLET_TILE_OPTIONS: Leaflet.TileLayerOptions = {
    maxZoom: 18,
    attribution: undefined,
    // attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
};

export type MapOptions = {
    mapOptions?: Leaflet.MapOptions,
    tileLayerUrlTemplate?: string,
    tileLayerOptions?: Leaflet.TileLayerOptions,
};


export default class IOIMap {
    public map: Leaflet.Map;
    public tiles: Leaflet.TileLayer;
    public canvas: HTMLCanvasElement;
    public leafletCanvas: Leaflet.Canvas;

    constructor(
      mapElement: HTMLElement,
      options?: MapOptions,
    ) {
        this.map = new Leaflet.Map(
          mapElement,
          {
              ...DEFAULT_LEAFLET_MAP_OPTIONS,
              ...options?.mapOptions
          }
        );
        this.map.attributionControl.setPrefix(false);

        this.tiles = Leaflet.tileLayer.provider(
          "Jawg.Dark",
          // "Stadia.AlidadeSmoothDark",
          {
              ...DEFAULT_LEAFLET_TILE_OPTIONS,
              ...options?.tileLayerOptions,
              accessToken: jawgAccessToken,
          }
        );
        this.tiles.addTo(this.map);

        // this.tiles = new Leaflet.TileLayer(
        //   options?.tileLayerUrlTemplate
        //     || "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
        //   {
        //       ...DEFAULT_LEAFLET_TILE_OPTIONS,
        //       ...options?.tileLayerOptions
        //   }
        // );
        // this.tiles.addTo(this.map);

        // @ts-ignore
        // Leaflet.Layer.CanvasOverlay = Leaflet.Layer.extend({
        //     onAdd(map: Leaflet.Map | Leaflet.LayerGroup) {
        //         this._container = Leaflet.DomUtil.create("div", "leaflet-layer");
        //         this._container.appendChild(canvasElement);
        //
        //         map.on("move", () => {
        //             const newPos = map.dragging._draggable._newPos;
        //             if (newPos) {
        //                 canvasContext.canvas.style.transform = `translate(${-newPos.x}px, ${-newPos.y}px)`;
        //             }
        //         });
        //     },
        //
        //     onRemove(map: Leaflet.Map | Leaflet.LayerGroup) {
        //         Leaflet.DomUtil.remove(this._container);
        //
        //         map.off("move");
        //         delete this._container;
        //     }
        // });

        // this.leafletCanvas = new Leaflet.Canvas();
        // this.map.addLayer(this.leafletCanvas);

        this.leafletCanvas = new CanvasV2();
        this.map.addLayer(this.leafletCanvas);

        // @ts-ignore
        this.canvas = this.leafletCanvas.getCanvasElement();
    }
}
