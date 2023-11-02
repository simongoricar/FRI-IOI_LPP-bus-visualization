import { getOptionalField, getRequiredField } from "../core/utilities.ts";
import { LatLng } from "leaflet";


export class AllStationsSnapshot {
    public capturedAt: Date;
    public stationDetails: StationDetailsWithBusesAndTimetables[];

    constructor(
      capturedAt: Date,
      stationDetails: StationDetailsWithBusesAndTimetables[],
    ) {
        this.capturedAt = capturedAt;
        this.stationDetails = stationDetails;
    }

    public static fromRawData(rawData: Record<string, any>): AllStationsSnapshot {
        const capturedAt = new Date(Number(getRequiredField(rawData, "captured_at")));

        const rawStationDetails = getRequiredField(rawData, "station_details");
        let stationDetails: StationDetailsWithBusesAndTimetables[] = [];
        for (const rawStation of rawStationDetails) {
            stationDetails.push(StationDetailsWithBusesAndTimetables.fromRawData(rawStation));
        }

        return new AllStationsSnapshot(capturedAt, stationDetails);
    }
}


export class StationDetailsWithBusesAndTimetables {
    public stationCode: string;
    public internalStationId: number;
    public name: string;
    public location: GeographicalLocation;
    public tripsOnStation: TripOnStation[];
    public timetables: RouteGroupTimetable[];

    constructor(
      stationCode: string,
      internalStationId: number,
      name: string,
      location: GeographicalLocation,
      tripsOnStation: TripOnStation[],
      timetables: RouteGroupTimetable[],
    ) {
        this.stationCode = stationCode;
        this.internalStationId = internalStationId;
        this.name = name;
        this.location = location;
        this.tripsOnStation = tripsOnStation;
        this.timetables = timetables;
    }

    public static fromRawData(rawData: Record<string, any>): StationDetailsWithBusesAndTimetables {
        const stationCode = String(getRequiredField(rawData, "station_code"));
        const internalStationId = Number(getRequiredField(rawData, "internal_station_id"));
        const name = String(getRequiredField(rawData, "name"));
        const location = GeographicalLocation.fromRawData(getRequiredField(rawData, "location"));

        const rawTripsOnStation = getRequiredField(rawData, "trips_on_station");
        let tripOnStation: TripOnStation[] = [];
        for (const rawTrip of rawTripsOnStation) {
            tripOnStation.push(TripOnStation.fromRawData(rawTrip));
        }

        const rawTimetables = getRequiredField(rawData, "timetables");
        let timetables: RouteGroupTimetable[] = [];
        for (const rawTimetable of rawTimetables) {
            timetables.push(RouteGroupTimetable.fromRawData(rawTimetable));
        }

        return new StationDetailsWithBusesAndTimetables(
          stationCode,
          internalStationId,
          name,
          location,
          tripOnStation,
          timetables
        );
    }
}


export class GeographicalLocation {
    public latitude: number;
    public longitude: number;

    constructor(
      latitude: number,
      longitude: number,
    ) {
        this.latitude = latitude;
        this.longitude = longitude;
    }

    public static fromRawData(rawData: Record<string, any>): GeographicalLocation {
        const latitude = Number(getRequiredField(rawData, "latitude"));
        const longitude = Number(getRequiredField(rawData, "longitude"));

        return new GeographicalLocation(latitude, longitude);
    }

    public leafletLatLng(): LatLng {
        return new LatLng(this.latitude, this.longitude);
    }
}


export class TripOnStation {
    public routeId: string;
    public tripId: string;
    public route: string;
    public shortTripName: string | null;
    public tripName: string;
    public endsInGarage: boolean;

    constructor(
      routeId: string,
      tripId: string,
      route: string,
      shortTripName: string | null,
      tripName: string,
      endsInGarage: boolean,
    ) {
        this.routeId = routeId;
        this.tripId = tripId;
        this.route = route;
        this.shortTripName = shortTripName;
        this.tripName = tripName;
        this.endsInGarage = endsInGarage;
    }

