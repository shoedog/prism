const express = require('express');
const app = express();

function mark(req, res, next) { next(); }
function showLog(req, res) { res.json({ entries: [] }); }

app.get('/logs', [mark, showLog]);

module.exports = app;
