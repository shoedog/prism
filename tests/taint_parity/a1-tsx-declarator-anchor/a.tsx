function View(p: {value: string}) {
  observe(p.value); // @SINK
  return null;
}
function entry() {
  const t = acquire(); // @SRC
  const node = <View value={t}/>;
  return node;
}
