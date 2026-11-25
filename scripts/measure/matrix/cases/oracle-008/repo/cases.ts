const value = 1;
function outer() {
  function inner() {
    const value = 2;
    return value;
  }
  return inner();
}
