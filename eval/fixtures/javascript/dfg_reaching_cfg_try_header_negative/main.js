function f() {
  let x = source();
  try {
    x = clean();
    throw new Error();
  } catch (error) {
    sink(x);
  }
}
