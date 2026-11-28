let get: () => unknown = () => "unset";
function stash(g: () => unknown): {a: number} { get = g; return {a:1}; }
let /*@out*/key = "outside";
for (let /*@head*/key in stash((): unknown => /*@read*/key)) {
  /*@write*/key = "changed";
  console.log(key);
  try { console.log(get()); } catch { console.log("RHS TDZ"); }
}
console.log(key);
