import { AllStationsSnapshot, GeographicalLocation, StationDetailsWithBusesAndTimetables } from "./models.ts";
import { LatLng } from "leaflet";

export class StationSearcher {
    private readonly stationsSnapshot: AllStationsSnapshot;
    private readonly optimizedStationsPerLocation: [
      GeographicalLocation,
      StationDetailsWithBusesAndTimetables
    ][];

    private generateOptimizedStations(): [GeographicalLocation, StationDetailsWithBusesAndTimetables][] {
        let optimizedStationsPerLocation:
          [GeographicalLocation, StationDetailsWithBusesAndTimetables][] = [];

        for (const station of this.stationsSnapshot.stationDetails) {
            optimizedStationsPerLocation.push([
              station.location,
              station
            ]);
        }

        return optimizedStationsPerLocation;
    }

    constructor(snapshot: AllStationsSnapshot) {
        this.stationsSnapshot = snapshot;
        this.optimizedStationsPerLocation = this.generateOptimizedStations();
    }

    getClosestStation(
      targetLocation: LatLng,
    ): [number, StationDetailsWithBusesAndTimetables] | null {
        let closestStation: StationDetailsWithBusesAndTimetables | null = null;
        let closestDistance: number | null = null;

        for (const [location, station] of this.optimizedStationsPerLocation) {
            const distanceToTarget = location.distanceToLeafetLatLng(targetLocation);

            if (closestStation === null || distanceToTarget < (closestDistance || Infinity)) {
                closestStation = station;
                closestDistance = distanceToTarget;
            }
        }

        if (closestStation === null || closestDistance === null) {
            return null;
        } else {
            return [closestDistance, closestStation];
        }
    }
}
