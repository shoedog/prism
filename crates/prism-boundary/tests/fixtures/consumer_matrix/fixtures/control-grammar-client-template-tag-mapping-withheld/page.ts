import { recordSable } from './quay';

export function openLeaf(pieces: TemplateStringsArray, ...parts: number[]) {
  let path = pieces[0];
  for (const [index, part] of parts.entries()) path += part + pieces[index + 1];
  return recordSable('POST', path);
}
