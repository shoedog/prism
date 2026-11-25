const express = require('express');
const app = express();

function prepare(req, res, next) { next(); }
function recover(error, req, res, next) { next(error); }
function finish(req, res) { res.send('export'); }

app.get('/export', [prepare, [(req, res, next) => next(), recover]], finish);

module.exports = app;
