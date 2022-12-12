interface Logger {
    info(msg: string, ...params: any[]): void;
}

declare const logger: Logger;