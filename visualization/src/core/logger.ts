/**
 * Enhanced logging module
 */

import { Colour, hexToRGB, rgbToHSL } from "./colour";

type LoggingObject = any;

/*
 * HELPER FUNCTIONS
 */
/**
 * Pick a random colour out of the Colour enum (see colour.ts).
 *
 * @returns A random colour.
 */
const generateRandomColour = (): Colour => {
    const list = Object.values(Colour);
    const randomIndex = Math.floor(Math.random() * list.length);

    return list[randomIndex];
};

/**
 * Checks whether the value is an Object.
 * With help from https://stackoverflow.com/questions/8511281/check-if-a-value-is-an-object-in-javascript and lodash.
 *
 * @param value - Value to check.
 * @returns Boolean indicating whether the value is an object.
 */
const isObject = (
    value: LoggingObject,
): boolean => typeof value === "object" && value !== null;

/**
 * Compute a well-contrasted colour to the specified one.
 *
 * @param hexColour - base (background) colour
 * @returns White or black hex code depending on which one is better-contrasted to `hexColour`.
 */
const getContrastingTextHexColour = (hexColour: Colour | string): string => {
    const [red, green, blue] = hexToRGB(hexColour);
    const lightness = rgbToHSL(red, green, blue)[2];

    return lightness > 50 ? "#000" : "#fff";
};

/**
 * Logger class, a beautified logging facility.
 * Automatically silences debug logs in production.
 */
class Logger {
    public readonly name: string;

    private readonly backgroundColour: Colour | string;

    private readonly textColour: Colour | string;

    /**
     * Construct a new Logger.
     *
     * @constructor
     * @param name - Name for the Logger.
     * @param colour - Background colour (`Colour` or hex value) for this Logger.
     */
    constructor(name: string, colour?: Colour) {
        this.name = name.toUpperCase();

        if (colour) {
            this.backgroundColour = colour;
        } else {
            this.backgroundColour = generateRandomColour();
        }

        this.textColour = getContrastingTextHexColour(this.backgroundColour);
    }

    /**
     * Formats the content for console output.
     *
     * @param content - Content to prepare for output to console.
     * @returns A list of argument to pass to `console.info/error/...`.
     */
    formatForConsole = (content: LoggingObject): (string | null)[] => {
        const additionalArguments = [];
        let formattedMessage;

        if (isObject(content)) {
            formattedMessage = "%O";
            additionalArguments.push(content);
        } else {
            formattedMessage = content;
        }

        return [
            `%c ${this.name} %c ${formattedMessage}`,
            // This null is just to reset the formatting on the second %c
            `background-color: ${this.backgroundColour};
            color: ${this.textColour}; font-weight: bold`,
            null, ...additionalArguments,
        ];
    };

    /**
     * Formats the content for console output, more specifically for group labels.
     *
     * @param label - Group label to use.
     * @returns A list of argument to pass to `console.group`/`console.groupCollapsed`.
     */
    formatForConsoleGroup = (label: LoggingObject): (string | null)[] => {
        const mainStyles = `
            background-color: ${this.backgroundColour};
            color: ${this.textColour};
        `;
        const labelStyles = `${mainStyles} text-decoration: underline; font-weight: bold`;

        return [
            `%c [${this.name.toUpperCase()}] %c${label}%c %c`,
            mainStyles, labelStyles, mainStyles, null,
        ];
    };

    /*
     * Beautified logging functions
     */
    /**
     * Log some content to console.
     *
     * @param content - Content to output, can be an Object.
     */
    log = (content: LoggingObject): void => {
        console.log(...this.formatForConsole(content));
    };

    /**
     * Log some content to console with severity `debug`.
     *
     * @param content - Content to output, can be an Object.
     */
    debug = (content: LoggingObject): void => {
        console.debug(...this.formatForConsole(content));
    };

    /**
     * Log some content to console with severity `INFO`.
     *
     * @param content - Content to output, can be an Object.
     */
    info = (content: LoggingObject): void => {
        console.info(...this.formatForConsole(content));
    };

    /**
     * Log some content to console with severity `WARN`.
     *
     * @param content - Content to output, can be an Object.
     */
    warn = (content: LoggingObject): void => {
        console.warn(...this.formatForConsole(content));
    };

    /**
     * Log some content to console with severity `ERROR`.
     *
     * @param content - Content to output, can be an Object.
     */
    error = (content: LoggingObject): void => {
        console.error(...this.formatForConsole(content));
    };

    /**
     * Creates a new group in the console. This indents messages by
     * an additional level, until `console.groupEnd()` is called.
     *
     * @param label - Group label to use.
     */
    group = (label: LoggingObject): void => {
        console.group(...this.formatForConsoleGroup(label));
    };

    /**
     * Creates a new group in the console. However, the group is initially collapsed.
     * This indents messages by an additional level, until `console.groupEnd()` is called.
     *
     * @param label - Group label to use.
     */
    groupCollapsed = (label: LoggingObject): void => {
        console.groupCollapsed(...this.formatForConsoleGroup(label));
    };

    /*
     * Other console functions pass-through
     */
    assert = (condition?: boolean, ...data: never[]): void => {
        console.assert(condition, ...data);
    };

    clear = (): void => {
        console.clear();
    };

    count = (label?: string): void => {
        console.count(label);
    };

    countReset = (label?: string): void => {
        console.countReset(label);
    };

    dir = (item?: any, options?: any): void => {
        console.dir(item, options);
    };

    dirxml = (...data: never[]): void => {
        console.dirxml(...data);
    };

    /**
     * Exit the current group in the console
     * (created by `console.group` or `console.groupCollapsed`).
     */
    groupEnd = (): void => {
        console.groupEnd();
    };

    table = (tabularData?: never, properties?: string[]): void => {
        console.table(tabularData, properties);
    };

    time = (label?: string): void => {
        console.time(label);
    };

    timeEnd = (label?: string): void => {
        console.timeEnd(label);
    };

    timeLog = (label?: string, ...data: never[]): void => {
        console.timeLog(label, ...data);
    };

    timeStamp = (label?: string): void => {
        console.timeStamp(label);
    };

    trace = (...data: never[]): void => {
        console.trace(...data);
    };
}

// Re-export Colour so the user doesn't need to import two modules
export { Colour };
export default Logger;
