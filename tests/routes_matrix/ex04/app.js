const express = require('express');
const app = express();

const asyncHandler = require('./async-handler');

function named(req, res) { res.sendStatus(204); }

app.get('/x', asyncHandler(named));

module.exports = app;
