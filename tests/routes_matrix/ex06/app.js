const express = require('express');
const app = express();

const site = require('./site');

app.get('/x', site.index);

module.exports = app;
