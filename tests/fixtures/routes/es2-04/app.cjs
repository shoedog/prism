const express = require('express');
const app = express();
const records = express.Router();
const settings = express.Router({ strict: true });

function inventory(req, res) { res.json([]); }
function preferences(req, res) { res.json({}); }
function logRequest(req, res, next) { next(); }

records.get('/inventory', inventory);
settings.get('/preferences', preferences);
app.use([logRequest, [records, settings]]);

module.exports = app;
