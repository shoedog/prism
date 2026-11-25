const express = require('express');
const reports = require('./reports.cjs');
const app = express();

app.use('/ops', reports.router);

module.exports = app;
