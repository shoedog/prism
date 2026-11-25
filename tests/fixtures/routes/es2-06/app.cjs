const express = require('express');
const app = express();

function summarize(req, res) { res.json([]); }

app.get('/summary', (req, res, next) => next(), (req, res, next) => next(), summarize);

module.exports = app;
