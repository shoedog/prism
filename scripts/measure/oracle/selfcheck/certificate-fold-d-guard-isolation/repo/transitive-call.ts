import { f } from './transitive';
export function call(actual: number) { return f(/*@transitive-actual*/actual); }
