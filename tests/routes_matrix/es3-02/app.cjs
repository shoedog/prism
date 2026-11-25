const express = require('express');
const app = express();

function notes(req, res) { res.send('notes'); }
function current(req, res) { res.send('current reports'); }

app.get('/n\u006ftes', notes);
app.get('/reports' + '/current', current);

module.exports = app;
