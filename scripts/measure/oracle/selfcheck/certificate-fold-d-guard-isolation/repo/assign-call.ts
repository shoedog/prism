import { f } from './assign';
export function call(actual: number) { return f(/*@assign-actual*/actual); }
