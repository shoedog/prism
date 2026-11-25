const express = require('express');
const app = express();
app.get('/known', function known(_req, res) { res.end(); });
const layer = app._router.stack.at(-1);
app._router.stack.push({ ...layer });
module.exports = app;
