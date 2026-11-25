const express = require('express');
const app = express();

function healthOptions(req, res) { res.sendStatus(204); }
function healthHead(req, res) { res.sendStatus(204); }

app.options('/health', healthOptions);
app.head('/health', healthHead);

module.exports = app;
