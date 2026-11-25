function entry() {
  const t = acquire(); // @SRC
  const u = acquire();
  const clean = escapeHtml(t);
  return <><div dangerouslySetInnerHTML={{__html: clean}}/><div dangerouslySetInnerHTML={{__html: u}}/></>; // @SINK
}
