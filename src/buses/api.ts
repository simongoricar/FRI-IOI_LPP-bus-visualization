import Logger, { Colour } from "../core/logger.ts";
import { StationDetails } from "./models.ts";
import { APIError } from "../core/errors.ts";
import { isObject } from "../core/utilities.ts";

// TODO Figure out a better solution than CORS-Anywhere.
export const LPP_API_BASE_URL: string = "http://localhost:8855/https://data.lpp.si/api/";
export const LPP_API_DEFAULT_FETCH_OPTIONS: RequestInit = {
    method: "GET",
    cache: "no-cache",
    mode: "cors",
    credentials: "omit",
};

/**
 * Returns a full LPP API url given the `subUrl`.
 * The sub-URL should *not* begin with a forward slash.
 *
 * @param subUrl Sub-URL to join on top of `LPP_API_BASE_URL`.
 */
export function constructLppApiUrl(subUrl: string): string {
    return LPP_API_BASE_URL + subUrl
}

export type ResponseValidationContext = {
    requestPath: string,
};

export function validateAsJSONResponse(response: Response, context: ResponseValidationContext): void {
    if (response.status >= 500 && response.status < 600) {
        throw new APIError(
          `Requested ${context.requestPath}, server responded with status `
          + `${response.status} ${response.statusText}!`
        );
    } else if (!(response.status >= 200 && response.status < 300)) {
        throw new APIError(
          `Requested ${context.requestPath}, server responded with status `
          + `${response.status} ${response.statusText}!`
        );
    }

    const contentType = response.headers.get("Content-Type")?.split(";")[0];
    if (!(contentType === "application/json")) {
        throw new APIError(
          `Requested ${context.requestPath} and expected JSON, but Content-Type is ${contentType}`
        );
    }
}

export async function extractJSONFromResponse(
  response: Response,
): Promise<Record<string, any>> {
    const jsonData = await response.json();

    if (!isObject(jsonData)) {
        const simpleType = Object.prototype.toString.call(jsonData);
        throw new APIError(`Expected JSON object (map), got ${simpleType}.`);
    }

    return jsonData;
}

export class LPPBusAPI {
    private logger: Logger

    constructor() {
        this.logger = new Logger(
          "LPP-Bus-API",
          Colour.CAMEL
        );
    }

    /**
     * Returns a list of all available stations.
     */
    async getAllStations(): Promise<StationDetails[]> {
        const STATION_DETAILS_URL = constructLppApiUrl("station/station-details");

        this.logger.info("Requesting all stations.");

        const response = await fetch(STATION_DETAILS_URL, LPP_API_DEFAULT_FETCH_OPTIONS);
        validateAsJSONResponse(response, { requestPath: STATION_DETAILS_URL });
        const responseJson = await extractJSONFromResponse(response);

        const success = Boolean(responseJson?.success || false);
        if (!success) {
            throw new APIError("Response received, but success field is not true?!");
        }

        // Iterate over all returned data and extract details about stations.
        const dataArray: Record<string, any>[] | undefined = responseJson?.data;
        if (typeof dataArray === "undefined") {
            throw new APIError(
              "Response received and success is true, but the data field does not exist!?"
            );
        }

        let stations: StationDetails[] = [];
        for (const data of dataArray) {
            let stationDetails = StationDetails.newFromUncheckedRawData(data);
            stations.push(stationDetails);
        }

        return stations
    }
}
