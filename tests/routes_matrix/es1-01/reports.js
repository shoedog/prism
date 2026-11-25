const express = require('express');
const reports = express.Router();
function summary(req, res) { res.send('summary'); }
reports.get('/summary', summary);
module.exports = reports;
