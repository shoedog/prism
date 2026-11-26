import { f } from './setPrototypeOf';
export function call(actual: number) { return f(/*@setPrototypeOf-actual*/actual); }
