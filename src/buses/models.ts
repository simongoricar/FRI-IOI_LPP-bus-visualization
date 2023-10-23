import { getRequiredField } from "../core/utilities.ts";
import { ResponseContentError } from "../core/errors.ts";
import { LatLng } from "leaflet";

export type RawStationDetails = {
    int_id: number,
    latitude: number,
    longitude: number,
    name: string,
    ref_id: string,
    route_groups_on_station: string[],
};

export class StationDetails {
    // LPP docs: Integer ID of station
    // (from attribute `int_id`)
    id: number

    // LPP docs: Geo latitude of station
    latitude: number

    // LPP docs: Geo longitude of station
    longitude: number

    // LPP docs: User friendly name of the station
    name: string

    // LPP docs: Ref ID / station code of the station (ex. "600011")
    // (from attribute `ref_id`)
    stationCode: string

    // LPP docs: Array of route groups on this station.
    // This contains only route group numbers (1,2,6...).
    // If `show-subroutes=1` is set, this will also include routes like 19I, 19B... with suffixes.
    routeGroupsOnStation: string[]

    constructor(
      id: number,
      latitude: number,
      longitude: number,
      name: string,
      stationCode: string,
      routeGroupsOnStation: string[]
    ) {
        this.id = id;
        this.latitude = latitude;
        this.longitude = longitude;
        this.name = name;
        this.stationCode = stationCode;
        this.routeGroupsOnStation = routeGroupsOnStation;
    }

    public static newFromUncheckedRawData(data: Partial<RawStationDetails>): StationDetails {
        const id = Number(getRequiredField(data, "int_id"));

        const latitude = Number(getRequiredField(data, "latitude"));
        const longitude = Number(getRequiredField(data, "longitude"));

        const name = String(getRequiredField(data, "name"));
        const stationCode = String(getRequiredField(data, "ref_id"));

        let routeGroupsOnStation = [];

        const rawRouteGroupsOnStation = getRequiredField(data, "route_groups_on_station");
        if (!Array.isArray(rawRouteGroupsOnStation)) {
            throw new ResponseContentError("Expected route_groups_on_station to be an array.");
        }

        for (const routeGroupName of rawRouteGroupsOnStation) {
            routeGroupsOnStation.push(String(routeGroupName));
        }


        return new StationDetails(
          id,
          latitude,
          longitude,
          name,
          stationCode,
          routeGroupsOnStation
        );
    }

    public latLng(): LatLng {
        return new LatLng(this.latitude, this.longitude)
    }

    public toString(): string {
        return `Station "${this.name}" <id=${this.id},stationCode=${this.stationCode}, \
                ${this.routeGroupsOnStation.length} route groups>`;
    }
}