    public static fromRawData(rawData: Record<string, any>): TripOnStation {
        const routeId = String(getRequiredField(rawData, "route_id"));
        const tripId = String(getRequiredField(rawData, "trip_id"));
        const route = String(getRequiredField(rawData, "route"));

        const rawShortTripName = getOptionalField(rawData, "short_trip_name", null);
        const shortTripName: string | null = rawShortTripName == null ? rawShortTripName : String(rawShortTripName);

        const tripName = String(getRequiredField(rawData, "trip_name"));
        const endsInGarage = Boolean(getRequiredField(rawData, "ends_in_garage"));

        return new TripOnStation(
          routeId,
          tripId,
          route,
          shortTripName,
          tripName,
          endsInGarage,
        )
    }
}

export class RouteGroupTimetable {
    public routeGroupName: number;
    public tripTimetables: TripTimetable[];

    constructor(
      routeGroupName: number,
      tripTimetables: TripTimetable[],
    ) {
        this.routeGroupName = routeGroupName;
        this.tripTimetables = tripTimetables;
    }

    public static fromRawData(rawData: Record<string, any>): RouteGroupTimetable {
        const routeGroupName = Number(getRequiredField(rawData, "route_group_name"));

        const rawTripTimetables = getRequiredField(rawData, "trip_timetables");
        let tripTimetables: TripTimetable[] = [];
        for (const rawEntry of rawTripTimetables) {
            tripTimetables.push(TripTimetable.fromRawData(rawEntry));
        }

        return new RouteGroupTimetable(routeGroupName, tripTimetables);
    }
}


export class TripTimetable {
    public route: string;
    public tripName: string;
    public shortTripName: string;
    public endsInGarage: boolean;
    public timetable: TimetableEntry[];
    public stations: StationOnTimetable[];

    constructor(
      route: string,
      tripName: string,
      shortTripName: string,
      endsInGarage: boolean,
      timetable: TimetableEntry[],
      stations: StationOnTimetable[],
    ) {
        this.route = route;
        this.tripName = tripName;
        this.shortTripName = shortTripName;
        this.endsInGarage = endsInGarage;
        this.timetable = timetable;
        this.stations = stations;
    }

    public static fromRawData(rawData: Record<string, any>): TripTimetable {
        const route = String(getRequiredField(rawData, "route"));
        const tripName = String(getRequiredField(rawData, "trip_name"));
        const shortTripName = String(getRequiredField(rawData, "short_trip_name"));
        const endsInGarage = Boolean(getRequiredField(rawData, "ends_in_garage"));

        const rawTimetable = getRequiredField(rawData, "timetable");
        let timetable: TimetableEntry[] = [];
        for (const rawEntry of rawTimetable) {
            timetable.push(TimetableEntry.fromRawData(rawEntry));
        }

        const rawStations = getRequiredField(rawData, "stations");
        let stations: StationOnTimetable[] = [];
        for (const rawEntry of rawStations) {
            stations.push(StationOnTimetable.fromRawData(rawEntry));
        }

        return new TripTimetable(
          route,
          tripName,
          shortTripName,
          endsInGarage,
          timetable,
          stations,
        );
    }
}

export class TimetableEntry {
    public hour: number;
    public minute: number;

    constructor(hour: number, minute: number) {
        this.hour = hour;
        this.minute = minute;
    }

    public static fromRawData(rawData: Record<string, any>): TimetableEntry {
        const hour = Number(getRequiredField(rawData, "hour"));
        const minute = Number(getRequiredField(rawData, "minute"));

        return new TimetableEntry(hour, minute);
    }
}


export class StationOnTimetable {
    public stationCode: string;
    public name: string;
    public stopNumber: number;

    constructor(stationCode: string, name: string, stopNumber: number) {
        this.stationCode = stationCode;
        this.name = name;
        this.stopNumber = stopNumber;
    }

    public static fromRawData(rawData: Record<string, any>): StationOnTimetable {
        const stationCode = String(getRequiredField(rawData, "station_code"));
        const name = String(getRequiredField(rawData, "name"));
        const stopNumber = Number(getRequiredField(rawData, "stop_number"));

        return new StationOnTimetable(stationCode, name, stopNumber);
    }
}
