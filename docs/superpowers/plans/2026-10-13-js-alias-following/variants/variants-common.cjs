// Shared pieces of the variant generator: invocation forms, file extensions, callee sources.
'use strict';

const FORMS = {
  call: { T: 'f', ALT: 'g', trap: 'decoy', fresh: 'h', site: (L) => `return ${L}(1); // SITE` },
  jsx: { T: 'F', ALT: 'G', trap: 'Decoy', fresh: 'H', site: (L) => `return <${L} />; // SITE` },
  opt: { T: 'f', ALT: 'g', trap: 'decoy', fresh: 'h', site: (L) => `return ${L}?.(1); // SITE` },
  new: { T: 'K', ALT: 'K2', trap: 'DecoyK', fresh: 'HK', site: (L) => `return new ${L}(1); // SITE` },
};

function ext(lang, form) {
  if (lang === 'ts') return form === 'jsx' ? 'tsx' : 'ts';
  return form === 'jsx' ? 'jsx' : 'js';
}
const ty = (lang, s) => (lang === 'ts' ? s : '');

function calleeSrc(lang) {
  const n = ty(lang, ': number');
  const a = ty(lang, ': any');
  return [
    `export function f(x${n}) { return x; }`,
    `export function g(x${n}) { return x + 1; }`,
    `export function F(props${a}) { return null; }`,
    `export function G(props${a}) { return null; }`,
    `export class K { constructor(x${n}) { this.v = x; } ${ty(lang, 'v: number;')} }`,
    `export class K2 { constructor(x${n}) { this.v = x + 1; } ${ty(lang, 'v: number;')} }`,
    `export const renamed = f;`,
    '',
  ].join('\n');
}
function otherSrc(lang) {
  const n = ty(lang, ': number');
  const a = ty(lang, ': any');
  return [
    `export function decoy(x${n}) { return -x; }`,
    `export function Decoy(props${a}) { return null; }`,
    `export class DecoyK { constructor(x${n}) { this.v = -x; } ${ty(lang, 'v: number;')} }`,
    '',
  ].join('\n');
}

function baseFiles(lang, naming) {
  const e = ext(lang, 'call');
  const files = { [`callee.${e}`]: calleeSrc(lang) };
  if (naming === 'trap') files[`other.${e}`] = otherSrc(lang);
  return files;
}

module.exports = { FORMS, ext, ty, calleeSrc, otherSrc, baseFiles };
