import { AllStationsSnapshot, GeographicalLocation } from "./models.ts";
import { clamp } from "../utilities.ts";
import { ProjectError } from "../core/errors.ts";

export class RelativeMinutes {
    // Can be fractional (and is not bounded to be smaller than 60).
    public minute: number;

    constructor(
      minute: number,
    ) {
        this.minute = minute;
    }

    static fromTimeOfDay(timeOfDay: TimeOfDay): RelativeMinutes {
        return new RelativeMinutes(timeOfDay.hour * 60 + timeOfDay.minute);
    }

    getTotalMinutes(): number {
        return this.minute;
    }
}

export class TimeOfDay {
    // Whole number.
    public hour: number;

    // Can be fractional.
    public minute: number;

    constructor(
      initialHour: number,
      initialMinute: number,
    ) {
        this.hour = clamp(initialHour, 1, 24);
        this.minute = clamp(initialMinute, 0, 59);
    }

    public resetTimeTo(hour: number, minute: number) {
        this.hour = clamp(hour, 1, 24);
        this.minute = clamp(minute, 0, 59);
    }

    private reflowUnits() {
        while (this.minute >= 60) {
            this.hour += 1;
            this.minute -= 60;
        }

        while (this.hour >= 25) {
            this.hour -= 24;
        }
    }

    public tick(minutes: number) {
        this.minute += minutes;
        this.reflowUnits();
    }

    public elapsedSince(preceedingTime: TimeOfDay): RelativeMinutes {
        if (preceedingTime.hour > this.hour) {
            throw new ProjectError("Invalid elapsedSince call: preceedingTime comes after us.");
        } else if (preceedingTime.hour === this.hour && preceedingTime.minute > this.minute) {
            throw new ProjectError("Invalid elapsedSince call: preceedingTime comes after us.");
        }

        return new RelativeMinutes(
          (this.hour - preceedingTime.hour) * 60 + (this.minute - preceedingTime.minute)
        );
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
        if (first.time.hour == second.time.hour) {
            return first.time.minute - second.time.minute;
        } else {
            return first.time.hour - second.time.hour;
        }
    });
}

export class ArrivalSet {
    public time: TimeOfDay;
    public arrivals: BusArrival[];

    constructor(time: TimeOfDay, arrivals: BusArrival[]) {
        this.time = time;
        this.arrivals = arrivals;
    }
}

export class BusArrivalPlayback {
    // This will be untouched, always.
    public stationsSnapshot: AllStationsSnapshot;

    public currentDayTime: TimeOfDay;
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
                              new TimeOfDay(hour, minute),
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

    private cutArrivalSetsBeforeOrAtTime(timeOfDay: TimeOfDay, arrivalSets: ArrivalSet[]): ArrivalSet[] {
        const hour = timeOfDay.hour;
        const minute = timeOfDay.minute;

        return arrivalSets.filter(
          (arrivalSet) => {
              const arrivalHour = arrivalSet.time.hour;
              const arrivalMinute = arrivalSet.time.minute;

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

    constructor(stationsSnapshot: AllStationsSnapshot, initialSimulationTime: TimeOfDay) {
        this.currentDayTime = initialSimulationTime;

        // This will be untouched, always.
        this.stationsSnapshot = stationsSnapshot;

        this.optimizedArrivals = this.cutArrivalSetsBeforeOrAtTime(
          this.currentDayTime,
          this.generateOptimizedArrivals()
        );
    }

    public getTimeOfDay(): TimeOfDay {
        return this.currentDayTime
    }

    public tick(minutes: number): ArrivalSet[] {
        this.currentDayTime.tick(minutes);

        const hour = this.currentDayTime.hour;
        const minute = this.currentDayTime.minute;

        let arrivalSets: ArrivalSet[] = [];

        while (true) {
            const nextArrivalSet = this.optimizedArrivals[0];
            if (typeof nextArrivalSet === "undefined") {
                break;
            }

            // If the next arrival is in this minute, we remove it
            // from our optimized arrivals and return it.
            if (nextArrivalSet.time.hour <= hour && nextArrivalSet.time.minute <= minute) {
                this.optimizedArrivals.shift();
                arrivalSets.push(nextArrivalSet);
            } else {
                break;
            }
        }

        // If we're past a whole day (having consumed all the arrivals),
        // we just reload them from the snapshot.
        if (this.optimizedArrivals.length === 0) {
            this.optimizedArrivals = this.cutArrivalSetsBeforeOrAtTime(
              this.currentDayTime,
              this.generateOptimizedArrivals()
            );
        }

        // If no arrival happens in the next minute, we return `null`.
        return arrivalSets;
    }
}
