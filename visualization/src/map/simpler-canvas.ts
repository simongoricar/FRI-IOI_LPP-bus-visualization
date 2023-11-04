import Leaflet from "leaflet";

export const CanvasV2 = Leaflet.Layer.extend({
    onAdd(map: Leaflet.Map) {
        const pane = map.getPane("overlayPane");
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

        this.canvasElement = canvasElement;
        pane.appendChild(this.canvasElement);
    },

    onRemove(_map: Leaflet.Map) {
        this.canvasElement.remove();
    },

    getCanvasElement(): HTMLCanvasElement {
        return this.canvasElement;
    }

    // createTile: function (coords) {
    //   var tile = document.createElement('canvas');
    //
    //   var tileSize = this.getTileSize();
    //   tile.setAttribute('width', tileSize.x);
    //   tile.setAttribute('height', tileSize.y);
    //
    //   var ctx = tile.getContext('2d');
    //
    //   // Draw whatever is needed in the canvas context
    //   // For example, circles which get bigger as we zoom in
    //   ctx.beginPath();
    //   ctx.arc(tileSize.x/2, tileSize.x/2, 4 + coords.z*4, 0, 2*Math.PI, false);
    //   ctx.fill();
    //
    //   return tile;
    // }
});
