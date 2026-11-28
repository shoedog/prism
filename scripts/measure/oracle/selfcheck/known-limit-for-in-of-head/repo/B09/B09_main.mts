let get = () => "missing";
for (let /*@head*/n of [0, 1]) {
  if (n === 0) get = () => "" + /*@read*/n;
  else { /*@write*/n = 7; console.log(get(), n); }
}
