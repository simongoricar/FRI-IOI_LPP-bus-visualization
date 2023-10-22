import { LPPBusAPI } from "./buses/api.ts";

console.log("Hello world!");

const api = new LPPBusAPI();
let stations = await api.getAllStations();
console.log(stations);
