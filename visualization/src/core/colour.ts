/**
 * A set of useful console colours and associated conversion functions.
 */

/**
 * A selection of pleasant console colours (generated using https://coolors.co).
 */
enum Colour {
    LIGHT_GRAY = "#CCDBDC",
    OPAL = "#9DC0BC",
    POWDER_BLUE = "#9AD1D4",
    CG_BLUE = "#007EA7",
    FRENCH_BLUE = "#0072BB",
    BDAZZLED_BLUE = "#345995",
    YELLOW_GREEN_CRAYOLA = "#CEEC97",
    STRAW = "#D0CE7C",
    BITTER_LIME = "#C3F73A",
    SAFFRON = "#EAC435",
    CAMEL = "#CF995F",
    TUMBLEWEED = "#F4B393",
    CADMIUM_ORANGE = "#E18335",
    OCHRE = "#D17A22",
    BEAVER = "#938274",
    GOLD_FUSION = "#736F4E",
    SHINY_SHAMROCK = "#68B684",
    MYRTLE_GREEN = "#1D7874",
    SLATE_GRAY = "#628395",
    DARK_LIVER = "#595358",
    PINE_TREE = "#313628",
    LAUREL_GREEN = "#A4AC96",
    RHYTHM = "#7C7287",
    PURPLE_NAVY = "#4F517D",
    DARK_PURPLE = "#372549",
    XIKETIC = "#1A1423",
    OLD_ROSE = "#AC7B7D",
    MOUNTBATTEN_PINK = "#977390",
    MAROON_X11 = "#BF1363",
}

/**
 * Converts a hexadecimal `#rrggbb` to corresponding RGB values.
 *
 * @param hexValue - hexadecimal representation of the colour.
 *        Allowed formats: `#fff`, `#ffffff` or any of those without leading `#`
 * @returns an array containing three values: red, green and blue value of the colour.
 */
const hexToRGB = (hexValue: string): number[] => {
    let fullHex = hexValue;

    // Remove the leading "#" if present
    if (fullHex.length === 4 || fullHex.length === 7) {
        fullHex = fullHex.substring(1, 7);
    }

    if (fullHex.length === 3) {
        fullHex = hexValue.charAt(0) + hexValue.charAt(0)
                + hexValue.charAt(1) + hexValue.charAt(1)
                + hexValue.charAt(2) + hexValue.charAt(2);
    } else if (fullHex.length !== 6) {
        throw new Error(`Invalid color: ${hexValue}`);
    }

    return [
        parseInt(fullHex.substring(0, 2), 16),
        parseInt(fullHex.substring(2, 4), 16),
        parseInt(fullHex.substring(4, 6), 16),
    ];
};

/**
 * Converts RGB values to HSL.
 *
 * For more information, see https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB
 *
 * @param red - red value (0 - 255)
 * @param green - green value (0 - 255)
 * @param blue - blue value (0 - 255)
 * @returns an array containing three values: hue, saturation and lightness
 */
const rgbToHSL = (red: number, green: number, blue: number): number[] => {
    const r1 = red / 255;
    const g1 = green / 255;
    const b1 = blue / 255;

    const cMax = Math.max(r1, g1, b1);
    const cMin = Math.min(r1, g1, b1);

    const delta = cMax - cMin;

    let hue;
    if (delta === 0) {
        hue = 0;
    } else if (cMax === r1) {
        hue = ((g1 - b1) / delta) % 6;
    } else if (cMax === g1) {
        hue = ((b1 - r1) / delta) + 2;
    } else {
        hue = ((r1 - g1) / delta) + 4;
    }

    hue = Math.round(hue * 60);
    if (hue < 0) {
        hue += 360;
    }

    let lightness = (cMin + cMax) / 2;

    let saturation;
    if (delta === 0) {
        saturation = 0;
    } else {
        saturation = delta / (1 - Math.abs(2 * lightness - 1));
    }

    saturation = Math.round(saturation * 100);
    lightness = Math.round(lightness * 100);

    return [
        hue, saturation, lightness,
    ];
};

export { Colour, hexToRGB, rgbToHSL };
