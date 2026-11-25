const express = require('express');
const app = express();

function named(req, res) { res.sendStatus(204); }

app.get(/^\/downloads\/[a-z]+$/, named);

module.exports = app;
