const express = require('express');
const app = express();

function schedule(req, res) { res.send('schedule'); }
function status(req, res) { res.send('ready'); }

const registerGet = app.get.bind(app);
registerGet('/schedule', schedule);
app.get('/status', status);

module.exports = app;
