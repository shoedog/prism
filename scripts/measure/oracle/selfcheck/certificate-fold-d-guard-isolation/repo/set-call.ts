import { f } from './set';
export function call(actual: number) { return f(/*@set-actual*/actual); }
