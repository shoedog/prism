function View(p: {children: string}) {
  observe(p.children); // @SINK
  return null;
}
function entry() {
  const t = acquire(); // @SRC
  return <View children={t}>fixed</View>;
}
