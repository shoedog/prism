function View(p: {title: string}) {
  observe(p.title); // @SINK
  return null;
}
function entry() {
  const title = acquire(); // @SRC
  return <View title={title}/>;
}
