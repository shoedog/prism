const express = require('express');
const app = express();

const asyncHandler = require('./async-handler');

function named(req, res) { res.sendStatus(204); }
const h = asyncHandler(named);

app.get('/', h);

module.exports = app;
