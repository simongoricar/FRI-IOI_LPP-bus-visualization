import { AllRoutesSnapshot, AllStationsSnapshot, GeographicalLocation } from "./models.ts";
import { clamp } from "../utilities.ts";

export class SimulatedTimeOfDay {
    public hour: number;
    public minute: number;

    constructor(
      initialHour: number,
      initialMinute: number,
    ) {
        this.hour = clamp(initialHour, 1, 24);
        this.minute = clamp(initialMinute, 0, 59);
    }

    resetTimeTo(hour: number, minute: number) {
        this.hour = clamp(hour, 1, 24);
        this.minute = clamp(minute, 0, 59);
    }

    tickOneMinute() {
        this.minute += 1;

        if (this.minute >= 60) {
            this.hour += 1;
            this.minute = 0;
        }

        // LPP API's hours go from 1 to 24 (e.g. 24:22 is 22 minutes after midnight).
        if (this.hour >= 25) {
            this.hour = 1;
        }
    }
}

export class BusArrival {
    public route: string;
    public tripName: string;
    public location: GeographicalLocation;

    constructor(route: string, tripName: string, location: GeographicalLocation) {
        this.route = route;
        this.tripName = tripName;
        this.location = location;
    }
}

export function sortArrivalSetArray(array: ArrivalSet[]) {
    array.sort((first, second) => {
        if (first.hour == second.hour) {
            return first.minute - second.minute;
        } else {
            return first.hour - second.hour;
        }
    });
}

export class ArrivalSet {
    public hour: number;
    public minute: number;
    public arrivals: BusArrival[];

    constructor(hour: number, minute: number, arrivals: BusArrival[]) {
        this.hour = hour;
        this.minute = minute;
        this.arrivals = arrivals;
    }
}

export class BusArrivalPlayback {
    // This will be untouched, always.
    public stationsSnapshot: AllStationsSnapshot;

    public currentDayTime: SimulatedTimeOfDay;
    public optimizedArrivals: ArrivalSet[];

    private generateOptimizedArrivals(): ArrivalSet[] {
        let optimizedArrivals: Map<string, ArrivalSet> = new Map();

        for (const stationDetails of this.stationsSnapshot.stationDetails) {
            const location = stationDetails.location.clone();

            for (const timetable of stationDetails.timetables) {
                for (const tripTimetable of timetable.tripTimetables) {
                    const route = tripTimetable.route;
                    const tripName = tripTimetable.tripName;

                    for (const timetableEntry of tripTimetable.timetable) {
                        const hour = timetableEntry.hour;
                        const minute = timetableEntry.minute;

                        const mapKey = `${hour}:${minute}`;

                        if (optimizedArrivals.has(mapKey)) {
                            const existingArrivalSet = optimizedArrivals.get(mapKey);
                            if (typeof existingArrivalSet === "undefined") {
                                throw new Error("BUG: How could existingArrivalSet be undefined, we just checked?!");
                            }

                            existingArrivalSet.arrivals.push(new BusArrival(route, tripName, location));
                        } else {
                            const newArrivalSet = new ArrivalSet(
                              hour,
                              minute,
                              [
                                new BusArrival(route, tripName, location)
                              ]
                            );

                            optimizedArrivals.set(mapKey, newArrivalSet);
                        }
                    }
                }
            }
        }

        let optimizedArrivalsArray = Array
          .from(optimizedArrivals)
          .map(([_, value]) => value);

        sortArrivalSetArray(optimizedArrivalsArray);

        return optimizedArrivalsArray;
    }

    private cutArrivalSetsBeforeOrAtTime(timeOfDay: SimulatedTimeOfDay, arrivalSets: ArrivalSet[]): ArrivalSet[] {
        const hour = timeOfDay.hour;
        const minute = timeOfDay.minute;

        return arrivalSets.filter(
          (arrivalSet) => {
              const arrivalHour = arrivalSet.hour;
              const arrivalMinute = arrivalSet.minute;

              if (arrivalHour < hour) {
                  return false;
              }

              if (arrivalHour == hour) {
                  if (arrivalMinute <= minute) {
                      return false;
                  }
              }

              return true;
          }
        );
    }

    constructor(stationsSnapshot: AllStationsSnapshot) {
        // DEBUGONLY
        this.currentDayTime = new SimulatedTimeOfDay(10, 30);

        // This will be untouched, always.
        this.stationsSnapshot = stationsSnapshot;

        this.optimizedArrivals = this.cutArrivalSetsBeforeOrAtTime(
          this.currentDayTime,
          this.generateOptimizedArrivals()
        );
    }

    getTimeOfDay(): SimulatedTimeOfDay {
        return this.currentDayTime
    }

    tickOneMinuteForward(): ArrivalSet | null {
        this.currentDayTime.tickOneMinute();

        const hour = this.currentDayTime.hour;
        const minute = this.currentDayTime.minute;

        // If we're past a whole day (having consumed all the arrivals),
        // we just reload them from the snapshot.
        if (this.optimizedArrivals.length === 0) {
            this.optimizedArrivals = this.cutArrivalSetsBeforeOrAtTime(
              this.currentDayTime,
              this.generateOptimizedArrivals()
            );
            return null;
        }

        const nextArrivalSet = this.optimizedArrivals[0];

        // If the next arrival is in this minute, we remove it
        // from our optimized arrivals and return it.
        if (nextArrivalSet.hour == hour && nextArrivalSet.minute === minute) {
            this.optimizedArrivals.shift();
            return nextArrivalSet;
        }

        // If no arrival happens in the next minute, we return `null`.
        return null;
    }
}
