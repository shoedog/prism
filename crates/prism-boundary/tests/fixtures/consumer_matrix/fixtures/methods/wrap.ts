import { recordSable } from './quay';

export default function stampWick(path: string) {
  return recordSable('POST', path);
}
export function byVerb(verb: string, path: string) {
  return recordSable(verb.toUpperCase(), path);
}
export function byOptions(path: string, data: unknown, options: { verb: string }) {
  return recordSable(options.verb.toUpperCase(), path, data);
}
export const via = {
  withVerb(path: string, verb: string) {
    return recordSable(verb.toUpperCase(), path);
  },
  withOptions(path: string, options: { verb: string }) {
    return recordSable(options.verb.toUpperCase(), path);
  },
};
export function defer(resolvePath: () => string) {
  return recordSable('GET', '/wicks/' + resolvePath());
}
