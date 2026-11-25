import runtime from 'node:process';
export function load(key) {
  const selected = runtime.env[key];
  return selected;
}
