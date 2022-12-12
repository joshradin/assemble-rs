class Id {
    readonly parent: Id | null;
    readonly identifier: string;

    constructor(parent: Id | null, identifier: string) {
        this.parent = parent;
        this.identifier = identifier;
    }

    is(other: Id | null): boolean {
        if (this.identifier !== other?.identifier) {
            return false
        }
        return this.parent?.is(other?.parent) ?? false;
    }
}
