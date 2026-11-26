const express = require('express');
const app = express();
const child = express();

function items(req, res) { res.json([]); }
child.get('/items', items);
app.use('/api', child);

module.exports = app;
module.exports.child = child;
