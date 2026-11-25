const express = require('express');
const app = express();

function named(req, res) { res.sendStatus(204); }

app.all('/ready', named);

module.exports = app;
