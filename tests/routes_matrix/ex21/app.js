const express = require('express');
const app = express();

const accounts = require('./accounts');

app.use('/v1', accounts);

module.exports = app;
