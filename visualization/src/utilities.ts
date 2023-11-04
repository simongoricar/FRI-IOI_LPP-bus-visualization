export function clamp(value: number, minimum: number, maximum: number): number {
    if (value > maximum) {
        return maximum;
    } else if (value < minimum) {
        return minimum;
    } else {
        return value;
    }
}
