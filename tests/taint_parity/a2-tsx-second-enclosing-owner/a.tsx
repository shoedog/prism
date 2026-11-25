function View(p: {value: string}) {
  observe(p.value); // @SINK
  return null;
}
function entry() {
  const t = acquire(); // @SRC
  return [0].map(function middle() {
    return [0].map(() => <View value={t}/>);
  });
}
