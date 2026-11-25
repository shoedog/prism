class A {
  run(v) {
    observe(v); // @SINK
  }
  start(other) {
    const t=acquire(); // @SRC
    other.run(t); this.run('fixed');
  }
}
