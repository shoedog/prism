const express = require('express');
const app = express();

function bindCollection(req, res) { res.sendStatus(204); }

app.bind('/collections', bindCollection);

module.exports = app;
