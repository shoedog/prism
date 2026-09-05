function f() {
  let x = source();
  const thunk = () => sink(x);
  x = clean();
  return thunk;
}
