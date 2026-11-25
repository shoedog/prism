function View(p: {key: string}) {
  observe(p.key); // @SINK
  return null;
}
function entry() {
  const t = acquire(); // @SRC
  return <View key={t}/>;
}
