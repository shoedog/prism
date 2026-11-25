const express = require('express');
const retired = express.Router({ strict: true });
const active = express.Router();

function oldReport(req, res) { res.send('retired'); }
function current(req, res) { res.send('current'); }

retired.get('/retired', oldReport);
active.get('/current', current);
module.exports = { router: active };
exports.router = retired;
