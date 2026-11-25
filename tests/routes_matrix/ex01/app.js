const express = require('express');
const app = express();

function named(req, res) { res.sendStatus(204); }

app.get('/x', named);

module.exports = app;
