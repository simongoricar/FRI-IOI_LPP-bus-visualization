import { AllRoutesSnapshot, AllStationsSnapshot } from "./models.ts";

export async function loadStationsSnapshot(
  fileName: string,
): Promise<AllStationsSnapshot> {
    const fullUrl = `/data/${fileName}`;

    let data = await fetch(fullUrl, {
        method: "GET",
        mode: "cors",
        credentials: "omit"
    });

    let jsonData = await data.json();

    return AllStationsSnapshot.fromRawData(jsonData);
}

export async function loadRoutesSnapshot(
  fileName: string,
): Promise<AllRoutesSnapshot> {
    const fullUrl = `/data/${fileName}`;

    let data = await fetch(fullUrl, {
        method: "GET",
        mode: "cors",
        credentials: "omit",
    });

    let jsonData = await data.json();

    return AllRoutesSnapshot.fromRawData(jsonData);
}
