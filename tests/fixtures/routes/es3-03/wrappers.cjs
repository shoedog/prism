function closedForReview(_handler) {
  return function closed(req, res) { res.send('closed for review'); };
}
function measure(handler) {
  return function measured(req, res) { handler(req, res); };
}
function forward(handler) {
  return function forwarded(req, res) { return handler(req, res); };
}
module.exports = { closedForReview, measure, forward };
