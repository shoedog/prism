class Runner {
  run(x) { // @SINK
    observe(x);
  }
}
function main(receiver) {
  const t = acquire(); // @SRC
  receiver.run(t);
}
main(new Runner());
