const express = require('express');
const audit = express.Router();
const archive = express.Router({ mergeParams: true });
function recent(req, res) { res.send('recent'); }
function history(req, res) { res.send('history'); }
audit.get('/recent', recent);
archive.get('/history', history);
exports.audit = audit;
exports.archive = archive;
