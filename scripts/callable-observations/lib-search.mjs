const aliases=new Map([
  ['es6','es2015'],['es7','es2016'],['esnext.symbol','es2019.symbol'],
  ['esnext.asynciterable','es2018.asynciterable'],['esnext.bigint','es2020.bigint'],
  ['esnext.string','es2024.string'],['esnext.weakref','es2021.weakref'],
  ['esnext.object','es2024.object'],['esnext.regexp','es2024.regexp'],
]);

export function libFileForReference(name) {
  if(typeof name!=='string')return null;
  const lower=name.toLowerCase();
  return `lib.${aliases.get(lower)??lower}.d.ts`;
}

export function libraryNameFromLibFile(libFile) {
  if(typeof libFile!=='string')return null;
  const parts=libFile.split('.');
  if(parts.length<4||parts[0]!=='lib'||parts.at(-2)!=='d'||parts.at(-1)!=='ts'||parts.slice(1,-2).some(p=>!p))return null;
  return `@typescript/lib-${parts[1]}${parts.slice(2,-2).map((part,index)=>`${index===0?'/':'-'}${part}`).join('')}`;
}
