function entry(record) {
  const t = acquire(); // @SRC
  record.observeValue(t); // @SINK
}
entry(observer);
