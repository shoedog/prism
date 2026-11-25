const express = require('express');
const app = express();
const reports = express.Router();
const audit = express.Router({ mergeParams: true });
function check(req, res, next) { next(); }
function events(req, res) { res.send('events'); }
function summary(req, res) { res.send('summary'); }
audit.get('/events', events);
reports.get('/summary', summary);
const children = [audit, reports];
app.use('/ops', [check, children]);
module.exports = app;
