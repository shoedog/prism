const express = require('express');
const { closedForReview, measure, forward } = require('./wrappers.cjs');
const app = express();

function archive(req, res) { res.send('archive'); }
function summary(req, res) { res.send('summary'); }

app.get('/archive', closedForReview(archive));
app.get('/summary', measure(forward(summary)));

module.exports = app;
