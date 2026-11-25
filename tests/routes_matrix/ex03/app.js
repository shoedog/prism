const express = require('express');
const app = express();

const asyncHandler = require('./async-handler');

app.get('/x', asyncHandler(async (req, res) => { res.sendStatus(204); }));

module.exports = app;
