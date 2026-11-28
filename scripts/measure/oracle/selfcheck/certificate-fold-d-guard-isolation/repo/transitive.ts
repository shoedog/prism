declare const module: { exports: object };
export function f(/*@transitive-formal*/formal: number) { return formal; }
function mutate() {
  const downstream = upstream;
  downstream.exports = {};
}
const upstream = module;
mutate();
