import { process } from "./util";

function run(): number {
    try {
        throw 1;
    } catch (process) {
        return process();
    }
}
