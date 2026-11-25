import { external } from "../external";
function local(value: number) { return value; }
function caller(actual: number) { return external(actual); }
