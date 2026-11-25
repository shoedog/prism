const express = require('express');
const app = express();

function user(req, res) { res.sendStatus(204); }
function asset(req, res) { res.sendStatus(204); }

app.get('/users/:id?', user);
app.get('/assets/*', asset);

module.exports = app;
