function D(props) {
  observe(props.a); // @SINK
  return null;
}
function main() {
  const x = input(); // @SRC
  return <D a={x} />;
}
