interface Logger {
    error(msg: string, ...params: any[]): void;
    warn(msg: string, ...params: any[]): void;
    info(msg: string, ...params: any[]): void;
    debug(msg: string, ...params: any[]): void;
    trace(msg: string, ...params: any[]): void;
}

declare const logger: Logger;

// emits to logger.info
function print(msg: string, ...params: any[]) {
    logger.info(msg, params)
}

/// emits to logger.error
function eprint(msg: string, ...params: any[]) {
    logger.error(msg, params)
}