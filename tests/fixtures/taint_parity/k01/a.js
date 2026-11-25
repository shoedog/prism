function main(handler) {
  const t = getInput(); // @SRC
  handler.run(t);
}
main({ run: Function.prototype });
