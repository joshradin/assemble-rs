class Assemble {
    public current_dir: string;
    private _project_dir: string | null = null;


    constructor(current_dir: string) {
        this.current_dir = current_dir;
    }

    get project_dir(): string {
        return this._project_dir ? this._project_dir : this.current_dir;
    }

    set project_dir(value: string) {
        this._project_dir = value;
    }
}

declare const current_dir: string;

let assemble: Assemble = new Assemble(current_dir);

