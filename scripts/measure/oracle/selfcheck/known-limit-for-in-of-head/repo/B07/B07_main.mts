let /*@out*/item = () => "outer";
for (let /*@head*/item of [(): unknown => /*@read*/item]) {
  const get = item;
  /*@write*/item = () => "iteration";
  console.log(item());
  try { console.log(get()); } catch { console.log("RHS TDZ"); }
}
console.log(item());
