const express = require('express');
const app = express();

function named(req, res) { res.sendStatus(204); }
const section = 'archived';

app.get(`/catalog`, named);
app.get(`/groups/${section}`, named);

module.exports = app;
