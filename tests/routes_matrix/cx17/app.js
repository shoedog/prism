const express = require('express');
const app = express();

function searchInventory(req, res) { res.sendStatus(204); }

app.search('/inventory', searchInventory);

module.exports = app;
