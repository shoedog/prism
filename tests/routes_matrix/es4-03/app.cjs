const express = require('express');
const app = express();
const preview = null;

function queue(req, res) { res.send('queue'); }

app?.get('/queue', queue);
preview?.get('/preview', queue);

module.exports = app;
