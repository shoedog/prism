const express = require('express');
const app = express();

function h1(req, res) { res.sendStatus(204); }
function h2(req, res) { res.sendStatus(201); }

app.route('/r').get(h1).post(h2);

module.exports = app;
