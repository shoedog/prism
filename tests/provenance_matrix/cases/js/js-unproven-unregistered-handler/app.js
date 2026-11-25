function echo(req, res) {
  const term = req.query.term;
  return term;
}
echo({ query: { term: "fixed" } }, {});
