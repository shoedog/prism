import { process } from "./util";

function run(process: () => number): number {
    return process();
}
