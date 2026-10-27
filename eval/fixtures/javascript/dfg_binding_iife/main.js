function outer() {
  let x = source();
  (function () { sink(x); })();
}
