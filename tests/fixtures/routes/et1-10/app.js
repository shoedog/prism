const express = require('express');
const app = express();
function show(req, res) { res.end(); }
for (const method of ['get', 'post']) {
  app[method]('/items', show);
}
module.exports = app;
