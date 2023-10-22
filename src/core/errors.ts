/**
 * General project error type.
 */
export class ProjectError extends Error {
    constructor(message: string = "") {
        super(message);
    }
}


export class APIError extends ProjectError {
    constructor(message: string = "") {
        super(message);
    }
}

export class ResponseContentError extends APIError {
    constructor(message: string = "") {
        super(message);
    }
}
