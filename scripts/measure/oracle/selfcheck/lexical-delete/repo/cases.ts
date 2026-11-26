export {};
const obj: { m?: number } = {};
obj.m = 1;
delete obj.m;
void obj.m;
const key = 'm';
const bag: { [k: string]: number } = {};
delete bag[key];
