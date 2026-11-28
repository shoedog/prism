import { f } from './control';
export function call(actual: number) { return f(/*@control-actual*/actual); }
