# LPP Bus visualisation

## 1. Preparation
### 1.1 Requesting daily timetables and other LPP data
Before you can use the visualisation, you need to request some timetable and station data from the LPP API.
To do this, you need to run the "recording" server inside `preparation`. You'll need [Rust](https://www.rust-lang.org/) for this.

The steps are as follows:

- Copy `preparation/data/configuration.TEMPLATE.toml` to `preparation/data/configuration.toml` and fill out any required fields.
- Build the project in release mode: run `cargo build --release` inside the `preparation` directory.
- To download data for the current day, run `cargo run --release -- --run-mode once` and wait for completion. This might take around half an hour.
  For any other available options, see `cargo run --release -- --help`.
- After the program exits successfully, you'll find the "recordings" in the configured output directory. 
  Copy the `route-details-*` and `station-details-*` bare files to `visualization/public/data` (create the directory if needed).

### 1.2 Download required assets
Download the Roboto font from [here](https://fonts.google.com/specimen/Roboto) into the `visualization/public/fonts/Roboto` directory 
(the file `visualization/public/fonts/Roboto/Roboto-Regular.ttf` should now exist, among others).

### 1.2 Building the visualization
You're now ready to build the visualization. You'll need [Node 18+](https://nodejs.org/en) and [Yarn](https://yarnpkg.com/).

The steps are as follows:
- In `visualization`: run `yarn install`
- In `visualization`: run `yarn build` (or `yarn dev` if you want to develop with hot reloading).

**If you built the project properly, you'll find the required files inside the `visualization/dist` directory. You're done!**
