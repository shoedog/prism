const express = require('express');
const app = express();

function mw(req, res, next) { next(); }

app.use(mw);

module.exports = app;
